#[derive(clap::Parser, Debug)]
#[command(version, about = "Find the goblins listening on a TCP port (Windows)")]
pub(crate) struct PineAppleClapParser {
    /// TCP port to inspect (1-65535)
    #[arg(value_parser = clap::value_parser!(u16).range(1..))]
    pub port: u16,
    /// Forcefully terminate all processes listening on this port
    #[arg(long, alias = "kill")]
    pub kill_mode: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn validates_ports_and_accepts_kill_alias() {
        for port in ["0", "65536", "-1", "goblin"] {
            assert!(PineAppleClapParser::try_parse_from(["pg", port]).is_err());
        }
        let args = PineAppleClapParser::try_parse_from(["pg", "8080", "--kill"]).unwrap();
        assert_eq!(args.port, 8080);
        assert!(args.kill_mode);
    }
}
