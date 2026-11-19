use clap::Parser;
use herdr_done::cli::{Cli, Commands};
use herdr_done::herdr;
use herdr_done::inbox;
use herdr_done::runner;
use herdr_done::store::Store;
use std::ffi::OsString;
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
        Commands::Inbox { oneshot } => open_inbox(oneshot),
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
    println!("Herdr location: {}", event.location_label());
    ExitCode::SUCCESS
}

fn open_inbox(oneshot: bool) -> ExitCode {
    let store = match Store::open_default() {
        Ok(store) => store,
        Err(error) => return fail(&error),
    };
    let mode = if oneshot {
        inbox::Mode::Oneshot
    } else {
        inbox::Mode::Live
    };
    match inbox::run(&store, mode) {
        Ok(Some(id)) => jump(&id),
        Ok(None) => ExitCode::SUCCESS,
        Err(error) => fail(&error),
    }
}

fn jump(id: &str) -> ExitCode {
    let store = match Store::open_default() {
        Ok(store) => store,
        Err(error) => return fail(&error),
    };
    match herdr::focus_event(&store, id) {
        Ok(pane_id) => {
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
