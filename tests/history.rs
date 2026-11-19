mod support;

use herdr_done::event::{Event, HerdrContext};
use herdr_done::store::Store;
use std::ffi::{OsStr, OsString};
use std::thread;
use std::time::Duration;
use support::{hd, text};
use tempfile::tempdir;

fn context() -> HerdrContext {
    HerdrContext {
        session_id: Some("job".to_owned()),
        workspace_id: Some("w3".to_owned()),
        workspace_name: None,
        tab_id: Some("w3:t1".to_owned()),
        pane_id: Some("w3:p2".to_owned()),
        terminal_id: None,
    }
}

#[test]
fn list_orders_newest_first_and_filters_unread() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let database = directory.path().join("events.db");
    let store = Store::open(&database)?;
    let older =
        Event::new(OsStr::new("older"), &[OsString::from("--flag")], context()).completed(0, None);
    thread::sleep(Duration::from_millis(5));
    let newer = Event::new(OsStr::new("newer"), &[], context()).completed(1, None);
    store.upsert(&older)?;
    store.upsert(&newer)?;
    store.set_read(&newer.id, true)?;

    let output = hd(&database).args(["list"]).output()?;
    assert!(output.status.success());
    let stdout = text(&output.stdout);
    let older_position = stdout
        .find(&older.id)
        .ok_or("older event missing from list".to_owned())?;
    let newer_position = stdout
        .find(&newer.id)
        .ok_or("newer event missing from list".to_owned())?;
    assert!(newer_position < older_position);

    let output = hd(&database).args(["list", "--unread"]).output()?;
    assert!(output.status.success());
    let stdout = text(&output.stdout);
    assert!(stdout.contains(&older.id));
    assert!(!stdout.contains(&newer.id));
    Ok(())
}

#[test]
fn show_displays_event_fields() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let database = directory.path().join("events.db");
    let store = Store::open(&database)?;
    let event = Event::new(
        OsStr::new("cargo"),
        &[OsString::from("build"), OsString::from("--release")],
        context(),
    )
    .completed(0, None);
    store.upsert(&event)?;

    let output = hd(&database).args(["show", &event.id]).output()?;
    assert!(output.status.success());
    let stdout = text(&output.stdout);
    assert!(stdout.contains("cargo build --release"));
    assert!(stdout.contains("Working directory:"));
    assert!(stdout.contains("Result: exit 0"));
    assert!(stdout.contains("Duration:"));
    assert!(stdout.contains("job"));
    assert!(stdout.contains("w3:p2"));
    Ok(())
}

#[test]
fn show_unknown_event_fails() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let output = hd(directory.path().join("events.db").as_path())
        .args(["show", "missing"])
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("no event found for missing"));
    Ok(())
}
