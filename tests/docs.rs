mod support;

use std::fs;
use support::{hd, text};
use tempfile::tempdir;

#[test]
fn readme_usage_matches_cli_help() -> Result<(), Box<dyn std::error::Error>> {
    let readme = fs::read_to_string("README.md")?;
    let directory = tempdir()?;
    for command in [
        "run", "event", "list", "show", "inbox", "next", "clear", "jump",
    ] {
        let output = hd(directory.path().join("events.db").as_path())
            .args([command, "--help"])
            .output()?;
        assert!(output.status.success());
        let help = text(&output.stdout);
        let usage = help
            .lines()
            .find(|line| line.starts_with("Usage: "))
            .ok_or_else(|| format!("no usage line for {command}"))?;
        assert!(readme.contains(usage), "README does not contain {usage}");
    }
    for command in ["run", "event"] {
        let output = hd(directory.path().join("events.db").as_path())
            .args([command, "--help"])
            .output()?;
        assert!(output.status.success());
        let help = text(&output.stdout);
        for flag in ["--sound", "--notify"] {
            assert!(
                help.contains(flag),
                "{command} help does not contain {flag}"
            );
        }
    }
    for flag in ["--sound", "--notify"] {
        assert!(readme.contains(flag), "README does not contain {flag}");
    }
    for phrase in [
        "no unread events",
        "cleared N read events",
        "Clear read events",
        "Press Ctrl-C twice within two seconds to exit",
        "To quit, press Ctrl-C twice",
        "hd event MESSAGE",
        "Alerts are sent only after the event is saved",
    ] {
        assert!(readme.contains(phrase), "README does not contain {phrase}");
    }
    Ok(())
}
