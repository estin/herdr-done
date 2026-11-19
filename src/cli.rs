use clap::{Parser, Subcommand, builder::OsStringValueParser};
use std::ffi::OsString;

#[derive(Parser, Debug)]
#[command(
    name = "hd",
    about = "Run commands and review their completion events",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a command directly and record its completion
    Run {
        /// Play Herdr's configured completion sound after the command finishes
        #[arg(long)]
        sound: bool,

        /// Send a system notification after the command finishes
        #[arg(long)]
        notify: bool,

        /// Executable and arguments, after --
        #[arg(
            value_name = "COMMAND",
            required = true,
            num_args = 1..,
            trailing_var_arg = true,
            value_parser = OsStringValueParser::new()
        )]
        command: Vec<OsString>,
    },

    /// List recent completion events
    List {
        /// Show only unread events
        #[arg(long)]
        unread: bool,

        /// Maximum number of events to show
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },

    /// Show one completion event
    Show {
        /// Event identifier
        id: String,
    },

    /// Open the interactive inbox
    Inbox {
        /// Close after a pane jump instead of staying open
        #[arg(long)]
        oneshot: bool,
    },

    /// Focus the Herdr pane where a command started
    Jump {
        /// Event identifier
        id: String,
    },
}

impl Cli {
    pub fn command(self) -> Commands {
        self.command
    }
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands};
    use clap::Parser;
    use std::ffi::OsString;

    #[test]
    fn run_keeps_arguments_after_double_dash() -> Result<(), String> {
        let cli = Cli::try_parse_from([
            "hd",
            "run",
            "--",
            "cargo",
            "test",
            "--workspace",
            "--all-features",
        ])
        .map_err(|error| error.to_string())?;
        match cli.command() {
            Commands::Run { command, .. } => {
                let expected: Vec<OsString> = ["cargo", "test", "--workspace", "--all-features"]
                    .into_iter()
                    .map(OsString::from)
                    .collect();
                assert_eq!(command, expected);
            }
            _ => return Err("expected the run command".to_owned()),
        }
        Ok(())
    }

    #[test]
    fn run_has_no_alerts_by_default() -> Result<(), String> {
        let cli =
            Cli::try_parse_from(["hd", "run", "--", "true"]).map_err(|error| error.to_string())?;
        match cli.command() {
            Commands::Run {
                sound,
                notify,
                command: _,
            } => {
                assert!(!sound);
                assert!(!notify);
            }
            _ => return Err("expected the run command".to_owned()),
        }
        Ok(())
    }

    #[test]
    fn run_parses_sound_and_notify_independently() -> Result<(), String> {
        let cases: [&[&str]; 3] = [
            &["hd", "run", "--sound", "--", "true"],
            &["hd", "run", "--notify", "--", "true"],
            &["hd", "run", "--sound", "--notify", "--", "true"],
        ];
        for args in cases {
            let cli = Cli::try_parse_from(args).map_err(|error| error.to_string())?;
            match cli.command() {
                Commands::Run {
                    sound,
                    notify,
                    command: _,
                } => {
                    assert_eq!(sound, args.contains(&"--sound"));
                    assert_eq!(notify, args.contains(&"--notify"));
                }
                _ => return Err("expected the run command".to_owned()),
            }
        }
        Ok(())
    }

    #[test]
    fn inbox_defaults_to_live_mode() -> Result<(), String> {
        let cli = Cli::try_parse_from(["hd", "inbox"]).map_err(|error| error.to_string())?;
        match cli.command() {
            Commands::Inbox { oneshot } => assert!(!oneshot),
            _ => return Err("expected the inbox command".to_owned()),
        }
        Ok(())
    }

    #[test]
    fn inbox_accepts_oneshot_mode() -> Result<(), String> {
        let cli =
            Cli::try_parse_from(["hd", "inbox", "--oneshot"]).map_err(|error| error.to_string())?;
        match cli.command() {
            Commands::Inbox { oneshot } => assert!(oneshot),
            _ => return Err("expected the inbox command".to_owned()),
        }
        Ok(())
    }

    #[test]
    fn list_accepts_unread_filter() -> Result<(), String> {
        let cli =
            Cli::try_parse_from(["hd", "list", "--unread"]).map_err(|error| error.to_string())?;
        match cli.command() {
            Commands::List { unread, limit } => {
                assert!(unread);
                assert_eq!(limit, 20);
            }
            _ => return Err("expected the list command".to_owned()),
        }
        Ok(())
    }
}
