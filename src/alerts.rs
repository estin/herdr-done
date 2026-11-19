use crate::event::Event;
use notify_rust::Notification;
use std::env;
use std::ffi::OsString;
use std::process::Command;

const TITLE: &str = "Command completed";

pub fn deliver(sound: bool, notify: bool, event: &Event) {
    deliver_with(sound, notify, event, &play_sound, &send_notification);
}

fn deliver_with(
    sound: bool,
    notify: bool,
    event: &Event,
    play: &dyn Fn(&str, &str) -> Result<(), String>,
    send: &dyn Fn(&str, &str) -> Result<(), String>,
) {
    let body = alert_body(event);
    if sound && let Err(error) = play(TITLE, &body) {
        eprintln!("hd: sound alert failed: {error}");
    }
    if notify && let Err(error) = send(TITLE, &body) {
        eprintln!("hd: notification alert failed: {error}");
    }
}

fn alert_body(event: &Event) -> String {
    format!("{}: {}", event.command_line(), event.result_label())
}

fn play_sound(title: &str, body: &str) -> Result<(), String> {
    if env::var_os("HERDR_ENV").as_deref() != Some(OsString::from("1").as_os_str()) {
        return Err("Herdr context is unavailable".to_owned());
    }
    let binary = env::var_os("HERDR_BIN_PATH").unwrap_or_else(|| OsString::from("herdr"));
    let output = Command::new(binary)
        .args([
            "notification",
            "show",
            title,
            "--body",
            body,
            "--sound",
            "done",
        ])
        .output()
        .map_err(|error| format!("cannot run Herdr: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if detail.is_empty() {
        return Err("Herdr rejected the sound alert".to_owned());
    }
    Err(detail)
}

fn send_notification(title: &str, body: &str) -> Result<(), String> {
    Notification::new()
        .summary(title)
        .body(body)
        .show()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{TITLE, deliver_with};
    use crate::event::{Event, HerdrContext};
    use std::cell::{Cell, RefCell};
    use std::ffi::OsStr;

    fn event() -> Event {
        Event::new(
            OsStr::new("cargo"),
            &["build".into(), "--release".into()],
            HerdrContext::default(),
        )
        .completed(42, None)
    }

    #[test]
    fn alert_body_contains_command_and_outcome() {
        let sent = RefCell::new(Vec::new());
        deliver_with(false, true, &event(), &|_, _| Ok(()), &|title, body| {
            sent.borrow_mut().push(format!("{title}: {body}"));
            Ok(())
        });
        assert_eq!(
            sent.into_inner(),
            vec![format!("{TITLE}: cargo build --release: exit 42")]
        );
    }

    #[test]
    fn alerts_are_selected_independently() {
        let sound_calls = RefCell::new(0);
        let notify_calls = RefCell::new(0);
        deliver_with(
            true,
            false,
            &event(),
            &|_, _| {
                *sound_calls.borrow_mut() += 1;
                Ok(())
            },
            &|_, _| {
                *notify_calls.borrow_mut() += 1;
                Ok(())
            },
        );
        assert_eq!(*sound_calls.borrow(), 1);
        assert_eq!(*notify_calls.borrow(), 0);
    }

    #[test]
    fn a_failed_sound_does_not_block_notification() {
        let notified = Cell::new(false);
        deliver_with(
            true,
            true,
            &event(),
            &|_, _| Err("sound failed".to_owned()),
            &|_, _| {
                notified.set(true);
                Ok(())
            },
        );
        assert!(notified.get());
    }

    #[test]
    fn no_alerts_are_sent_when_not_requested() {
        let called = Cell::new(false);
        deliver_with(
            false,
            false,
            &event(),
            &|_, _| {
                called.set(true);
                Ok(())
            },
            &|_, _| {
                called.set(true);
                Ok(())
            },
        );
        assert!(!called.get());
    }
}
