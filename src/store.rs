use crate::event::{Event, EventStatus, HerdrContext, now_millis};
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

const COLUMNS: &str = "id, program, args_json, cwd, started_at, finished_at, duration_ms, \
                       status, exit_code, signal, failure_reason, read_at, session_id, \
                       workspace_id, workspace_name, tab_id, pane_id, terminal_id, hostname";

const MIGRATION_TABLE: &str = "
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at INTEGER NOT NULL
);
";

const MIGRATION_V1: &str = "
CREATE TABLE events (
    id TEXT PRIMARY KEY,
    program TEXT NOT NULL,
    args_json TEXT NOT NULL,
    cwd TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    finished_at INTEGER,
    duration_ms INTEGER,
    status TEXT NOT NULL,
    exit_code INTEGER,
    signal INTEGER,
    failure_reason TEXT,
    read_at INTEGER,
    session_id TEXT,
    workspace_id TEXT,
    workspace_name TEXT,
    tab_id TEXT,
    pane_id TEXT,
    terminal_id TEXT,
    hostname TEXT NOT NULL
);

CREATE INDEX idx_events_finished ON events(finished_at DESC);
CREATE INDEX idx_events_unread ON events(read_at, finished_at DESC);
CREATE INDEX idx_events_pane ON events(session_id, pane_id);
";

pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let mut last_error = None;
        for attempt in 0..20 {
            match Self::open_once(path) {
                Ok(store) => return Ok(store),
                Err(error) if is_busy(&error) => {
                    last_error = Some(error);
                    thread::sleep(Duration::from_millis(25 + attempt * 5));
                }
                Err(error) => return Err(error),
            }
        }
        Err(last_error.ok_or_else(|| storage_error("database is busy".to_owned()))?)
    }

    fn open_once(path: &Path) -> Result<Self, String> {
        create_parent(path)?;
        let connection = Connection::open(path).map_err(storage_error)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(storage_error)?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(storage_error)?;
        restrict_file(path)?;
        let mut store = Self { connection };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_default() -> Result<Self, String> {
        Self::open(default_database_path()?)
    }

    pub fn upsert(&self, event: &Event) -> Result<(), String> {
        let args_json = serde_json::to_string(&event.args).map_err(storage_error)?;
        self.connection
            .execute(
                "INSERT OR REPLACE INTO events (
                    id, program, args_json, cwd, started_at, finished_at, duration_ms,
                    status, exit_code, signal, failure_reason, read_at,
                    session_id, workspace_id, workspace_name, tab_id, pane_id, terminal_id,
                    hostname
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                           ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
                params![
                    &event.id,
                    &event.program,
                    &args_json,
                    &event.cwd,
                    event.started_at,
                    event.finished_at,
                    event.duration_ms,
                    event.status.as_str(),
                    event.exit_code,
                    event.signal,
                    &event.failure_reason,
                    event.read_at,
                    &event.herdr.session_id,
                    &event.herdr.workspace_id,
                    &event.herdr.workspace_name,
                    &event.herdr.tab_id,
                    &event.herdr.pane_id,
                    &event.herdr.terminal_id,
                    &event.hostname,
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Option<Event>, String> {
        self.connection
            .query_row(
                &format!("SELECT {COLUMNS} FROM events WHERE id = ?1"),
                params![id],
                row_to_event,
            )
            .optional()
            .map_err(storage_error)
    }

    pub fn list(&self, limit: usize, unread_only: bool) -> Result<Vec<Event>, String> {
        let filter = if unread_only {
            "WHERE status <> 'running' AND read_at IS NULL"
        } else {
            "WHERE status <> 'running'"
        };
        let sql = format!(
            "SELECT {COLUMNS} FROM events {filter} ORDER BY finished_at DESC, started_at DESC LIMIT ?1"
        );
        let mut statement = self.connection.prepare(&sql).map_err(storage_error)?;
        let rows = statement
            .query_map(
                params![i64::try_from(limit).unwrap_or(i64::MAX)],
                row_to_event,
            )
            .map_err(storage_error)?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row.map_err(storage_error)?);
        }
        Ok(events)
    }

    pub fn set_read(&self, id: &str, read: bool) -> Result<bool, String> {
        let rows = if read {
            self.connection
                .execute(
                    "UPDATE events SET read_at = ?1 WHERE id = ?2",
                    params![now_millis(), id],
                )
                .map_err(storage_error)?
        } else {
            self.connection
                .execute(
                    "UPDATE events SET read_at = NULL WHERE id = ?2",
                    params![id],
                )
                .map_err(storage_error)?
        };
        Ok(rows > 0)
    }

    fn migrate(&mut self) -> Result<(), String> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        transaction
            .execute_batch(MIGRATION_TABLE)
            .map_err(storage_error)?;
        let version: Option<i64> = transaction
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .map_err(storage_error)?;
        if version.unwrap_or(0) < 1 {
            transaction
                .execute_batch(MIGRATION_V1)
                .map_err(storage_error)?;
            transaction
                .execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (1, ?1)",
                    params![now_millis()],
                )
                .map_err(storage_error)?;
        }
        transaction.commit().map_err(storage_error)
    }
}

pub fn default_database_path() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("HD_DATABASE")
        && !path.is_empty()
    {
        return Ok(PathBuf::from(path));
    }
    let base = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .ok_or_else(|| "cannot find the local data directory".to_owned())?;
    Ok(base.join("herdr-done").join("events.db"))
}

fn row_to_event(row: &Row<'_>) -> Result<Event, rusqlite::Error> {
    let args_json: String = row.get(2)?;
    let args: Vec<String> = serde_json::from_str(&args_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let status_text: String = row.get(7)?;
    let status = EventStatus::parse(&status_text).map_err(|message| {
        rusqlite::Error::FromSqlConversionFailure(
            7,
            rusqlite::types::Type::Text,
            Box::new(InvalidStatus(message)),
        )
    })?;
    Ok(Event {
        id: row.get(0)?,
        program: row.get(1)?,
        args,
        cwd: row.get(3)?,
        started_at: row.get(4)?,
        finished_at: row.get(5)?,
        duration_ms: row.get(6)?,
        status,
        exit_code: row.get(8)?,
        signal: row.get(9)?,
        failure_reason: row.get(10)?,
        read_at: row.get(11)?,
        herdr: HerdrContext {
            session_id: row.get(12)?,
            workspace_id: row.get(13)?,
            workspace_name: row.get(14)?,
            tab_id: row.get(15)?,
            pane_id: row.get(16)?,
            terminal_id: row.get(17)?,
        },
        hostname: row.get(18)?,
    })
}

#[derive(Debug)]
struct InvalidStatus(String);

impl fmt::Display for InvalidStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for InvalidStatus {}

fn is_busy(error: &str) -> bool {
    let message = error.to_ascii_lowercase();
    message.contains("locked") || message.contains("busy")
}

fn storage_error(error: impl fmt::Display) -> String {
    format!("storage error: {error}")
}

fn create_parent(path: &Path) -> Result<(), String> {
    let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(storage_error)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(storage_error)?;
    }
    Ok(())
}

fn restrict_file(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(storage_error)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Store;
    use crate::event::{Event, EventStatus, HerdrContext};
    use std::ffi::{OsStr, OsString};
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn creates_schema_and_saves_updates() -> Result<(), String> {
        let directory = tempdir().map_err(|error| error.to_string())?;
        let path = directory.path().join("events.db");
        let store = Store::open(&path)?;
        let event = Event::new(
            OsStr::new("cargo"),
            &[OsString::from("test")],
            HerdrContext::default(),
        );
        store.upsert(&event)?;
        let finished = event.clone().completed(0, None);
        store.upsert(&finished)?;

        let saved = store.get(&event.id)?.ok_or("event not found".to_owned())?;
        assert_eq!(saved.status, EventStatus::Completed);
        assert_eq!(saved.args, vec!["test".to_owned()]);

        let table: String = store
            .connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'events'",
                [],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        assert_eq!(table, "events");
        Ok(())
    }

    #[test]
    fn lists_newest_first_and_filters_unread() -> Result<(), String> {
        let directory = tempdir().map_err(|error| error.to_string())?;
        let store = Store::open(directory.path().join("events.db"))?;
        let first =
            Event::new(OsStr::new("first"), &[], HerdrContext::default()).completed(0, None);
        thread::sleep(Duration::from_millis(5));
        let second =
            Event::new(OsStr::new("second"), &[], HerdrContext::default()).completed(1, None);
        store.upsert(&first)?;
        store.upsert(&second)?;
        assert!(store.set_read(&second.id, true)?);

        let all = store.list(10, false)?;
        assert_eq!(all[0].id, second.id);
        assert_eq!(all[1].id, first.id);
        let unread = store.list(10, true)?;
        assert_eq!(unread.len(), 1);
        assert_eq!(unread[0].id, first.id);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn database_is_owner_only() -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempdir().map_err(|error| error.to_string())?;
        let path = directory.path().join("events.db");
        let _store = Store::open(&path)?;
        let mode = std::fs::metadata(&path)
            .map_err(|error| error.to_string())?
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        Ok(())
    }
}
