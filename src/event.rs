use chrono::{DateTime, Local};
use std::env;
use std::ffi::{OsStr, OsString};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use ulid::Ulid;

pub fn now_millis() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_millis()).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventStatus {
    Running,
    Completed,
    FailedToStart,
    Interrupted,
}

impl EventStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::FailedToStart => "failed_to_start",
            Self::Interrupted => "interrupted",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed_to_start" => Ok(Self::FailedToStart),
            "interrupted" => Ok(Self::Interrupted),
            other => Err(format!("unknown event status: {other}")),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HerdrContext {
    pub session_id: Option<String>,
    pub workspace_id: Option<String>,
    pub workspace_name: Option<String>,
    pub tab_id: Option<String>,
    pub tab_name: Option<String>,
    pub pane_id: Option<String>,
    pub pane_name: Option<String>,
    pub terminal_id: Option<String>,
}

impl HerdrContext {
    pub fn is_empty(&self) -> bool {
        self.session_id.is_none()
            && self.workspace_id.is_none()
            && self.workspace_name.is_none()
            && self.tab_id.is_none()
            && self.tab_name.is_none()
            && self.pane_id.is_none()
            && self.pane_name.is_none()
            && self.terminal_id.is_none()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    pub id: String,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub duration_ms: Option<i64>,
    pub status: EventStatus,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub failure_reason: Option<String>,
    pub read_at: Option<i64>,
    pub herdr: HerdrContext,
    pub hostname: String,
}

impl Event {
    pub fn new(program: &OsStr, args: &[OsString], herdr: HerdrContext) -> Self {
        let started_at = now_millis();
        Self {
            id: Ulid::generate().to_string(),
            program: program.to_string_lossy().into_owned(),
            args: args
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
            cwd: env::current_dir().map_or_else(
                |_| "unknown".to_owned(),
                |path| path.to_string_lossy().into_owned(),
            ),
            started_at,
            finished_at: None,
            duration_ms: None,
            status: EventStatus::Running,
            exit_code: None,
            signal: None,
            failure_reason: None,
            read_at: None,
            herdr,
            hostname: hostname(),
        }
    }

    pub fn completed(self, exit_code: i32, signal: Option<i32>) -> Self {
        self.finish(EventStatus::Completed, Some(exit_code), signal, None)
    }

    pub fn interrupted(self, exit_code: i32, signal: Option<i32>) -> Self {
        self.finish(EventStatus::Interrupted, Some(exit_code), signal, None)
    }

    pub fn failed_to_start(self, reason: String) -> Self {
        self.finish(EventStatus::FailedToStart, None, None, Some(reason))
    }

    fn finish(
        mut self,
        status: EventStatus,
        exit_code: Option<i32>,
        signal: Option<i32>,
        failure_reason: Option<String>,
    ) -> Self {
        let finished_at = now_millis();
        self.duration_ms = Some(finished_at - self.started_at);
        self.finished_at = Some(finished_at);
        self.status = status;
        self.exit_code = exit_code;
        self.signal = signal;
        self.failure_reason = failure_reason;
        self
    }

    pub fn command_line(&self) -> String {
        let mut parts = Vec::with_capacity(self.args.len() + 1);
        parts.push(self.program.as_str());
        parts.extend(self.args.iter().map(String::as_str));
        parts.join(" ")
    }

    pub fn result_label(&self) -> String {
        match self.status {
            EventStatus::Running => "running".to_owned(),
            EventStatus::Completed => match self.exit_code {
                Some(code) => format!("exit {code}"),
                None => "completed".to_owned(),
            },
            EventStatus::FailedToStart => match &self.failure_reason {
                Some(reason) => format!("launch failed: {reason}"),
                None => "launch failed".to_owned(),
            },
            EventStatus::Interrupted => match self.signal {
                Some(signal) => format!("interrupted by signal {signal}"),
                None => "interrupted".to_owned(),
            },
        }
    }

    pub fn duration_label(&self) -> String {
        match self.duration_ms {
            Some(ms) if ms >= 0 => {
                let duration = Duration::from_millis(u64::try_from(ms).unwrap_or(u64::MAX));
                humantime::format_duration(duration).to_string()
            }
            Some(ms) => ms.to_string(),
            None => "-".to_owned(),
        }
    }

    pub fn started_label(&self) -> String {
        format_timestamp(self.started_at)
    }

    pub fn finished_label(&self) -> String {
        match self.finished_at {
            Some(ms) => format_timestamp(ms),
            None => "running".to_owned(),
        }
    }

    pub fn location_label(&self) -> String {
        let names: Vec<&str> = [
            self.herdr.workspace_name.as_deref(),
            self.herdr.tab_name.as_deref(),
            self.herdr.pane_name.as_deref(),
        ]
        .into_iter()
        .flatten()
        .filter(|name| !name.is_empty())
        .collect();
        if !names.is_empty() {
            return names.join(" / ");
        }
        if self.herdr.workspace_id.is_some()
            || self.herdr.tab_id.is_some()
            || self.herdr.pane_id.is_some()
        {
            return "location unavailable".to_owned();
        }
        "outside Herdr".to_owned()
    }
}

fn format_timestamp(milliseconds: i64) -> String {
    DateTime::from_timestamp_millis(milliseconds).map_or_else(
        || milliseconds.to_string(),
        |value| {
            value
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        },
    )
}

fn hostname() -> String {
    env::var("HOSTNAME")
        .or_else(|_| env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{Event, EventStatus, HerdrContext};
    use std::ffi::{OsStr, OsString};

    fn event() -> Event {
        Event::new(
            OsStr::new("cargo"),
            &[OsString::from("build"), OsString::from("--release")],
            HerdrContext::default(),
        )
    }

    #[test]
    fn new_event_starts_in_running_state() {
        let event = event();
        assert_eq!(event.status, EventStatus::Running);
        assert_eq!(event.command_line(), "cargo build --release");
        assert_eq!(event.finished_at, None);
    }

    #[test]
    fn completion_sets_finish_fields() {
        let event = event().completed(42, None);
        assert_eq!(event.status, EventStatus::Completed);
        assert_eq!(event.exit_code, Some(42));
        assert!(event.finished_at.is_some());
        assert!(event.duration_ms.is_some());
        assert_eq!(event.result_label(), "exit 42");
    }

    #[test]
    fn location_uses_names_not_ids() {
        let event = Event::new(
            OsStr::new("cargo"),
            &[],
            HerdrContext {
                workspace_id: Some("w1".to_owned()),
                workspace_name: Some("worker".to_owned()),
                tab_id: Some("w1:t1".to_owned()),
                tab_name: Some("backend".to_owned()),
                pane_id: Some("w1:p1".to_owned()),
                pane_name: Some("build".to_owned()),
                ..HerdrContext::default()
            },
        );
        assert_eq!(event.location_label(), "worker / backend / build");
    }

    #[test]
    fn location_hides_ids_when_names_are_missing() {
        let event = Event::new(
            OsStr::new("cargo"),
            &[],
            HerdrContext {
                workspace_id: Some("w1".to_owned()),
                tab_id: Some("w1:t1".to_owned()),
                pane_id: Some("w1:p1".to_owned()),
                ..HerdrContext::default()
            },
        );
        assert_eq!(event.location_label(), "location unavailable");
    }

    #[test]
    fn location_shows_only_available_names() {
        let event = Event::new(
            OsStr::new("cargo"),
            &[],
            HerdrContext {
                workspace_id: Some("w1".to_owned()),
                workspace_name: Some("worker".to_owned()),
                tab_id: Some("w1:t1".to_owned()),
                pane_id: Some("w1:p1".to_owned()),
                pane_name: Some("build".to_owned()),
                ..HerdrContext::default()
            },
        );
        assert_eq!(event.location_label(), "worker / build");
    }

    #[test]
    fn status_values_round_trip() {
        for status in [
            EventStatus::Running,
            EventStatus::Completed,
            EventStatus::FailedToStart,
            EventStatus::Interrupted,
        ] {
            assert_eq!(EventStatus::parse(status.as_str()), Ok(status));
        }
    }
}
