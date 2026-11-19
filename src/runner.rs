use crate::alerts;
use crate::event::Event;
use crate::herdr;
use crate::store::Store;
use std::ffi::OsString;
use std::process::{Command, ExitCode, ExitStatus, Stdio};
use std::time::Duration;

#[cfg(unix)]
use nix::sys::signal::{Signal, kill};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

#[cfg(unix)]
use nix::unistd::Pid;
#[cfg(unix)]
use signal_hook::SigId;
#[cfg(unix)]
use signal_hook::consts::{SIGINT, SIGTERM};
#[cfg(unix)]
use signal_hook::flag::register_usize;

pub fn run(program: &OsString, args: &[OsString], sound: bool, notify: bool) -> ExitCode {
    let mut event = Event::new(program, args, herdr::current_context());
    let store = match Store::open_default() {
        Ok(store) => Some(store),
        Err(error) => {
            eprintln!("hd: {error}");
            None
        }
    };
    persist(store.as_ref(), &event);

    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(unix)]
    let monitor = SignalMonitor::new();
    event.started_at = crate::event::now_millis();
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            eprintln!("hd: failed to run {}: {error}", program.to_string_lossy());
            persist(store.as_ref(), &event.failed_to_start(error.to_string()));
            return ExitCode::FAILURE;
        }
    };

    #[cfg(unix)]
    let monitor = monitor.forward(child.id());
    #[cfg(not(unix))]
    let monitor = SignalMonitor::start(child.id());
    let status = match child.wait() {
        Ok(status) => status,
        Err(error) => {
            eprintln!("hd: could not wait for the child: {error}");
            let _monitor = monitor.stop();
            persist(store.as_ref(), &event.failed_to_start(error.to_string()));
            return ExitCode::FAILURE;
        }
    };
    let requested_signal = monitor.stop();
    let signal = status_signal(status).or(requested_signal);
    let code = signal.map_or_else(|| exit_code(status), exit_code_from_signal);
    let code_value = i32::from(code);
    let finished = if signal.is_some() {
        event.interrupted(code_value, signal)
    } else {
        event.completed(code_value, signal)
    };
    persist(store.as_ref(), &finished);
    alerts::deliver(sound, notify, &finished);
    ExitCode::from(code)
}

fn persist(store: Option<&Store>, event: &Event) {
    if let Some(store) = store
        && let Err(error) = store.upsert(event)
    {
        eprintln!("hd: {error}");
    }
}

fn exit_code(status: ExitStatus) -> u8 {
    if let Some(code) = status.code() {
        return u8::try_from(code).unwrap_or(1);
    }
    match status_signal(status) {
        Some(signal) => u8::try_from(128 + signal).unwrap_or(1),
        None => 1,
    }
}

fn exit_code_from_signal(signal: i32) -> u8 {
    u8::try_from(128 + signal).unwrap_or(1)
}

#[cfg(unix)]
fn status_signal(status: ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn status_signal(_status: ExitStatus) -> Option<i32> {
    None
}

#[cfg(unix)]
struct SignalMonitor {
    requested: Arc<std::sync::atomic::AtomicUsize>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    _registrations: Vec<SigId>,
}

#[cfg(unix)]
impl SignalMonitor {
    fn new() -> Self {
        use std::sync::atomic::AtomicUsize;

        let requested = Arc::new(AtomicUsize::new(0));
        let mut registrations = Vec::new();
        for signal in [SIGINT, SIGTERM] {
            let Ok(value) = usize::try_from(signal) else {
                continue;
            };
            match register_usize(signal, Arc::clone(&requested), value) {
                Ok(id) => registrations.push(id),
                Err(error) => eprintln!("hd: cannot handle signal {signal}: {error}"),
            }
        }

        Self {
            requested,
            stop: Arc::new(AtomicBool::new(false)),
            handle: None,
            _registrations: registrations,
        }
    }

    fn forward(mut self, pid: u32) -> Self {
        let thread_requested = Arc::clone(&self.requested);
        let thread_stop = Arc::clone(&self.stop);
        let process_group = i32::try_from(pid).ok();
        self.handle = Some(thread::spawn(move || {
            loop {
                let requested = thread_requested.load(Ordering::Relaxed);
                if requested != 0 {
                    if let Some(process_group) = process_group
                        && let Ok(signal_number) = i32::try_from(requested)
                        && let Ok(signal) = Signal::try_from(signal_number)
                    {
                        let _ = kill(Pid::from_raw(-process_group), signal);
                    }
                    return;
                }
                if thread_stop.load(Ordering::Relaxed) {
                    return;
                }
                thread::sleep(Duration::from_millis(20));
            }
        }));
        self
    }

    fn stop(self) -> Option<i32> {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle {
            let _ = handle.join();
        }
        i32::try_from(self.requested.load(Ordering::Relaxed))
            .ok()
            .filter(|signal| *signal != 0)
    }
}

#[cfg(not(unix))]
struct SignalMonitor;

#[cfg(not(unix))]
impl SignalMonitor {
    fn start(_pid: u32) -> Self {
        Self
    }

    fn stop(self) -> Option<i32> {
        None
    }
}
