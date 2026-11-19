use crate::event::{Event, now_millis};
use crate::herdr;
use crate::store::Store;
use crossterm::event::{self, Event as TerminalEvent, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Wrap,
};
use ratatui::{Frame, Terminal};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

const REFRESH_INTERVAL: Duration = Duration::from_secs(1);
const QUIT_INTERVAL: Duration = Duration::from_secs(2);
const QUIT_NOTICE: &str = "To quit, press Ctrl-C twice";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Live,
    Oneshot,
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
    message: Option<String>,
    last_ctrl_c: Option<Instant>,
}

impl App {
    pub fn new(events: Vec<Event>) -> Self {
        Self {
            events,
            selected: 0,
            view: View::List,
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
        match (self.view, key.code) {
            (_, KeyCode::Char('q') | KeyCode::Esc) => {
                self.set_message(QUIT_NOTICE.to_owned());
                Action::None
            }
            (_, KeyCode::Down | KeyCode::Char('j')) => {
                self.move_selection(1);
                Action::None
            }
            (_, KeyCode::Up | KeyCode::Char('k')) => {
                self.move_selection(-1);
                Action::None
            }
            (View::List, KeyCode::Enter) => {
                self.view = View::Detail;
                Action::None
            }
            (_, KeyCode::Char('n')) => {
                self.select_next_unread();
                Action::None
            }
            (_, KeyCode::Char('r')) => self
                .selected_event()
                .map_or(Action::None, |event| Action::Read(event.id.clone(), true)),
            (_, KeyCode::Char('u')) => self
                .selected_event()
                .map_or(Action::None, |event| Action::Read(event.id.clone(), false)),
            (_, KeyCode::Char('J')) => self
                .selected_event()
                .map_or(Action::None, |event| Action::Jump(event.id.clone())),
            (_, KeyCode::Char('c')) => Action::Clear,
            _ => Action::None,
        }
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

pub fn run(store: &Store, mode: Mode, start_unread: bool) -> Result<Option<String>, String> {
    let events = store.list(100, false)?;
    let mut app = App::new(events);
    if start_unread {
        app.select_first_unread();
    }
    let outcome = run_terminal(&mut app, store, mode)?;
    Ok(outcome)
}

fn run_terminal(app: &mut App, store: &Store, mode: Mode) -> Result<Option<String>, String> {
    enable_raw_mode().map_err(|error| format!("terminal error: {error}"))?;
    let mut stdout = io::stdout();
    if let Err(error) = execute!(stdout, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(format!("terminal error: {error}"));
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|error| format!("terminal error: {error}"))?;

    let result = event_loop(&mut terminal, app, store, mode);
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
) -> Result<Option<String>, String> {
    let mut last_refresh = Instant::now();
    loop {
        terminal
            .draw(|frame| render(frame, app))
            .map_err(|error| format!("terminal error: {error}"))?;
        if event::poll(Duration::from_millis(200))
            .map_err(|error| format!("input error: {error}"))?
        {
            let TerminalEvent::Key(key) =
                event::read().map_err(|error| format!("input error: {error}"))?
            else {
                continue;
            };
            let action = app.handle_key(key);
            match apply_action(app, store, action, mode, &herdr::focus_event) {
                Outcome::Continue => {}
                Outcome::Quit => return Ok(None),
                Outcome::Jump(id) => return Ok(Some(id)),
            }
        }
        if last_refresh.elapsed() >= REFRESH_INTERVAL {
            app.refresh(store.list(100, false)?);
            last_refresh = Instant::now();
        }
    }
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

fn render(frame: &mut Frame<'_>, app: &App) {
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
    frame.render_widget(
        Paragraph::new(title).style(Style::default().add_modifier(Modifier::BOLD)),
        areas[0],
    );

    match app.view {
        View::List => render_list(frame, app, areas[1]),
        View::Detail => render_detail(frame, app, areas[1]),
    }

    let footer = app.message.clone().unwrap_or_else(|| {
        "j/k navigate - Enter inspect - n next unread - r read - u unread - c clear read - J jump - Ctrl-C twice quit"
            .to_owned()
    });
    frame.render_widget(Paragraph::new(footer), areas[2]);
}

fn render_list(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .events
        .iter()
        .map(|event| {
            let marker = if event.read_at.is_none() { "*" } else { " " };
            let first = format!(
                "{marker} {:<44} {:<16} {}",
                event.command_line(),
                event.result_label(),
                event.duration_label()
            );
            let second = format!("    {}", event.location_label());
            ListItem::new(vec![
                Line::from(first),
                Line::from(Span::styled(second, Style::default().fg(Color::Gray))),
            ])
        })
        .collect();
    let mut state = ListState::default();
    state.select(Some(app.selected));
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Events"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    StatefulWidget::render(list, area, frame.buffer_mut(), &mut state);
}

fn render_detail(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let lines: Vec<Line> = match app.selected_event() {
        Some(event) => vec![
            detail_line("Command", event.command_line()),
            detail_line("Working directory", event.cwd.clone()),
            detail_line("Result", event.result_label()),
            detail_line("Started", event.started_label()),
            detail_line("Finished", event.finished_label()),
            detail_line("Duration", event.duration_label()),
            detail_line("Host", event.hostname.clone()),
            detail_line("Read", if event.read_at.is_none() { "no" } else { "yes" }),
            detail_line(
                "Herdr session",
                event.herdr.session_id.as_deref().unwrap_or("-"),
            ),
            detail_line("Herdr location", event.location_label()),
        ],
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

fn detail_line(label: &'static str, value: impl Into<String>) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(value.into()),
    ])
}

#[cfg(test)]
mod tests {
    use super::{
        Action, App, Mode, Outcome, QUIT_INTERVAL, QUIT_NOTICE, View, apply_action, render,
    };
    use crate::event::{Event, HerdrContext, now_millis};
    use crate::store::Store;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::ffi::OsStr;
    use std::time::{Duration, Instant};
    use tempfile::tempdir;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
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
    }

    fn events() -> Vec<Event> {
        ["first", "second", "third"]
            .into_iter()
            .map(named_event)
            .collect()
    }

    #[test]
    fn navigation_moves_selection() {
        let mut app = App::new(events());
        assert_eq!(app.handle_key(key(KeyCode::Down)), Action::None);
        assert_eq!(app.selected_index(), 1);
        assert_eq!(app.handle_key(key(KeyCode::Char('k'))), Action::None);
        assert_eq!(app.selected_index(), 0);
        assert_eq!(app.handle_key(key(KeyCode::Up)), Action::None);
        assert_eq!(app.selected_index(), 2);
    }

    #[test]
    fn refresh_keeps_the_selected_event_by_id() {
        let mut app = App::new(events());
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
        let mut app = App::new(events());
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
        let mut app = App::new(events());
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
        let mut app = App::new(events());
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
        let mut app = App::new(events());
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
        let mut app = App::new(events());
        let first = Instant::now();
        assert_eq!(app.register_ctrl_c(first), Action::None);
        assert_eq!(
            app.register_ctrl_c(first + Duration::from_millis(900)),
            Action::Quit
        );
    }

    #[test]
    fn expired_first_ctrl_c_resets_the_timer() {
        let mut app = App::new(events());
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
        let mut app = App::new(events());
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(app.handle_key(ctrl_c), Action::None);
        assert_eq!(app.message.as_deref(), Some(QUIT_NOTICE));
    }

    #[test]
    fn q_and_esc_show_quit_notice_without_changing_view() {
        for view in [View::List, View::Detail] {
            for code in [KeyCode::Char('q'), KeyCode::Esc] {
                let mut app = App::new(events());
                app.view = view;
                assert_eq!(app.handle_key(key(code)), Action::None);
                assert_eq!(app.view, view);
                assert_eq!(app.message.as_deref(), Some(QUIT_NOTICE));
            }
        }
    }

    #[test]
    fn clear_key_returns_clear_action() {
        let mut app = App::new(events());
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

        let mut app = App::new(store.list(100, false)?);
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
        let app = App::new(events());
        assert_eq!(app.unread_count(), 3);
    }

    #[test]
    fn read_action_targets_selected_event() {
        let mut app = App::new(events());
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
        let mut app = App::new(events.clone());
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
        let mut app = App::new(events());
        let read_id = app.events[1].id.clone();
        app.apply_read(&read_id, true);
        let _ = app.handle_key(key(KeyCode::Char('n')));
        assert_eq!(app.selected_index(), 2);
        assert_eq!(app.view, View::Detail);
    }

    #[test]
    fn footer_shows_clear_shortcut() -> Result<(), String> {
        let app = App::new(events());
        let backend = TestBackend::new(120, 12);
        let mut terminal =
            Terminal::new(backend).map_err(|error| format!("terminal error: {error}"))?;
        terminal
            .draw(|frame| render(frame, &app))
            .map_err(|error| format!("terminal error: {error}"))?;
        let content: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(content.contains("c clear read"));
        assert!(content.contains("Ctrl-C twice quit"));
        Ok(())
    }

    #[test]
    fn first_ctrl_c_renders_quit_notice() -> Result<(), String> {
        let mut app = App::new(events());
        app.register_ctrl_c(Instant::now());
        let backend = TestBackend::new(120, 12);
        let mut terminal =
            Terminal::new(backend).map_err(|error| format!("terminal error: {error}"))?;
        terminal
            .draw(|frame| render(frame, &app))
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
    fn detail_view_shows_event_fields() -> Result<(), String> {
        let mut app = App::new(events());
        app.view = View::Detail;
        let backend = TestBackend::new(100, 20);
        let mut terminal =
            Terminal::new(backend).map_err(|error| format!("terminal error: {error}"))?;
        terminal
            .draw(|frame| render(frame, &app))
            .map_err(|error| format!("terminal error: {error}"))?;
        let content: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(content.contains("Command:"));
        assert!(content.contains("first"));
        assert!(content.contains("Result:"));
        assert!(content.contains("worker / backend / build"));
        assert!(!content.contains("w1:p1"));
        Ok(())
    }
}
