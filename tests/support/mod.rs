use std::path::Path;
use std::process::Command;

pub fn hd(database: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hd"));
    command.env("HD_DATABASE", database);
    for key in [
        "HERDR_ENV",
        "HERDR_BIN_PATH",
        "HERDR_SOCKET_PATH",
        "HERDR_SESSION",
        "HERDR_WORKSPACE_ID",
        "HERDR_TAB_ID",
        "HERDR_PANE_ID",
    ] {
        command.env_remove(key);
    }
    command
}

pub fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}
