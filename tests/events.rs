mod support;

use herdr_done::event::EventStatus;
use herdr_done::store::Store;
use std::fs;
use std::process::Command;
use support::{hd, text};
use tempfile::tempdir;

#[test]
fn creates_completed_event_from_message() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let database = directory.path().join("events.db");
    let output = hd(&database)
        .env("HERDR_ENV", "1")
        .env("HERDR_SESSION", "job")
        .env("HERDR_WORKSPACE_ID", "w9")
        .env("HERDR_TAB_ID", "w9:t3")
        .env("HERDR_PANE_ID", "w9:p3")
        .env("HERDR_BIN_PATH", "/nonexistent/herdr")
        .args(["event", "deployment is ready"])
        .output()?;
    assert!(output.status.success());
    assert_eq!(text(&output.stdout), "");

    let store = Store::open(&database)?;
    let events = store.list(10, false)?;
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event.program, "deployment is ready");
    assert_eq!(event.args.len(), 0);
    assert_eq!(event.status, EventStatus::Completed);
    assert_eq!(event.exit_code, Some(0));
    assert!(event.finished_at.is_some());
    assert!(event.duration_ms.is_some());
    assert_ne!(event.cwd, "");
    assert_eq!(event.herdr.session_id.as_deref(), Some("job"));
    assert_eq!(event.herdr.pane_id.as_deref(), Some("w9:p3"));
    Ok(())
}

#[test]
fn prints_created_id_when_requested() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let database = directory.path().join("events.db");
    let output = hd(&database)
        .args(["event", "--print-id", "deployment is ready"])
        .output()?;
    assert!(output.status.success());

    let store = Store::open(&database)?;
    let events = store.list(10, false)?;
    assert_eq!(events.len(), 1);
    assert_eq!(
        text(&output.stdout),
        format!("created event {}\n", events[0].id)
    );
    Ok(())
}

#[test]
fn distinguishes_manual_events_from_command_runs() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let database = directory.path().join("events.db");
    assert!(hd(&database).args(["event", "manual"]).status()?.success());
    assert!(
        hd(&database)
            .args(["run", "--", "date"])
            .status()?
            .success()
    );

    let events = Store::open(&database)?.list(10, false)?;
    assert_eq!(events.len(), 2);
    assert!(
        events
            .iter()
            .any(|event| event.program == "manual" && event.is_manual_event())
    );
    assert!(
        events
            .iter()
            .any(|event| event.program == "date" && !event.is_manual_event())
    );
    Ok(())
}

#[test]
fn configured_database_is_shared_by_commands() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let database = directory.path().join("configured.db");
    let config_home = directory.path().join("config");
    let config_dir = config_home.join("herdr-done");
    fs::create_dir_all(&config_dir)?;
    fs::write(
        config_dir.join("config.toml"),
        format!("database = \"{}\"\n", database.display()),
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_hd"))
        .env("XDG_CONFIG_HOME", &config_home)
        .env_remove("HD__DATABASE")
        .args(["event", "configured"])
        .output()?;
    assert!(output.status.success(), "{}", text(&output.stderr));

    let store = Store::open(&database)?;
    let events = store.list(10, false)?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].program, "configured");
    Ok(())
}

#[test]
fn missing_message_is_invalid_usage() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let database = directory.path().join("events.db");
    let output = hd(&database).args(["event"]).output()?;
    assert_eq!(output.status.code(), Some(2));
    assert!(!database.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn alert_flags_use_the_shared_alert_path() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir()?;
    let database = directory.path().join("events.db");
    let log = directory.path().join("alerts.log");
    let fake_herdr = directory.path().join("fake-herdr");
    fs::write(
        &fake_herdr,
        "#!/bin/sh\nif [ \"$1\" = \"notification\" ]; then printf '%s\\n' \"$@\" >> \"$ALERT_LOG\"; fi\nexit 0\n",
    )?;
    fs::set_permissions(&fake_herdr, fs::Permissions::from_mode(0o755))?;

    let output = hd(&database)
        .env("HERDR_ENV", "1")
        .env("HERDR_BIN_PATH", &fake_herdr)
        .env("ALERT_LOG", &log)
        .args(["event", "--sound", "--notify", "deployment is ready"])
        .output()?;
    assert!(output.status.success());

    let log_text = fs::read_to_string(&log)?;
    assert!(log_text.contains("notification"));
    assert!(log_text.contains("--sound"));
    assert!(log_text.contains("done"));
    assert!(log_text.contains("deployment is ready: exit 0"));
    assert_eq!(Store::open(&database)?.list(10, false)?.len(), 1);
    Ok(())
}

#[cfg(unix)]
#[test]
fn no_alerts_are_sent_by_default() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir()?;
    let log = directory.path().join("alerts.log");
    let fake_herdr = directory.path().join("fake-herdr");
    fs::write(
        &fake_herdr,
        "#!/bin/sh\nif [ \"$1\" = \"notification\" ]; then printf called >> \"$ALERT_LOG\"; fi\nexit 0\n",
    )?;
    fs::set_permissions(&fake_herdr, fs::Permissions::from_mode(0o755))?;

    let output = hd(directory.path().join("events.db").as_path())
        .env("HERDR_ENV", "1")
        .env("HERDR_BIN_PATH", &fake_herdr)
        .env("ALERT_LOG", &log)
        .args(["event", "deployment is ready"])
        .output()?;
    assert!(output.status.success());
    assert!(!log.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn storage_failure_suppresses_alerts_and_returns_failure() -> Result<(), Box<dyn std::error::Error>>
{
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir()?;
    let blocker = directory.path().join("not-a-directory");
    fs::write(&blocker, b"file")?;
    let log = directory.path().join("alerts.log");
    let fake_herdr = directory.path().join("fake-herdr");
    fs::write(
        &fake_herdr,
        "#!/bin/sh\nif [ \"$1\" = \"notification\" ]; then printf called >> \"$ALERT_LOG\"; fi\nexit 0\n",
    )?;
    fs::set_permissions(&fake_herdr, fs::Permissions::from_mode(0o755))?;

    let output = hd(blocker.join("events.db").as_path())
        .env("HERDR_ENV", "1")
        .env("HERDR_BIN_PATH", &fake_herdr)
        .env("ALERT_LOG", &log)
        .args(["event", "--sound", "--print-id", "deployment is ready"])
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(text(&output.stderr).contains("storage error"));
    assert!(!log.exists());
    Ok(())
}
