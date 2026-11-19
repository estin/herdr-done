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
use signal_hook::flag::register;

pub fn run(program: &OsString, args: &[OsString]) -> ExitCode {
    let event = Event::new(program, args, herdr::current_context());
    let store = match Store::open_default() {
        Ok(store) => Some(store),
        Err(error) => {
            eprintln!("hd: {error}");
            None
        }
    };
    persist(store.as_ref(), &event);

    let mut child = match Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            eprintln!("hd: failed to run {}: {error}", program.to_string_lossy());
            persist(store.as_ref(), &event.failed_to_start(error.to_string()));
            return ExitCode::FAILURE;
        }
    };

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
    let signal_requested = monitor.stop();
    let signal = status_signal(status);
    let code = exit_code(status);
    let code_value = i32::from(code);
    let finished = if signal.is_some() || signal_requested {
        event.interrupted(code_value, signal)
    } else {
        event.completed(code_value, signal)
    };
    persist(store.as_ref(), &finished);
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
    requested: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    _registrations: Vec<SigId>,
}

#[cfg(unix)]
impl SignalMonitor {
    fn start(pid: u32) -> Self {
        let requested = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let mut registrations = Vec::new();
        for signal in [SIGINT, SIGTERM] {
            match register(signal, Arc::clone(&requested)) {
                Ok(id) => registrations.push(id),
                Err(error) => eprintln!("hd: cannot handle signal {signal}: {error}"),
            }
        }

        let thread_requested = Arc::clone(&requested);
        let thread_stop = Arc::clone(&stop);
        let child_pid = i32::try_from(pid).unwrap_or(0);
        let handle = thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                if thread_requested.load(Ordering::Relaxed) {
                    let _ = kill(Pid::from_raw(child_pid), Signal::SIGTERM);
                    return;
                }
                thread::sleep(Duration::from_millis(20));
            }
        });
        Self {
            requested,
            stop,
            handle: Some(handle),
            _registrations: registrations,
        }
    }

    fn stop(self) -> bool {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle {
            let _ = handle.join();
        }
        self.requested.load(Ordering::Relaxed)
    }
}

#[cfg(not(unix))]
struct SignalMonitor;

#[cfg(not(unix))]
impl SignalMonitor {
    fn start(_pid: u32) -> Self {
        Self
    }

    fn stop(self) -> bool {
        false
    }
}
