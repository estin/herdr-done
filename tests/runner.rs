mod support;

use herdr_done::event::EventStatus;
use herdr_done::store::Store;
use rusqlite::Connection;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};
use support::{hd, text};
use tempfile::tempdir;

#[test]
fn runs_arguments_directly_without_shell_expansion() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let output = hd(directory.path().join("events.db").as_path())
        .args(["run", "--", "printf", "%s", "$HOME"])
        .output()?;
    assert!(output.status.success());
    assert_eq!(text(&output.stdout), "$HOME");
    Ok(())
}

#[test]
fn inherits_stdin_and_stdout() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let mut child = hd(directory.path().join("events.db").as_path())
        .args(["run", "--", "cat"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(b"inherited input")?;
    }
    let output = child.wait_with_output()?;
    assert!(output.status.success());
    assert_eq!(text(&output.stdout), "inherited input");
    Ok(())
}

#[test]
fn inherits_stderr() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let output = hd(directory.path().join("events.db").as_path())
        .args(["run", "--", "sh", "-c", "printf inherited-error >&2"])
        .output()?;
    assert!(output.status.success());
    assert_eq!(text(&output.stderr), "inherited-error");
    Ok(())
}

#[test]
fn preserves_nonzero_exit_code() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let output = hd(directory.path().join("events.db").as_path())
        .args(["run", "--", "sh", "-c", "exit 42"])
        .output()?;
    assert_eq!(output.status.code(), Some(42));
    Ok(())
}

#[test]
fn records_launch_failure() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let database = directory.path().join("events.db");
    let output = hd(&database)
        .args(["run", "--", "/definitely/not/a/program"])
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("failed to run"));

    let store = Store::open(&database)?;
    let events = store.list(10, false)?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status, EventStatus::FailedToStart);
    Ok(())
}

#[test]
fn does_not_store_output_or_environment() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let database = directory.path().join("events.db");
    let output = hd(&database)
        .env("HD_TEST_SECRET", "secret-marker")
        .args([
            "run",
            "--",
            "sh",
            "-c",
            r#"printf '%s%s' out put-marker; printf '%s' "$HD_TEST_SECRET" >&2"#,
        ])
        .output()?;
    assert!(output.status.success());
    assert!(text(&output.stdout).contains("output-marker"));
    assert!(text(&output.stderr).contains("secret-marker"));

    let connection = Connection::open(&database)?;
    let mut statement = connection.prepare("PRAGMA table_info(events)")?;
    let columns: Vec<String> = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<_, _>>()?;
    let select = format!(
        "SELECT {} FROM events",
        columns
            .iter()
            .map(|column| format!("CAST(\"{column}\" AS TEXT)"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut statement = connection.prepare(&select)?;
    let rows = statement.query_map([], |row| {
        let mut values = String::new();
        for index in 0..columns.len() {
            if let Some(value) = row.get::<_, Option<String>>(index)? {
                values.push_str(&value);
            }
        }
        Ok(values)
    })?;
    for row in rows {
        let row = row?;
        assert!(!row.contains("output-marker"));
        assert!(!row.contains("secret-marker"));
    }
    Ok(())
}

#[test]
fn storage_failure_preserves_child_exit_code() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let blocker = directory.path().join("not-a-directory");
    fs::write(&blocker, b"file")?;
    let database = blocker.join("events.db");
    let output = hd(&database)
        .args(["run", "--", "sh", "-c", "exit 7"])
        .output()?;
    assert_eq!(output.status.code(), Some(7));
    assert!(text(&output.stderr).contains("storage error"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn terminates_child_and_records_interrupted_event() -> Result<(), Box<dyn std::error::Error>> {
    use nix::sys::signal::Signal;

    assert_signal_forwarded(Signal::SIGTERM, 143, true)
}

#[cfg(unix)]
#[test]
fn forwards_sigint_and_records_interrupted_event() -> Result<(), Box<dyn std::error::Error>> {
    use nix::sys::signal::Signal;

    assert_signal_forwarded(Signal::SIGINT, 130, false)
}

#[cfg(unix)]
fn assert_signal_forwarded(
    signal: nix::sys::signal::Signal,
    expected_code: i32,
    check_descendant_cleanup: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;

    let directory = tempdir()?;
    let database = directory.path().join("events.db");
    let ready_file = directory.path().join("child-ready");
    let marker_file = directory.path().join("child-finished");
    let script = directory.path().join("slow.sh");
    let child = if check_descendant_cleanup {
        format!("(sleep 1; touch \"{}\") &\n", marker_file.display())
    } else {
        String::new()
    };
    fs::write(
        &script,
        format!(
            "#!/bin/sh\n{child}printf ready > \"{}\"\nexec sleep 30\n",
            ready_file.display()
        ),
    )?;
    set_executable(&script)?;

    let mut child = hd(&database).arg("run").arg("--").arg(&script).spawn()?;
    wait_for_file(&ready_file)?;
    let wrapper_pid = i32::try_from(child.id()).map_err(|error| error.to_string())?;
    kill(Pid::from_raw(wrapper_pid), signal)?;

    let status = wait_for_exit(&mut child)?;
    assert_eq!(status.code(), Some(expected_code));
    if check_descendant_cleanup {
        std::thread::sleep(Duration::from_millis(1100));
        assert!(!marker_file.exists());
    }

    let store = Store::open(&database)?;
    let events = store.list(10, false)?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status, EventStatus::Interrupted);
    assert_eq!(events[0].signal, Some(expected_code - 128));
    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(unix)]
fn wait_for_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !path.exists() {
        if Instant::now() >= deadline {
            return Err(format!("timed out waiting for {}", path.display()).into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

fn wait_for_exit(
    child: &mut std::process::Child,
) -> Result<std::process::ExitStatus, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            return Err("timed out waiting for hd to exit".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
