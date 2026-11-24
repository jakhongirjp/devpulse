use std::process::ExitCode;

use clap::Parser;
use devpulse::cli::{Cli, run};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mut stdout = std::io::stdout().lock();
    match run(cli, &mut stdout) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}
