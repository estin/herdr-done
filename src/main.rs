use std::ffi::OsString;
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hd")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(required = true, trailing_var_arg = true)]
        command: Vec<OsString>,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Commands::Run { command } => run(&command),
    }
}

fn run(command: &[OsString]) -> ExitCode {
    let started = Instant::now();
    let status = Command::new(&command[0]).args(&command[1..]).status();
    report_duration(started.elapsed());

    match status {
        Ok(status) => status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .map_or(ExitCode::FAILURE, ExitCode::from),
        Err(error) => {
            eprintln!(
                "hd: failed to run {}: {error}",
                command[0].to_string_lossy()
            );
            ExitCode::FAILURE
        }
    }
}

fn report_duration(duration: Duration) {
    eprintln!("Finished in {}", humantime::format_duration(duration));
}

#[cfg(test)]
mod tests {
    use super::report_duration;
    use std::time::Duration;

    #[test]
    fn duration_uses_human_readable_units() {
        let duration = humantime::format_duration(Duration::from_millis(12_400)).to_string();
        assert_eq!(duration, "12s 400ms");
    }

    #[test]
    fn reporting_a_duration_does_not_panic() {
        report_duration(Duration::ZERO);
    }
}
