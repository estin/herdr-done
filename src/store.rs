use crate::event::{Event, EventKind, EventStatus, HerdrContext, now_millis};
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

const COLUMNS: &str = "id, program, args_json, cwd, started_at, finished_at, duration_ms, \
                       status, exit_code, signal, failure_reason, read_at, session_id, \
                       workspace_id, workspace_name, tab_id, pane_id, terminal_id, hostname, \
                       tab_name, pane_name, kind";

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

const MIGRATION_V2: &str = "
ALTER TABLE events ADD COLUMN tab_name TEXT;
ALTER TABLE events ADD COLUMN pane_name TEXT;
";

const MIGRATION_V3: &str = "
ALTER TABLE events ADD COLUMN kind TEXT NOT NULL DEFAULT 'run';
";

pub struct Store {
    connection: Connection,
    database: PathBuf,
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
        let mut store = Self {
            connection,
            // Watchers report absolute paths; keep ours absolute too.
            database: fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.database
    }

    pub fn wal_path(&self) -> PathBuf {
        let mut value = self.database.as_os_str().to_os_string();
        value.push("-wal");
        PathBuf::from(value)
    }

    pub fn upsert(&self, event: &Event) -> Result<(), String> {
        let args_json = serde_json::to_string(&event.args).map_err(storage_error)?;
        self.connection
            .execute(
                "INSERT OR REPLACE INTO events (
                    id, program, args_json, cwd, started_at, finished_at, duration_ms,
                    status, exit_code, signal, failure_reason, read_at,
                    session_id, workspace_id, workspace_name, tab_id, pane_id, terminal_id,
                    hostname, tab_name, pane_name, kind
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                           ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
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
                    &event.herdr.tab_name,
                    &event.herdr.pane_name,
                    event.kind.as_str(),
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

    pub fn clear_read(&self) -> Result<usize, String> {
        self.connection
            .execute("DELETE FROM events WHERE read_at IS NOT NULL", [])
            .map_err(storage_error)
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
        let version: Option<i64> = transaction
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .map_err(storage_error)?;
        if version.unwrap_or(0) < 2 {
            transaction
                .execute_batch(MIGRATION_V2)
                .map_err(storage_error)?;
            transaction
                .execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (2, ?1)",
                    params![now_millis()],
                )
                .map_err(storage_error)?;
        }
        let version: Option<i64> = transaction
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .map_err(storage_error)?;
        if version.unwrap_or(0) < 3 {
            transaction
                .execute_batch(MIGRATION_V3)
                .map_err(storage_error)?;
            transaction
                .execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (3, ?1)",
                    params![now_millis()],
                )
                .map_err(storage_error)?;
        }
        transaction.commit().map_err(storage_error)
    }
}

pub fn default_database_path() -> Result<PathBuf, String> {
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
            Box::new(InvalidValue(message)),
        )
    })?;
    let kind_text: String = row.get(21)?;
    let kind = EventKind::parse(&kind_text).map_err(|message| {
        rusqlite::Error::FromSqlConversionFailure(
            21,
            rusqlite::types::Type::Text,
            Box::new(InvalidValue(message)),
        )
    })?;
    Ok(Event {
        id: row.get(0)?,
        kind,
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
            tab_name: row.get(19)?,
            pane_id: row.get(16)?,
            pane_name: row.get(20)?,
            terminal_id: row.get(17)?,
        },
        hostname: row.get(18)?,
    })
}

#[derive(Debug)]
struct InvalidValue(String);

impl fmt::Display for InvalidValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for InvalidValue {}

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
    use super::{MIGRATION_TABLE, MIGRATION_V1, Store};
    use crate::event::{Event, EventStatus, HerdrContext};
    use rusqlite::Connection;
    use std::ffi::{OsStr, OsString};
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn exposes_database_and_wal_paths() -> Result<(), String> {
        let directory = tempdir().map_err(|error| error.to_string())?;
        let path = directory.path().join("events.db");
        let store = Store::open(&path)?;
        assert_eq!(store.path(), path);
        assert_eq!(store.wal_path(), directory.path().join("events.db-wal"));
        Ok(())
    }

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
    fn loads_events_created_before_name_columns() -> Result<(), String> {
        let directory = tempdir().map_err(|error| error.to_string())?;
        let path = directory.path().join("events.db");
        let connection = Connection::open(&path).map_err(|error| error.to_string())?;
        connection
            .execute_batch(MIGRATION_TABLE)
            .map_err(|error| error.to_string())?;
        connection
            .execute_batch(MIGRATION_V1)
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (1, 0)",
                [],
            )
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT INTO events (
                    id, program, args_json, cwd, started_at, finished_at, duration_ms,
                    status, exit_code, signal, failure_reason, read_at,
                    session_id, workspace_id, workspace_name, tab_id, pane_id, terminal_id,
                    hostname
                 ) VALUES (
                    'old', 'cargo', '[]', '/tmp', 1, 2, 1, 'completed', 0, NULL,
                    NULL, NULL, 'job', 'w1', 'worker', 'w1:t1', 'w1:p1', NULL, 'host'
                 )",
                [],
            )
            .map_err(|error| error.to_string())?;
        drop(connection);

        let store = Store::open(&path)?;
        let event = store
            .get("old")?
            .ok_or_else(|| "old event missing".to_owned())?;
        assert_eq!(event.herdr.workspace_name.as_deref(), Some("worker"));
        assert_eq!(event.herdr.tab_name, None);
        assert_eq!(event.herdr.pane_name, None);
        assert_eq!(event.herdr.pane_id.as_deref(), Some("w1:p1"));
        assert!(!event.is_manual_event());
        Ok(())
    }

    #[test]
    fn saves_event_kind_with_events() -> Result<(), String> {
        let directory = tempdir().map_err(|error| error.to_string())?;
        let store = Store::open(directory.path().join("events.db"))?;
        let manual = Event::new(
            OsStr::new("deployment is ready"),
            &[],
            HerdrContext::default(),
        )
        .completed(0, None)
        .manual();
        let run = Event::new(OsStr::new("date"), &[], HerdrContext::default()).completed(0, None);
        store.upsert(&manual)?;
        store.upsert(&run)?;

        assert!(
            store
                .get(&manual.id)?
                .ok_or_else(|| "manual event missing".to_owned())?
                .is_manual_event()
        );
        assert!(
            !store
                .get(&run.id)?
                .ok_or_else(|| "run event missing".to_owned())?
                .is_manual_event()
        );
        Ok(())
    }

    #[test]
    fn saves_name_columns_with_events() -> Result<(), String> {
        let directory = tempdir().map_err(|error| error.to_string())?;
        let store = Store::open(directory.path().join("events.db"))?;
        let event = Event::new(
            OsStr::new("cargo"),
            &[],
            HerdrContext {
                workspace_name: Some("worker".to_owned()),
                tab_name: Some("backend".to_owned()),
                pane_name: Some("build".to_owned()),
                ..HerdrContext::default()
            },
        )
        .completed(0, None);
        store.upsert(&event)?;
        let saved = store.get(&event.id)?.ok_or("event missing".to_owned())?;
        assert_eq!(saved.herdr.tab_name.as_deref(), Some("backend"));
        assert_eq!(saved.herdr.pane_name.as_deref(), Some("build"));
        Ok(())
    }

    #[test]
    fn clear_read_keeps_unread_events() -> Result<(), String> {
        let directory = tempdir().map_err(|error| error.to_string())?;
        let store = Store::open(directory.path().join("events.db"))?;
        let read = Event::new(OsStr::new("read"), &[], HerdrContext::default()).completed(0, None);
        let unread =
            Event::new(OsStr::new("unread"), &[], HerdrContext::default()).completed(0, None);
        store.upsert(&read)?;
        store.upsert(&unread)?;
        store.set_read(&read.id, true)?;

        assert_eq!(store.clear_read()?, 1);
        assert!(store.get(&read.id)?.is_none());
        assert!(store.get(&unread.id)?.is_some());
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

    #[cfg(unix)]
    #[test]
    fn opening_database_does_not_change_parent_permissions() -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempdir().map_err(|error| error.to_string())?;
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o755))
            .map_err(|error| error.to_string())?;
        let path = directory.path().join("events.db");

        let _store = Store::open(path)?;

        let mode = std::fs::metadata(directory.path())
            .map_err(|error| error.to_string())?
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o755);
        Ok(())
    }
}
