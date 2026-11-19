use clap::Parser;
use herdr_done::cli::{Cli, Commands};
use herdr_done::herdr;
use herdr_done::inbox;
use herdr_done::runner;
use herdr_done::store::Store;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    dispatch(Cli::parse().command())
}

fn dispatch(command: Commands) -> ExitCode {
    match command {
        Commands::Run { command } => {
            let mut parts = command.into_iter();
            let Some(program) = parts.next() else {
                eprintln!("hd: missing command");
                return ExitCode::FAILURE;
            };
            let args: Vec<OsString> = parts.collect();
            runner::run(&program, &args)
        }
        Commands::List { unread, limit } => list(unread, limit),
        Commands::Show { id } => show(&id),
        Commands::Inbox => open_inbox(),
        Commands::Jump { id } => jump(&id),
    }
}

fn list(unread: bool, limit: usize) -> ExitCode {
    let store = match Store::open_default() {
        Ok(store) => store,
        Err(error) => return fail(&error),
    };
    let events = match store.list(limit, unread) {
        Ok(events) => events,
        Err(error) => return fail(&error),
    };
    if events.is_empty() {
        println!("no events");
        return ExitCode::SUCCESS;
    }
    println!(
        "{:<26} {:<18} {:<10} {:<20} COMMAND",
        "ID", "RESULT", "DURATION", "FINISHED"
    );
    for event in events {
        println!(
            "{:<26} {:<18} {:<10} {:<20} {}",
            event.id,
            truncate(&event.result_label(), 18),
            event.duration_label(),
            event.finished_label(),
            event.command_line()
        );
    }
    ExitCode::SUCCESS
}

fn show(id: &str) -> ExitCode {
    let store = match Store::open_default() {
        Ok(store) => store,
        Err(error) => return fail(&error),
    };
    let event = match store.get(id) {
        Ok(Some(event)) => event,
        Ok(None) => return fail(&format!("no event found for {id}")),
        Err(error) => return fail(&error),
    };
    println!("ID: {}", event.id);
    println!("Command: {}", event.command_line());
    println!("Working directory: {}", event.cwd);
    println!("Result: {}", event.result_label());
    println!("Started: {}", event.started_label());
    println!("Finished: {}", event.finished_label());
    println!("Duration: {}", event.duration_label());
    println!("Host: {}", event.hostname);
    println!(
        "Read: {}",
        if event.read_at.is_none() { "no" } else { "yes" }
    );
    println!(
        "Herdr session: {}",
        event.herdr.session_id.as_deref().unwrap_or("-")
    );
    println!(
        "Workspace: {}",
        event.herdr.workspace_id.as_deref().unwrap_or("-")
    );
    println!("Tab: {}", event.herdr.tab_id.as_deref().unwrap_or("-"));
    println!("Pane: {}", event.herdr.pane_id.as_deref().unwrap_or("-"));
    ExitCode::SUCCESS
}

fn open_inbox() -> ExitCode {
    let store = match Store::open_default() {
        Ok(store) => store,
        Err(error) => return fail(&error),
    };
    match inbox::run(&store) {
        Ok(Some(id)) => jump(&id),
        Ok(None) => ExitCode::SUCCESS,
        Err(error) => fail(&error),
    }
}

fn jump(id: &str) -> ExitCode {
    if env::var_os("HERDR_ENV").as_deref() != Some(OsString::from("1").as_os_str()) {
        return fail("Herdr context is unavailable");
    }
    let store = match Store::open_default() {
        Ok(store) => store,
        Err(error) => return fail(&error),
    };
    let event = match store.get(id) {
        Ok(Some(event)) => event,
        Ok(None) => return fail(&format!("no event found for {id}")),
        Err(error) => return fail(&error),
    };
    let Some(pane_id) = event.herdr.pane_id else {
        return fail("event has no Herdr pane");
    };
    if let (Some(expected), Some(actual)) =
        (&event.herdr.session_id, env::var("HERDR_SESSION").ok())
        && expected != &actual
    {
        return fail(&format!(
            "event belongs to Herdr session {expected}, not {actual}"
        ));
    }
    let Some(socket_path) = env::var_os("HERDR_SOCKET_PATH") else {
        return fail("HERDR_SOCKET_PATH is unavailable");
    };
    let socket_path = PathBuf::from(socket_path);
    match herdr::focus_pane(&socket_path, &pane_id) {
        Ok(()) => {
            println!("focused pane {pane_id}");
            ExitCode::SUCCESS
        }
        Err(error) => fail(&error),
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("hd: {message}");
    ExitCode::FAILURE
}

fn truncate(value: &str, maximum: usize) -> String {
    if value.chars().count() <= maximum {
        return value.to_owned();
    }
    value
        .chars()
        .take(maximum.saturating_sub(1))
        .collect::<String>()
        + "…"
}
