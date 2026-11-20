use clap::Parser;
use herdr_done::alerts;
use herdr_done::cli::{Cli, Commands};
use herdr_done::config;
use herdr_done::event::Event;
use herdr_done::herdr;
use herdr_done::inbox;
use herdr_done::runner;
use herdr_done::store::Store;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let config = match config::load() {
        Ok(config) => config,
        Err(error) => return fail(&error),
    };
    dispatch(Cli::parse().command(), &config)
}

fn dispatch(command: Commands, config: &config::Config) -> ExitCode {
    match command {
        Commands::Run {
            sound,
            notify,
            command,
        } => {
            let mut parts = command.into_iter();
            let Some(program) = parts.next() else {
                eprintln!("hd: missing command");
                return ExitCode::FAILURE;
            };
            let args: Vec<OsString> = parts.collect();
            runner::run(&program, &args, sound, notify, &config.database)
        }
        Commands::Event {
            sound,
            notify,
            print_id,
            message,
        } => create_event(&message, sound, notify, print_id, &config.database),
        Commands::List { unread, limit } => list(unread, limit, &config.database),
        Commands::Show { id } => show(&id, &config.database),
        Commands::Inbox { oneshot } => open_inbox(oneshot, false, config),
        Commands::Next => next(config),
        Commands::Clear => clear(&config.database),
        Commands::Jump { id } => jump(&id, &config.database),
    }
}

fn create_event(
    message: &str,
    sound: bool,
    notify: bool,
    print_id: bool,
    database: &Path,
) -> ExitCode {
    let store = match Store::open(database) {
        Ok(store) => store,
        Err(error) => return fail(&error),
    };
    let event = Event::new(OsStr::new(message), &[], herdr::current_context())
        .completed(0, None)
        .manual();
    if let Err(error) = store.upsert(&event) {
        return fail(&error);
    }
    alerts::deliver(sound, notify, &event);
    if print_id {
        println!("created event {}", event.id);
    }
    ExitCode::SUCCESS
}

fn list(unread: bool, limit: usize, database: &Path) -> ExitCode {
    let store = match Store::open(database) {
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

fn show(id: &str, database: &Path) -> ExitCode {
    let store = match Store::open(database) {
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

fn open_inbox(oneshot: bool, start_unread: bool, config: &config::Config) -> ExitCode {
    let store = match Store::open(&config.database) {
        Ok(store) => store,
        Err(error) => return fail(&error),
    };
    let mode = if oneshot {
        inbox::Mode::Oneshot
    } else {
        inbox::Mode::Live
    };
    match inbox::run(&store, mode, start_unread, config) {
        Ok(Some(id)) => jump(&id, &config.database),
        Ok(None) => ExitCode::SUCCESS,
        Err(error) => fail(&error),
    }
}

fn next(config: &config::Config) -> ExitCode {
    let store = match Store::open(&config.database) {
        Ok(store) => store,
        Err(error) => return fail(&error),
    };
    let unread = match store.list(1, true) {
        Ok(unread) => unread,
        Err(error) => return fail(&error),
    };
    if unread.is_empty() {
        return fail("no unread events");
    }
    drop(store);
    open_inbox(false, true, config)
}

fn clear(database: &Path) -> ExitCode {
    let store = match Store::open(database) {
        Ok(store) => store,
        Err(error) => return fail(&error),
    };
    match store.clear_read() {
        Ok(0) => {
            println!("no read events were removed");
            ExitCode::SUCCESS
        }
        Ok(count) => {
            println!("cleared {count} read events");
            ExitCode::SUCCESS
        }
        Err(error) => fail(&error),
    }
}

fn jump(id: &str, database: &Path) -> ExitCode {
    let store = match Store::open(database) {
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
