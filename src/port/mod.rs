mod pineapple_investigator;
pub(crate) use pineapple_investigator::PineAppleInvestigator;

use owo_colors::{OwoColorize, Style};
use std::io::IsTerminal;

fn paint(value: impl std::fmt::Display, style: Style) -> String {
    let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
    if std::io::stdout().is_terminal() && !no_color {
        value.style(style).to_string()
    } else {
        value.to_string()
    }
}

pub(super) fn termination_message(pid: u32) -> String {
    format!(
        "  {} Termination requested for {}",
        paint("[EVICT]", Style::new().magenta().bold()),
        paint(format!("PID {pid}"), Style::new().cyan()),
    )
}

#[derive(Debug)]
pub(crate) struct Gremlin {
    pub pid: u32,
    pub name: Option<String>,
    pub start_time: Option<u64>,
}

#[derive(Debug)]
pub(crate) enum PortStatus {
    Free,
    Occupied(Vec<Gremlin>),
    Unavailable(String),
}

pub(crate) struct PortReport {
    pub port: u16,
    pub status: PortStatus,
}

impl std::fmt::Display for PortReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.status {
            PortStatus::Free => write!(
                f,
                "  {} {}\n  {} No goblins here. Ready to bind!",
                paint("[FREE]", Style::new().green().bold()),
                paint(format!("TCP :{}", self.port), Style::new().cyan().bold()),
                paint("`--", Style::new().dimmed()),
            ),
            PortStatus::Occupied(gremlins) => {
                writeln!(
                    f,
                    "  {} {}",
                    paint("[OCCUPIED]", Style::new().red().bold()),
                    paint(format!("TCP :{}", self.port), Style::new().cyan().bold()),
                )?;
                for (index, gremlin) in gremlins.iter().enumerate() {
                    if index > 0 {
                        writeln!(f)?;
                    }
                    write!(
                        f,
                        "  {} {}  {}",
                        paint(
                            if index + 1 == gremlins.len() {
                                "`--"
                            } else {
                                "|--"
                            },
                            Style::new().dimmed()
                        ),
                        paint(
                            gremlin.name.as_deref().unwrap_or("Unknown goblin"),
                            Style::new().yellow().bold()
                        ),
                        paint(format!("(PID {})", gremlin.pid), Style::new().dimmed()),
                    )?;
                }
                Ok(())
            }
            PortStatus::Unavailable(reason) => {
                write!(
                    f,
                    "  {} {}\n  {} {reason}",
                    paint("[UNAVAILABLE]", Style::new().yellow().bold()),
                    paint(format!("TCP :{}", self.port), Style::new().cyan().bold()),
                    paint("`--", Style::new().dimmed()),
                )
            }
        }
    }
}
