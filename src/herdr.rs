use crate::event::HerdrContext;
use crate::store::Store;
use serde_json::Value;
use std::env;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::net::UnixStream;

pub fn current_context() -> HerdrContext {
    detect_context(&|key| env::var_os(key))
}

pub fn detect_context(lookup: &dyn Fn(&str) -> Option<OsString>) -> HerdrContext {
    if string_value(lookup, "HERDR_ENV").as_deref() != Some("1") {
        return HerdrContext::default();
    }

    let mut context = HerdrContext {
        session_id: string_value(lookup, "HERDR_SESSION"),
        workspace_id: string_value(lookup, "HERDR_WORKSPACE_ID"),
        workspace_name: None,
        tab_id: string_value(lookup, "HERDR_TAB_ID"),
        pane_id: string_value(lookup, "HERDR_PANE_ID"),
        terminal_id: None,
    };

    let binary = string_value(lookup, "HERDR_BIN_PATH").unwrap_or_else(|| "herdr".to_owned());
    if let Ok(output) = std::process::Command::new(binary)
        .args(["pane", "current", "--current"])
        .output()
        && output.status.success()
        && let Ok(from_cli) = parse_pane_current(&output.stdout)
    {
        merge(&mut context, from_cli);
    }
    context
}

fn merge(context: &mut HerdrContext, from_cli: HerdrContext) {
    if from_cli.workspace_id.is_some() {
        context.workspace_id = from_cli.workspace_id;
    }
    if from_cli.workspace_name.is_some() {
        context.workspace_name = from_cli.workspace_name;
    }
    if from_cli.tab_id.is_some() {
        context.tab_id = from_cli.tab_id;
    }
    if from_cli.pane_id.is_some() {
        context.pane_id = from_cli.pane_id;
    }
    if from_cli.terminal_id.is_some() {
        context.terminal_id = from_cli.terminal_id;
    }
}

pub fn parse_pane_current(bytes: &[u8]) -> Result<HerdrContext, String> {
    let response: Value = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let pane = response
        .pointer("/result/pane")
        .ok_or_else(|| "pane.current response has no pane".to_owned())?;
    let field = |name: &str| {
        pane.get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .filter(|value| !value.is_empty())
    };
    Ok(HerdrContext {
        session_id: None,
        workspace_id: field("workspace_id"),
        workspace_name: None,
        tab_id: field("tab_id"),
        pane_id: field("pane_id"),
        terminal_id: field("terminal_id"),
    })
}

pub fn focus_event(store: &Store, id: &str) -> Result<String, String> {
    if env::var_os("HERDR_ENV").as_deref() != Some(OsString::from("1").as_os_str()) {
        return Err("Herdr context is unavailable".to_owned());
    }
    let event = store
        .get(id)?
        .ok_or_else(|| format!("no event found for {id}"))?;
    let pane_id = event
        .herdr
        .pane_id
        .ok_or_else(|| "event has no Herdr pane".to_owned())?;
    if let (Some(expected), Some(actual)) =
        (&event.herdr.session_id, env::var("HERDR_SESSION").ok())
        && expected != &actual
    {
        return Err(format!(
            "event belongs to Herdr session {expected}, not {actual}"
        ));
    }
    let socket_path = env::var_os("HERDR_SOCKET_PATH")
        .map(PathBuf::from)
        .ok_or_else(|| "HERDR_SOCKET_PATH is unavailable".to_owned())?;
    focus_pane(&socket_path, &pane_id)?;
    Ok(pane_id)
}

#[cfg(unix)]
pub fn focus_pane(socket_path: &Path, pane_id: &str) -> Result<(), String> {
    let mut stream =
        UnixStream::connect(socket_path).map_err(|error| format!("cannot reach Herdr: {error}"))?;
    let request = serde_json::json!({
        "id": "hd_jump",
        "method": "pane.focus",
        "params": { "pane_id": pane_id },
    });
    stream
        .write_all(format!("{request}\n").as_bytes())
        .map_err(|error| format!("cannot send focus request: {error}"))?;
    stream
        .flush()
        .map_err(|error| format!("cannot send focus request: {error}"))?;

    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|error| format!("cannot read Herdr response: {error}"))?;
    let response: Value =
        serde_json::from_str(&line).map_err(|error| format!("invalid Herdr response: {error}"))?;
    if let Some(error) = response.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Herdr rejected the focus request");
        return Err(message.to_owned());
    }
    if response.get("result").is_none() {
        return Err("Herdr returned no focus result".to_owned());
    }
    Ok(())
}

#[cfg(not(unix))]
pub fn focus_pane(_socket_path: &Path, _pane_id: &str) -> Result<(), String> {
    Err("pane focus is available only on Unix".to_owned())
}

fn string_value(lookup: &dyn Fn(&str) -> Option<OsString>, key: &str) -> Option<String> {
    lookup(key)
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{detect_context, focus_pane, parse_pane_current};
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::io::{BufRead, BufReader, Write};
    use std::path::Path;

    const CAPTURED: &str = r#"{"id":"cli:pane:current","result":{"pane":{"agent":"codex","agent_status":"working","cwd":"/home/user/prj/herdr-done","focused":true,"foreground_cwd":"/home/user/prj/herdr-done","pane_id":"w11:p7","revision":369,"tab_id":"w11:t3","terminal_id":"term_6609258221ebd50","terminal_title":"Apply OpenSpec change","terminal_title_stripped":"Apply OpenSpec change","workspace_id":"w11","type":"pane_current"}}}"#;

    #[test]
    fn parses_captured_pane_current_output() -> Result<(), String> {
        let context = parse_pane_current(CAPTURED.as_bytes())?;
        assert_eq!(context.workspace_id.as_deref(), Some("w11"));
        assert_eq!(context.tab_id.as_deref(), Some("w11:t3"));
        assert_eq!(context.pane_id.as_deref(), Some("w11:p7"));
        assert_eq!(context.terminal_id.as_deref(), Some("term_6609258221ebd50"));
        Ok(())
    }

    #[test]
    fn outside_herdr_has_no_context() {
        let values: HashMap<String, OsString> = HashMap::new();
        let context = detect_context(&|key| values.get(key).cloned());
        assert!(context.is_empty());
    }

    #[test]
    fn keeps_environment_context_when_cli_is_unavailable() {
        let values = HashMap::from([
            ("HERDR_ENV".to_owned(), OsString::from("1")),
            ("HERDR_SESSION".to_owned(), OsString::from("job")),
            ("HERDR_WORKSPACE_ID".to_owned(), OsString::from("w4")),
            ("HERDR_TAB_ID".to_owned(), OsString::from("w4:t2")),
            ("HERDR_PANE_ID".to_owned(), OsString::from("w4:p3")),
            (
                "HERDR_BIN_PATH".to_owned(),
                OsString::from("/nonexistent/herdr"),
            ),
        ]);
        let context = detect_context(&|key| values.get(key).cloned());
        assert_eq!(context.session_id.as_deref(), Some("job"));
        assert_eq!(context.workspace_id.as_deref(), Some("w4"));
        assert_eq!(context.tab_id.as_deref(), Some("w4:t2"));
        assert_eq!(context.pane_id.as_deref(), Some("w4:p3"));
    }

    #[cfg(unix)]
    #[test]
    fn focus_request_uses_documented_socket_api() -> Result<(), String> {
        use std::os::unix::net::UnixListener;
        use std::thread;

        let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
        let socket_path = directory.path().join("herdr.sock");
        let listener = UnixListener::bind(&socket_path).map_err(|error| error.to_string())?;
        let server = thread::spawn(move || -> std::io::Result<String> {
            let (mut stream, _) = listener.accept()?;
            let mut request = String::new();
            BufReader::new(&stream).read_line(&mut request)?;
            stream.write_all(b"{\"id\":\"hd_jump\",\"result\":{\"type\":\"pane_focus\"}}\n")?;
            Ok(request)
        });

        focus_pane(&socket_path, "w9:p3")?;
        let request = server
            .join()
            .map_err(|_| "server thread failed".to_owned())?
            .map_err(|error| error.to_string())?;
        let value: serde_json::Value =
            serde_json::from_str(&request).map_err(|error| error.to_string())?;
        assert_eq!(value["method"], "pane.focus");
        assert_eq!(value["params"]["pane_id"], "w9:p3");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn focus_reports_stale_pane_error() -> Result<(), String> {
        use std::os::unix::net::UnixListener;
        use std::thread;

        let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
        let socket_path = directory.path().join("herdr.sock");
        let listener = UnixListener::bind(&socket_path).map_err(|error| error.to_string())?;
        let server = thread::spawn(move || -> std::io::Result<()> {
            let (mut stream, _) = listener.accept()?;
            let mut request = String::new();
            BufReader::new(&stream).read_line(&mut request)?;
            stream.write_all(
                b"{\"id\":\"hd_jump\",\"error\":{\"code\":\"not_found\",\"message\":\"pane not found\"}}\n",
            )?;
            Ok(())
        });

        let error = focus_pane(Path::new(&socket_path), "w9:p9")
            .err()
            .ok_or_else(|| "expected an error".to_owned())?;
        assert_eq!(error, "pane not found");
        server
            .join()
            .map_err(|_| "server thread failed".to_owned())?
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
