use std::{
    collections::BTreeSet,
    net::TcpListener,
    process::Command,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, bail};
use sysinfo::{Pid, ProcessesToUpdate, System};

use super::{Gremlin, PortReport, PortStatus};

pub(crate) struct PineAppleInvestigator {
    port: u16,
}

impl PineAppleInvestigator {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub fn investigate(&self) -> anyhow::Result<PortReport> {
        let pids = listener_pids(self.port)?;
        let status = if pids.is_empty() {
            match probe_port(self.port) {
                Ok(()) => PortStatus::Free,
                Err(error) => PortStatus::Unavailable(error.to_string()),
            }
        } else {
            let mut system = System::new();
            let ids: Vec<_> = pids.iter().copied().map(Pid::from_u32).collect();
            system.refresh_processes(ProcessesToUpdate::Some(&ids), true);
            PortStatus::Occupied(
                pids.into_iter()
                    .map(|pid| {
                        let process = system.process(Pid::from_u32(pid));
                        Gremlin {
                            pid,
                            name: process.map(|p| p.name().to_string_lossy().into_owned()),
                            start_time: process.map(|p| p.start_time()).filter(|time| *time != 0),
                        }
                    })
                    .collect(),
            )
        };
        Ok(PortReport {
            port: self.port,
            status,
        })
    }

    pub fn kill(&self, report: &PortReport) -> anyhow::Result<()> {
        let gremlins = match &report.status {
            PortStatus::Free => return Ok(()),
            PortStatus::Unavailable(reason) => bail!("Cannot free port {}: {reason}", self.port),
            PortStatus::Occupied(gremlins) => gremlins,
        };
        // Validate every owner before terminating any of them.
        let mut system = System::new();
        let ids: Vec<_> = gremlins.iter().map(|g| Pid::from_u32(g.pid)).collect();
        system.refresh_processes(ProcessesToUpdate::Some(&ids), true);
        let current = listener_pids(self.port)?;
        for gremlin in gremlins {
            if !current.contains(&gremlin.pid) {
                continue;
            }
            if gremlin.pid <= 4 || gremlin.pid == std::process::id() {
                bail!("Refusing to terminate protected PID {}", gremlin.pid);
            }
            let process = system
                .process(Pid::from_u32(gremlin.pid))
                .context("Cannot verify the owner; inspect the port again")?;
            if gremlin.start_time != Some(process.start_time()) {
                bail!(
                    "PID {} changed or could not be identified; inspect the port again",
                    gremlin.pid
                );
            }
        }
        for gremlin in gremlins {
            if !current.contains(&gremlin.pid) {
                continue;
            }
            let process = system
                .process(Pid::from_u32(gremlin.pid))
                .context("Process disappeared")?;
            if !process.kill() {
                bail!(
                    "Could not terminate PID {}. Access may be denied; try an elevated terminal if appropriate",
                    gremlin.pid
                );
            }
            println!("{}", super::termination_message(gremlin.pid));
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let report = self.investigate()?;
            if matches!(report.status, PortStatus::Free) {
                println!("{report}");
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!("Port was not released (a service may have restarted): {report}");
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

fn probe_port(port: u16) -> std::io::Result<()> {
    let _ipv4 = TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED, port))?;
    match TcpListener::bind((std::net::Ipv6Addr::UNSPECIFIED, port)) {
        Ok(_ipv6) => Ok(()),
        // Windows WSAEAFNOSUPPORT: IPv6 may be disabled on the machine.
        Err(error) if error.raw_os_error() == Some(10047) => Ok(()),
        Err(error) => Err(error),
    }
}

fn listener_pids(port: u16) -> anyhow::Result<BTreeSet<u32>> {
    if !cfg!(windows) {
        bail!("Port Goblin currently supports Windows only");
    }
    let output = Command::new("netstat")
        .arg("-ano")
        .output()
        .context("Failed to run Windows netstat")?;
    if !output.status.success() {
        bail!(
            "netstat failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    parse_listeners(&String::from_utf8_lossy(&output.stdout), port)
}

fn parse_listeners(output: &str, port: u16) -> anyhow::Result<BTreeSet<u32>> {
    let mut pids = BTreeSet::new();
    for line in output.lines() {
        let columns: Vec<_> = line.split_whitespace().collect();
        if columns.first() != Some(&"TCP") || columns.len() != 5 || columns[3] != "LISTENING" {
            continue;
        }
        let local_port = columns[1]
            .rsplit_once(':')
            .and_then(|(_, p)| p.parse::<u16>().ok());
        if local_port == Some(port) {
            pids.insert(
                columns[4]
                    .parse()
                    .context("Invalid listener PID in netstat output")?,
            );
        }
    }
    Ok(pids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn refuses_to_terminate_itself() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let investigator = PineAppleInvestigator::new(listener.local_addr().unwrap().port());
        let report = investigator.investigate().unwrap();
        let error = investigator.kill(&report).unwrap_err();
        assert!(error.to_string().contains("protected PID"), "{error}");
        assert!(listener.local_addr().is_ok());
    }

    #[test]
    fn filters_protocol_state_and_exact_local_port_and_deduplicates() {
        let output = "\n  TCP  0.0.0.0:80  0.0.0.0:0  LISTENING  100\n\
                      TCP  [::]:80  [::]:0  LISTENING  100\n\
                      TCP  127.0.0.2:80  0.0.0.0:0  LISTENING  200\n\
                      TCP  0.0.0.0:8080  0.0.0.0:0  LISTENING  300\n\
                      TCP  127.0.0.1:80  127.0.0.1:9000  ESTABLISHED  400\n\
                      TCP  127.0.0.1:90  127.0.0.1:80  ESTABLISHED  500\n\
                      TCP  127.0.0.1:80  127.0.0.1:9000  TIME_WAIT  0\n\
                      UDP  0.0.0.0:80  *:*  600\n";
        assert_eq!(
            parse_listeners(output, 80).unwrap(),
            BTreeSet::from([100, 200])
        );
        assert!(parse_listeners(output, 443).unwrap().is_empty());
    }

    #[test]
    fn malformed_matching_pid_is_an_error() {
        assert!(parse_listeners("TCP 0.0.0.0:80 0.0.0.0:0 LISTENING nope", 80).is_err());
    }
}
