use clap::Parser;

mod cli;
mod port;

fn main() -> anyhow::Result<()> {
    let args = cli::PineAppleClapParser::parse();
    let investigator = port::PineAppleInvestigator::new(args.port);
    let report = investigator.investigate()?;
    println!("{report}");
    if args.kill_mode {
        investigator.kill(&report)?;
    }
    Ok(())
}
