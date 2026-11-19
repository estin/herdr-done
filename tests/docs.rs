mod support;

use std::fs;
use support::{hd, text};
use tempfile::tempdir;

#[test]
fn readme_usage_matches_cli_help() -> Result<(), Box<dyn std::error::Error>> {
    let readme = fs::read_to_string("README.md")?;
    let directory = tempdir()?;
    for command in ["run", "list", "show", "inbox", "jump"] {
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
    Ok(())
}
