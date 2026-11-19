mod support;

use herdr_done::event::{Event, HerdrContext};
use herdr_done::store::Store;
use std::ffi::OsStr;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::thread;
use support::{hd, text};
use tempfile::tempdir;

#[cfg(unix)]
fn write_fake_herdr(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    fs::write(
        path,
        r#"#!/bin/sh
printf '{"id":"cli:pane:current","result":{"pane":{"pane_id":"%s","tab_id":"w9:t1","workspace_id":"w9","terminal_id":"term_fake","focused":true,"revision":1,"type":"pane_current"}}}\n' "$FAKE_PANE_ID"
"#,
    )?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn commands_in_separate_panes_keep_their_context() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let database = directory.path().join("events.db");
    let fake_herdr = directory.path().join("fake-herdr");
    write_fake_herdr(&fake_herdr)?;

    let mut first = hd(&database)
        .env("HERDR_ENV", "1")
        .env("HERDR_SESSION", "job")
        .env("HERDR_BIN_PATH", &fake_herdr)
        .env("FAKE_PANE_ID", "w9:p1")
        .args(["run", "--", "sh", "-c", "sleep 0.1; exit 0"])
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    let mut second = hd(&database)
        .env("HERDR_ENV", "1")
        .env("HERDR_SESSION", "job")
        .env("HERDR_BIN_PATH", &fake_herdr)
        .env("FAKE_PANE_ID", "w9:p2")
        .args(["run", "--", "sh", "-c", "sleep 0.2; exit 0"])
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    assert!(first.wait()?.success());
    assert!(second.wait()?.success());

    let store = Store::open(&database)?;
    let events = store.list(10, false)?;
    assert_eq!(events.len(), 2);
    let panes: Vec<&str> = events
        .iter()
        .filter_map(|event| event.herdr.pane_id.as_deref())
        .collect();
    assert!(panes.contains(&"w9:p1"));
    assert!(panes.contains(&"w9:p2"));
    for event in &events {
        assert_eq!(event.herdr.session_id.as_deref(), Some("job"));
        assert_eq!(event.herdr.workspace_id.as_deref(), Some("w9"));
        assert_eq!(event.herdr.tab_id.as_deref(), Some("w9:t1"));
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn jump_sends_the_documented_focus_request() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::net::UnixListener;

    let directory = tempdir()?;
    let database = directory.path().join("events.db");
    let socket_path = directory.path().join("herdr.sock");
    let store = Store::open(&database)?;
    let mut event = Event::new(OsStr::new("cargo"), &[], context()).completed(0, None);
    event.herdr.session_id = Some("job".to_owned());
    event.herdr.pane_id = Some("w8:p2".to_owned());
    store.upsert(&event)?;

    let listener = UnixListener::bind(&socket_path)?;
    let server = thread::spawn(move || -> std::io::Result<String> {
        let (mut stream, _) = listener.accept()?;
        let mut request = String::new();
        BufReader::new(&stream).read_line(&mut request)?;
        stream.write_all(b"{\"id\":\"hd_jump\",\"result\":{\"type\":\"pane_focus\"}}\n")?;
        Ok(request)
    });

    let output = hd(&database)
        .env("HERDR_ENV", "1")
        .env("HERDR_SESSION", "job")
        .env("HERDR_SOCKET_PATH", &socket_path)
        .args(["jump", &event.id])
        .output()?;
    assert!(output.status.success());
    assert!(text(&output.stdout).contains("focused pane w8:p2"));
    let request = server
        .join()
        .map_err(|_| "server thread failed".to_owned())??;
    let value: serde_json::Value = serde_json::from_str(&request)?;
    assert_eq!(value["method"], "pane.focus");
    assert_eq!(value["params"]["pane_id"], "w8:p2");
    Ok(())
}

#[cfg(unix)]
#[test]
fn jump_reports_a_stale_pane() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::net::UnixListener;

    let directory = tempdir()?;
    let database = directory.path().join("events.db");
    let socket_path = directory.path().join("herdr.sock");
    let store = Store::open(&database)?;
    let mut event = Event::new(OsStr::new("cargo"), &[], context()).completed(0, None);
    event.herdr.session_id = Some("job".to_owned());
    event.herdr.pane_id = Some("w8:p9".to_owned());
    store.upsert(&event)?;

    let listener = UnixListener::bind(&socket_path)?;
    let server = thread::spawn(move || -> std::io::Result<()> {
        let (mut stream, _) = listener.accept()?;
        let mut request = String::new();
        BufReader::new(&stream).read_line(&mut request)?;
        stream.write_all(
            b"{\"id\":\"hd_jump\",\"error\":{\"code\":\"not_found\",\"message\":\"pane not found\"}}\n",
        )?;
        Ok(())
    });

    let output = hd(&database)
        .env("HERDR_ENV", "1")
        .env("HERDR_SESSION", "job")
        .env("HERDR_SOCKET_PATH", &socket_path)
        .args(["jump", &event.id])
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("pane not found"));
    server
        .join()
        .map_err(|_| "server thread failed".to_owned())??;
    Ok(())
}

#[cfg(unix)]
#[test]
fn jump_rejects_another_herdr_session() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let database = directory.path().join("events.db");
    let store = Store::open(&database)?;
    let mut event = Event::new(OsStr::new("cargo"), &[], context()).completed(0, None);
    event.herdr.session_id = Some("other".to_owned());
    event.herdr.pane_id = Some("w8:p2".to_owned());
    store.upsert(&event)?;

    let output = hd(&database)
        .env("HERDR_ENV", "1")
        .env("HERDR_SESSION", "job")
        .args(["jump", &event.id])
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("belongs to Herdr session other"));
    Ok(())
}

fn context() -> HerdrContext {
    HerdrContext::default()
}
