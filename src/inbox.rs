use crate::config::{Bindings, Config, Theme};
use crate::event::{Event, EventStatus, now_millis};
use crate::herdr;
use crate::store::Store;
use crossterm::event::{self, Event as TerminalEvent, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use notify::event::Event as WatcherEvent;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Wrap,
};
use ratatui::{Frame, Terminal};
use std::io::{self, Stdout};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

const QUIT_INTERVAL: Duration = Duration::from_secs(2);
const QUIT_NOTICE: &str = "To quit, press Ctrl-C twice";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Live,
    Oneshot,
}

struct WalWatcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<notify::Result<WatcherEvent>>,
    wal_path: PathBuf,
}

impl WalWatcher {
    fn start(mode: Mode, wal_path: &Path) -> Option<Self> {
        if mode == Mode::Oneshot {
            return None;
        }
        let (sender, receiver) = mpsc::channel();
        let mut watcher =
            notify::recommended_watcher(move |result: notify::Result<WatcherEvent>| {
                let _ = sender.send(result);
            })
            .ok()?;
        let directory = wal_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        watcher.watch(directory, RecursiveMode::NonRecursive).ok()?;
        Some(Self {
            _watcher: watcher,
            receiver,
            wal_path: wal_path.to_path_buf(),
        })
    }

    fn pending(&self) -> bool {
        self.receiver.try_iter().any(|result| {
            result.is_ok_and(|event| event.paths.iter().any(|path| path == &self.wal_path))
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Outcome {
    Continue,
    Quit,
    Jump(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum View {
    List,
    Detail,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    None,
    Quit,
    Read(String, bool),
    Jump(String),
    Clear,
}

pub struct App {
    events: Vec<Event>,
    selected: usize,
    view: View,
    bindings: Bindings,
    message: Option<String>,
    last_ctrl_c: Option<Instant>,
}

impl App {
    pub fn new(events: Vec<Event>, bindings: Bindings) -> Self {
        Self {
            events,
            selected: 0,
            view: View::List,
            bindings,
            message: None,
            last_ctrl_c: None,
        }
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn selected_event(&self) -> Option<&Event> {
        self.events.get(self.selected)
    }

    pub fn unread_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.read_at.is_none())
            .count()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        self.message = None;
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return self.register_ctrl_c(Instant::now());
        }
        if self.bindings.back.contains(&key) {
            return match self.view {
                View::Detail => {
                    self.view = View::List;
                    Action::None
                }
                View::List => {
                    self.set_message(QUIT_NOTICE.to_owned());
                    Action::None
                }
            };
        }
        if self.view == View::List && self.bindings.inspect.contains(&key) {
            self.view = View::Detail;
            return Action::None;
        }
        if self.bindings.down.contains(&key) {
            self.move_selection(1);
            return Action::None;
        }
        if self.bindings.up.contains(&key) {
            self.move_selection(-1);
            return Action::None;
        }
        if self.bindings.next_unread.contains(&key) {
            self.select_next_unread();
            return Action::None;
        }
        if self.bindings.mark_read.contains(&key) {
            return self
                .selected_event()
                .map_or(Action::None, |event| Action::Read(event.id.clone(), true));
        }
        if self.bindings.mark_unread.contains(&key) {
            return self
                .selected_event()
                .map_or(Action::None, |event| Action::Read(event.id.clone(), false));
        }
        if self.bindings.jump.contains(&key) {
            return self
                .selected_event()
                .map_or(Action::None, |event| Action::Jump(event.id.clone()));
        }
        if self.bindings.clear_read.contains(&key) {
            return Action::Clear;
        }
        Action::None
    }

    pub fn register_ctrl_c(&mut self, now: Instant) -> Action {
        if let Some(last) = self.last_ctrl_c
            && now.duration_since(last) < QUIT_INTERVAL
        {
            self.last_ctrl_c = None;
            return Action::Quit;
        }
        self.last_ctrl_c = Some(now);
        self.set_message(QUIT_NOTICE.to_owned());
        Action::None
    }

    pub fn apply_read(&mut self, id: &str, read: bool) {
        if let Some(event) = self.events.iter_mut().find(|event| event.id == id) {
            event.read_at = read.then(now_millis);
        }
    }

    pub fn set_message(&mut self, message: String) {
        self.message = Some(message);
    }

    pub fn refresh(&mut self, events: Vec<Event>) {
        let selected_id = self.selected_event().map(|event| event.id.clone());
        self.events = events;
        self.selected = selected_id
            .as_deref()
            .and_then(|id| self.events.iter().position(|event| event.id == id))
            .unwrap_or(0);
    }

    fn move_selection(&mut self, delta: isize) {
        if self.events.is_empty() {
            return;
        }
        let length = isize::try_from(self.events.len()).unwrap_or(isize::MAX);
        let current = isize::try_from(self.selected).unwrap_or(0);
        let next = (current + delta).rem_euclid(length);
        self.selected = usize::try_from(next).unwrap_or(0);
    }

    fn select_first_unread(&mut self) {
        if let Some(index) = self.events.iter().position(|event| event.read_at.is_none()) {
            self.selected = index;
        }
    }

    fn select_next_unread(&mut self) {
        if self.events.is_empty() {
            return;
        }
        for offset in 1..=self.events.len() {
            let index = (self.selected + offset) % self.events.len();
            if self.events[index].read_at.is_none() {
                self.selected = index;
                self.view = View::Detail;
                self.message = None;
                return;
            }
        }
        self.message = Some("no unread events".to_owned());
    }
}

pub fn run(
    store: &Store,
    mode: Mode,
    start_unread: bool,
    config: &Config,
) -> Result<Option<String>, String> {
    let events = store.list(100, false)?;
    let mut app = App::new(events, config.bindings.clone());
    if start_unread {
        app.select_first_unread();
    }
    let wal_watcher = WalWatcher::start(mode, &store.wal_path());
    if mode == Mode::Live && wal_watcher.is_none() {
        app.set_message(format!(
            "watch unavailable; refreshing every {}",
            humantime::format_duration(config.refresh_interval)
        ));
    }
    let outcome = run_terminal(&mut app, store, mode, config, wal_watcher.as_ref())?;
    Ok(outcome)
}

fn run_terminal(
    app: &mut App,
    store: &Store,
    mode: Mode,
    config: &Config,
    wal_watcher: Option<&WalWatcher>,
) -> Result<Option<String>, String> {
    enable_raw_mode().map_err(|error| format!("terminal error: {error}"))?;
    let mut stdout = io::stdout();
    if let Err(error) = execute!(stdout, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(format!("terminal error: {error}"));
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|error| format!("terminal error: {error}"))?;

    let result = event_loop(&mut terminal, app, store, mode, config, wal_watcher);
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    store: &Store,
    mode: Mode,
    config: &Config,
    wal_watcher: Option<&WalWatcher>,
) -> Result<Option<String>, String> {
    let mut last_refresh = Instant::now();
    loop {
        terminal
            .draw(|frame| render(frame, app, &config.theme))
            .map_err(|error| format!("terminal error: {error}"))?;
        if event::poll(Duration::from_millis(200))
            .map_err(|error| format!("input error: {error}"))?
            && let TerminalEvent::Key(key) =
                event::read().map_err(|error| format!("input error: {error}"))?
        {
            let action = app.handle_key(key);
            match apply_action(app, store, action, mode, &herdr::focus_event) {
                Outcome::Continue => {}
                Outcome::Quit => return Ok(None),
                Outcome::Jump(id) => return Ok(Some(id)),
            }
        }
        let now = Instant::now();
        let wal_change = wal_watcher.is_some_and(WalWatcher::pending);
        if should_refresh(wal_change, last_refresh, now, config.refresh_interval) {
            app.refresh(store.list(100, false)?);
            last_refresh = now;
        }
    }
}

fn should_refresh(
    wal_change: bool,
    last_refresh: Instant,
    now: Instant,
    interval: Duration,
) -> bool {
    wal_change || now.duration_since(last_refresh) >= interval
}

fn apply_action(
    app: &mut App,
    store: &Store,
    action: Action,
    mode: Mode,
    focus: &dyn Fn(&Store, &str) -> Result<String, String>,
) -> Outcome {
    match action {
        Action::None => {}
        Action::Quit => return Outcome::Quit,
        Action::Jump(id) => match mode {
            Mode::Live => match focus(store, &id) {
                Ok(pane_id) => app.set_message(format!("focused pane {pane_id}")),
                Err(error) => app.set_message(error),
            },
            Mode::Oneshot => return Outcome::Jump(id),
        },
        Action::Read(id, read) => match store.set_read(&id, read) {
            Ok(true) => app.apply_read(&id, read),
            Ok(false) => app.set_message("event no longer exists".to_owned()),
            Err(error) => app.set_message(error),
        },
        Action::Clear => match store.clear_read() {
            Ok(count) => match store.list(100, false) {
                Ok(events) => {
                    app.refresh(events);
                    app.set_message(format!("cleared {count} read events"));
                }
                Err(error) => app.set_message(error),
            },
            Err(error) => app.set_message(error),
        },
    }
    Outcome::Continue
}

fn render(frame: &mut Frame<'_>, app: &App, theme: &Theme) {
    if let Some(background) = theme.background {
        let mut style = Style::default().bg(background);
        if let Some(text) = theme.text {
            style = style.fg(text);
        }
        frame.render_widget(Block::default().style(style), frame.area());
    }
    let areas = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(frame.area());
    let title = format!(
        "Command Inbox - {} unread{}",
        app.unread_count(),
        if app.view == View::Detail {
            " - details"
        } else {
            ""
        }
    );
    frame.render_widget(Paragraph::new(title).style(theme.title), areas[0]);

    match app.view {
        View::List => render_list(frame, app, areas[1], theme),
        View::Detail => render_detail(frame, app, areas[1], theme),
    }

    let footer = app.message.clone().unwrap_or_else(|| app.bindings.help());
    frame.render_widget(Paragraph::new(footer), areas[2]);
}

fn render_list(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect, theme: &Theme) {
    let items: Vec<ListItem> = app
        .events
        .iter()
        .map(|event| {
            let unread = event.read_at.is_none();
            let marker = if unread { "*" } else { " " };
            let marker_style = if unread {
                theme.unread
            } else {
                Style::default()
            };
            let first = Line::from(vec![
                Span::styled(marker, marker_style),
                Span::raw(format!(" {:<44} ", event.command_line())),
                Span::styled(
                    format!("{:<16} ", event.result_label()),
                    result_style(theme, event),
                ),
                Span::raw(format!("Started: {} ", event.started_time_label())),
                Span::raw(event.duration_label()),
            ]);
            let second = format!("    {}", event.location_label());
            ListItem::new(vec![
                first,
                Line::from(Span::styled(second, theme.secondary)),
            ])
        })
        .collect();
    let mut state = ListState::default();
    state.select(Some(app.selected));
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Events"))
        .highlight_style(theme.selection);
    StatefulWidget::render(list, area, frame.buffer_mut(), &mut state);
}

fn render_detail(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect, theme: &Theme) {
    let lines: Vec<Line> = match app.selected_event() {
        Some(event) => {
            let mut lines = vec![
                detail_line(
                    if event.is_manual_event() {
                        "Message"
                    } else {
                        "Command"
                    },
                    event.command_line(),
                    Style::default(),
                    theme,
                ),
                detail_line(
                    "Working directory",
                    event.cwd.clone(),
                    Style::default(),
                    theme,
                ),
                detail_line(
                    "Result",
                    event.result_label(),
                    result_style(theme, event),
                    theme,
                ),
            ];
            let timing = if event.is_manual_event() {
                vec![detail_line(
                    "Invocation",
                    event.started_label(),
                    Style::default(),
                    theme,
                )]
            } else {
                vec![
                    detail_line("Started", event.started_label(), Style::default(), theme),
                    detail_line("Finished", event.finished_label(), Style::default(), theme),
                    detail_line("Duration", event.duration_label(), Style::default(), theme),
                ]
            };
            lines.extend(timing);
            lines.extend([
                detail_line("Host", event.hostname.clone(), Style::default(), theme),
                detail_line(
                    "Read",
                    if event.read_at.is_none() { "no" } else { "yes" },
                    Style::default(),
                    theme,
                ),
                detail_line(
                    "Herdr session",
                    event.herdr.session_id.as_deref().unwrap_or("-"),
                    Style::default(),
                    theme,
                ),
                detail_line(
                    "Herdr location",
                    event.location_label(),
                    Style::default(),
                    theme,
                ),
            ]);
            lines
        }
        None => vec![Line::from("No events")],
    };
    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Event details"),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn detail_line(
    label: &'static str,
    value: impl Into<String>,
    value_style: Style,
    theme: &Theme,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            theme.secondary.add_modifier(Modifier::BOLD),
        ),
        Span::styled(value.into(), value_style),
    ])
}

fn result_style(theme: &Theme, event: &Event) -> Style {
    match event.status {
        EventStatus::Running => theme.running,
        EventStatus::Completed if event.exit_code.unwrap_or(0) == 0 => theme.success,
        EventStatus::Completed | EventStatus::FailedToStart | EventStatus::Interrupted => {
            theme.failure
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Action, App, Mode, Outcome, QUIT_INTERVAL, QUIT_NOTICE, View, WalWatcher, apply_action,
        render, should_refresh,
    };
    use crate::config::{Bindings, Theme};
    use crate::event::{Event, HerdrContext, now_millis};
    use crate::store::Store;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use notify::event::{Event as WatcherEvent, EventAttributes, EventKind, ModifyKind};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;
    use std::ffi::{OsStr, OsString};
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::{self, Receiver};
    use std::time::{Duration, Instant};
    use tempfile::tempdir;

    fn watcher_event(path: PathBuf) -> WatcherEvent {
        WatcherEvent {
            kind: EventKind::Modify(ModifyKind::Any),
            paths: vec![path],
            attrs: EventAttributes::new(),
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn char_key(ch: char) -> KeyEvent {
        key(KeyCode::Char(ch))
    }

    fn app() -> App {
        App::new(events(), Bindings::default())
    }

    fn named_event(name: &str) -> Event {
        Event::new(
            OsStr::new(name),
            &[],
            HerdrContext {
                workspace_name: Some("worker".to_owned()),
                tab_name: Some("backend".to_owned()),
                pane_id: Some("w1:p1".to_owned()),
                pane_name: Some("build".to_owned()),
                ..HerdrContext::default()
            },
        )
        .completed(0, None)
        .manual()
    }

    fn timed_event(name: &str, args: &[&str], started_at: i64, manual: bool) -> Event {
        let args = args.iter().map(OsString::from).collect::<Vec<_>>();
        let event = Event::new(OsStr::new(name), &args, HerdrContext::default()).completed(0, None);
        let mut event = if manual { event.manual() } else { event };
        event.started_at = started_at;
        event.finished_at = Some(started_at);
        event.duration_ms = Some(0);
        event
    }

    fn rendered(app: &App) -> Result<String, String> {
        rendered_with_width(app, 160)
    }

    fn rendered_with_width(app: &App, width: u16) -> Result<String, String> {
        let backend = TestBackend::new(width, 30);
        let mut terminal =
            Terminal::new(backend).map_err(|error| format!("terminal error: {error}"))?;
        terminal
            .draw(|frame| render(frame, app, &Theme::default()))
            .map_err(|error| format!("terminal error: {error}"))?;
        Ok(terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect())
    }

    fn events() -> Vec<Event> {
        ["first", "second", "third"]
            .into_iter()
            .map(named_event)
            .collect()
    }

    fn watcher_with(
        receiver: Receiver<notify::Result<WatcherEvent>>,
    ) -> Result<WalWatcher, String> {
        Ok(WalWatcher {
            _watcher: notify::recommended_watcher(|_| {}).map_err(|error| error.to_string())?,
            receiver,
            wal_path: PathBuf::from("events.db-wal"),
        })
    }

    #[test]
    fn wal_path_event_marks_a_pending_change() -> Result<(), String> {
        let (sender, receiver) = mpsc::channel::<notify::Result<WatcherEvent>>();
        sender
            .send(Ok(watcher_event(PathBuf::from("events.db-wal"))))
            .map_err(|error| error.to_string())?;
        assert!(watcher_with(receiver)?.pending());
        Ok(())
    }

    #[test]
    fn non_wal_path_does_not_mark_a_pending_change() -> Result<(), String> {
        let (sender, receiver) = mpsc::channel::<notify::Result<WatcherEvent>>();
        sender
            .send(Ok(watcher_event(PathBuf::from("events.db"))))
            .map_err(|error| error.to_string())?;
        assert!(!watcher_with(receiver)?.pending());
        Ok(())
    }

    #[test]
    fn wal_change_beats_the_interval() {
        let now = Instant::now();
        let recent = now.checked_sub(Duration::from_millis(10)).unwrap_or(now);
        assert!(should_refresh(true, recent, now, Duration::from_secs(10)));
    }

    #[test]
    fn interval_fallback_still_refreshes() {
        let now = Instant::now();
        let old = now
            .checked_sub(Duration::from_secs(10) + Duration::from_millis(1))
            .unwrap_or(now);
        assert!(should_refresh(false, old, now, Duration::from_secs(10)));
        let recent = now.checked_sub(Duration::from_millis(10)).unwrap_or(now);
        assert!(!should_refresh(false, recent, now, Duration::from_secs(10)));
    }

    #[test]
    fn oneshot_mode_creates_no_watcher() {
        assert!(WalWatcher::start(Mode::Oneshot, Path::new("events.db-wal")).is_none());
    }

    #[test]
    fn navigation_moves_selection() {
        let mut app = app();
        assert_eq!(app.handle_key(key(KeyCode::Down)), Action::None);
        assert_eq!(app.selected_index(), 1);
        assert_eq!(app.handle_key(key(KeyCode::Char('k'))), Action::None);
        assert_eq!(app.selected_index(), 0);
        assert_eq!(app.handle_key(key(KeyCode::Up)), Action::None);
        assert_eq!(app.selected_index(), 2);
    }

    #[test]
    fn refresh_keeps_the_selected_event_by_id() {
        let mut app = app();
        app.selected = 1;
        let selected = app.events[1].clone();
        app.refresh(vec![named_event("newest"), selected.clone()]);
        assert_eq!(app.selected_index(), 1);
        assert_eq!(
            app.selected_event().map(|event| event.id.as_str()),
            Some(selected.id.as_str())
        );
    }

    #[test]
    fn refresh_selects_newest_when_selection_disappears() {
        let mut app = app();
        app.selected = 1;
        app.refresh(vec![named_event("newest")]);
        assert_eq!(app.selected_index(), 0);
        assert_eq!(
            app.selected_event().map(Event::command_line),
            Some("newest".to_owned())
        );
    }

    #[test]
    fn live_jump_stays_open_after_success() -> Result<(), String> {
        let directory = tempdir().map_err(|error| error.to_string())?;
        let store = Store::open(directory.path().join("events.db"))?;
        let mut app = app();
        let id = app
            .selected_event()
            .map(|event| event.id.clone())
            .ok_or_else(|| "no event selected".to_owned())?;

        let outcome = apply_action(&mut app, &store, Action::Jump(id), Mode::Live, &|_, _| {
            Ok("w1:p1".to_owned())
        });

        assert_eq!(outcome, Outcome::Continue);
        assert_eq!(app.message.as_deref(), Some("focused pane w1:p1"));
        Ok(())
    }

    #[test]
    fn live_jump_shows_focus_errors() -> Result<(), String> {
        let directory = tempdir().map_err(|error| error.to_string())?;
        let store = Store::open(directory.path().join("events.db"))?;
        let mut app = app();
        let id = app
            .selected_event()
            .map(|event| event.id.clone())
            .ok_or_else(|| "no event selected".to_owned())?;

        let outcome = apply_action(&mut app, &store, Action::Jump(id), Mode::Live, &|_, _| {
            Err("pane not found".to_owned())
        });

        assert_eq!(outcome, Outcome::Continue);
        assert_eq!(app.message.as_deref(), Some("pane not found"));
        Ok(())
    }

    #[test]
    fn oneshot_jump_closes_the_inbox() -> Result<(), String> {
        let directory = tempdir().map_err(|error| error.to_string())?;
        let store = Store::open(directory.path().join("events.db"))?;
        let mut app = app();
        let id = app
            .selected_event()
            .map(|event| event.id.clone())
            .ok_or_else(|| "no event selected".to_owned())?;

        let outcome = apply_action(
            &mut app,
            &store,
            Action::Jump(id.clone()),
            Mode::Oneshot,
            &|_, _| Err("focus must not run".to_owned()),
        );

        assert_eq!(outcome, Outcome::Jump(id));
        Ok(())
    }

    #[test]
    fn double_ctrl_c_exits_within_the_interval() {
        let mut app = app();
        let first = Instant::now();
        assert_eq!(app.register_ctrl_c(first), Action::None);
        assert_eq!(
            app.register_ctrl_c(first + Duration::from_millis(900)),
            Action::Quit
        );
    }

    #[test]
    fn expired_first_ctrl_c_resets_the_timer() {
        let mut app = app();
        let first = Instant::now();
        assert_eq!(app.register_ctrl_c(first), Action::None);
        let expired = first + QUIT_INTERVAL + Duration::from_millis(1);
        assert_eq!(app.register_ctrl_c(expired), Action::None);
        assert_eq!(
            app.register_ctrl_c(expired + Duration::from_millis(100)),
            Action::Quit
        );
    }

    #[test]
    fn ctrl_c_key_event_is_recognized() {
        let mut app = app();
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(app.handle_key(ctrl_c), Action::None);
        assert_eq!(app.message.as_deref(), Some(QUIT_NOTICE));
    }

    #[test]
    fn q_and_esc_show_quit_notice_in_list_view() {
        for code in [KeyCode::Char('q'), KeyCode::Esc] {
            let mut app = app();
            assert_eq!(app.handle_key(key(code)), Action::None);
            assert_eq!(app.view, View::List);
            assert_eq!(app.message.as_deref(), Some(QUIT_NOTICE));
        }
    }

    #[test]
    fn q_and_esc_return_to_list_from_detail_view() {
        for code in [KeyCode::Char('q'), KeyCode::Esc] {
            let mut app = app();
            app.view = View::Detail;
            assert_eq!(app.handle_key(key(code)), Action::None);
            assert_eq!(app.view, View::List);
            assert_eq!(app.message, None);
        }
    }

    #[test]
    fn clear_key_returns_clear_action() {
        let mut app = app();
        assert_eq!(app.handle_key(key(KeyCode::Char('c'))), Action::Clear);
    }

    #[test]
    fn clear_action_refreshes_and_keeps_selection() -> Result<(), String> {
        let directory = tempdir().map_err(|error| error.to_string())?;
        let store = Store::open(directory.path().join("events.db"))?;
        let all_events = events();
        let read_id = all_events[0].id.clone();
        let selected_id = all_events[1].id.clone();
        for event in all_events {
            store.upsert(&event)?;
        }
        store.set_read(&read_id, true)?;

        let mut app = App::new(store.list(100, false)?, Bindings::default());
        app.selected = app
            .events
            .iter()
            .position(|event| event.id == selected_id)
            .ok_or_else(|| "selected event missing".to_owned())?;
        let outcome = apply_action(&mut app, &store, Action::Clear, Mode::Live, &|_, _| {
            Err("focus must not run".to_owned())
        });

        assert_eq!(outcome, Outcome::Continue);
        assert_eq!(
            app.selected_event().map(|event| event.id.as_str()),
            Some(selected_id.as_str())
        );
        assert!(app.events.iter().all(|event| event.read_at.is_none()));
        assert_eq!(app.message.as_deref(), Some("cleared 1 read events"));
        assert_eq!(store.list(100, false)?.len(), 2);
        Ok(())
    }

    #[test]
    fn opening_inbox_does_not_mark_events_read() {
        let app = app();
        assert_eq!(app.unread_count(), 3);
    }

    #[test]
    fn read_action_targets_selected_event() {
        let mut app = app();
        let id = app
            .selected_event()
            .map(|event| event.id.clone())
            .unwrap_or_default();
        assert_eq!(
            app.handle_key(key(KeyCode::Char('r'))),
            Action::Read(id.clone(), true)
        );
        app.apply_read(&id, true);
        assert_eq!(app.unread_count(), 2);
    }

    #[test]
    fn first_unread_selection_does_not_mark_event_read() {
        let mut events = events();
        events[0].read_at = Some(now_millis());
        let mut app = App::new(events.clone(), Bindings::default());
        app.select_first_unread();
        assert_eq!(
            app.selected_event().map(|event| event.id.as_str()),
            Some(events[1].id.as_str())
        );
        assert!(
            app.selected_event()
                .is_some_and(|event| event.read_at.is_none())
        );
    }

    #[test]
    fn next_unread_skips_read_events() {
        let mut app = app();
        let read_id = app.events[1].id.clone();
        app.apply_read(&read_id, true);
        let _ = app.handle_key(key(KeyCode::Char('n')));
        assert_eq!(app.selected_index(), 2);
        assert_eq!(app.view, View::Detail);
    }

    #[test]
    fn custom_binding_runs_action_and_replaces_default() {
        let bindings = Bindings {
            mark_read: vec![char_key('x')],
            ..Bindings::default()
        };
        let mut app = App::new(events(), bindings);
        let id = app
            .selected_event()
            .map(|event| event.id.clone())
            .unwrap_or_default();
        assert_eq!(app.handle_key(char_key('x')), Action::Read(id, true));
        assert_eq!(app.handle_key(char_key('r')), Action::None);
    }

    #[test]
    fn list_rows_show_invocation_time_for_events_and_runs() -> Result<(), String> {
        let manual = timed_event("manual", &[], 1_700_000_000_000, true);
        let run = timed_event("cargo", &["build"], 1_700_000_060_000, false);
        let app = App::new(vec![manual.clone(), run.clone()], Bindings::default());
        let content = rendered_with_width(&app, 100)?;
        assert!(
            content.contains(&format!("Started: {}", manual.started_time_label())),
            "{content}"
        );
        assert!(
            content.contains(&format!("Started: {}", run.started_time_label())),
            "{content}"
        );
        assert!(!content.contains(&manual.started_label()), "{content}");
        assert!(content.contains("0s"), "{content}");
        Ok(())
    }

    #[test]
    fn footer_reflects_active_bindings() -> Result<(), String> {
        let bindings = Bindings {
            clear_read: vec![char_key('x')],
            ..Bindings::default()
        };
        let app = App::new(events(), bindings);
        let backend = TestBackend::new(120, 12);
        let mut terminal =
            Terminal::new(backend).map_err(|error| format!("terminal error: {error}"))?;
        terminal
            .draw(|frame| render(frame, &app, &Theme::default()))
            .map_err(|error| format!("terminal error: {error}"))?;
        let content: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(content.contains("x clear read"), "{content}");
        assert!(!content.contains("c clear read"), "{content}");
        Ok(())
    }

    #[test]
    fn footer_shows_clear_shortcut() -> Result<(), String> {
        let app = app();
        let backend = TestBackend::new(120, 12);
        let mut terminal =
            Terminal::new(backend).map_err(|error| format!("terminal error: {error}"))?;
        terminal
            .draw(|frame| render(frame, &app, &Theme::default()))
            .map_err(|error| format!("terminal error: {error}"))?;
        let content: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(content.contains("c clear read"));
        assert!(content.contains("Ctrl-C twice"));
        Ok(())
    }

    #[test]
    fn first_ctrl_c_renders_quit_notice() -> Result<(), String> {
        let mut app = app();
        app.register_ctrl_c(Instant::now());
        let backend = TestBackend::new(120, 12);
        let mut terminal =
            Terminal::new(backend).map_err(|error| format!("terminal error: {error}"))?;
        terminal
            .draw(|frame| render(frame, &app, &Theme::default()))
            .map_err(|error| format!("terminal error: {error}"))?;
        let content: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(content.contains(QUIT_NOTICE));
        Ok(())
    }

    #[test]
    fn scheme_background_fills_empty_cells() -> Result<(), String> {
        let app = app();
        let theme = Theme {
            background: Some(Color::Rgb(1, 2, 3)),
            text: Some(Color::Rgb(4, 5, 6)),
            ..Theme::default()
        };
        let backend = TestBackend::new(80, 12);
        let mut terminal =
            Terminal::new(backend).map_err(|error| format!("terminal error: {error}"))?;
        terminal
            .draw(|frame| render(frame, &app, &theme))
            .map_err(|error| format!("terminal error: {error}"))?;
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(79, 0)].bg, Color::Rgb(1, 2, 3));
        assert_eq!(buffer[(79, 0)].fg, Color::Rgb(4, 5, 6));
        assert!(
            buffer
                .content
                .iter()
                .all(|cell| cell.bg == Color::Rgb(1, 2, 3))
        );
        Ok(())
    }

    #[test]
    fn without_background_cells_keep_the_default() -> Result<(), String> {
        let app = app();
        let backend = TestBackend::new(80, 12);
        let mut terminal =
            Terminal::new(backend).map_err(|error| format!("terminal error: {error}"))?;
        terminal
            .draw(|frame| render(frame, &app, &Theme::default()))
            .map_err(|error| format!("terminal error: {error}"))?;
        let buffer = terminal.backend().buffer();
        assert!(buffer.content.iter().all(|cell| cell.bg == Color::Reset));
        Ok(())
    }

    #[test]
    fn detail_view_shows_event_fields() -> Result<(), String> {
        let mut app = app();
        app.view = View::Detail;
        let content = rendered(&app)?;
        assert!(content.contains("Message:"));
        assert!(content.contains("first"));
        assert!(content.contains("Result:"));
        assert!(content.contains("worker / backend / build"));
        assert!(!content.contains("w1:p1"));
        assert!(!content.contains("Finished:"));
        assert!(!content.contains("Duration:"));
        Ok(())
    }

    #[test]
    fn manual_detail_omits_finish_and_duration() -> Result<(), String> {
        let event = timed_event("deployment is ready", &[], 1_700_000_000_000, true);
        let mut app = App::new(vec![event.clone()], Bindings::default());
        app.view = View::Detail;
        let content = rendered(&app)?;
        assert!(content.contains("Message:"));
        assert!(content.contains("deployment is ready"));
        assert!(content.contains(&format!("Invocation: {}", event.started_label())));
        assert!(!content.contains("Finished:"));
        assert!(!content.contains("Duration:"));
        Ok(())
    }

    #[test]
    fn run_detail_keeps_all_timing_fields() -> Result<(), String> {
        let event = timed_event("date", &[], 1_700_000_060_000, false);
        let mut app = App::new(vec![event.clone()], Bindings::default());
        app.view = View::Detail;
        let content = rendered(&app)?;
        assert!(content.contains("Command:"));
        assert!(content.contains("date"));
        assert!(content.contains(&format!("Started: {}", event.started_label())));
        assert!(content.contains(&format!("Finished: {}", event.finished_label())));
        assert!(content.contains(&format!("Duration: {}", event.duration_label())));
        assert!(!content.contains("Message:"));
        Ok(())
    }
}
