use crate::event::{Event, now_millis};
use crate::store::Store;
use crossterm::event::{self, Event as TerminalEvent, KeyCode};
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
use std::time::Duration;

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
}

pub struct App {
    events: Vec<Event>,
    selected: usize,
    view: View,
    message: Option<String>,
}

impl App {
    pub fn new(events: Vec<Event>) -> Self {
        Self {
            events,
            selected: 0,
            view: View::List,
            message: None,
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

    pub fn handle_key(&mut self, key: KeyCode) -> Action {
        self.message = None;
        match (self.view, key) {
            (View::List, KeyCode::Char('q')) => Action::Quit,
            (View::Detail, KeyCode::Esc) => {
                self.view = View::List;
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
            _ => Action::None,
        }
    }

    pub fn apply_read(&mut self, id: &str, read: bool) {
        if let Some(event) = self.events.iter_mut().find(|event| event.id == id) {
            event.read_at = read.then(now_millis);
        }
    }

    pub fn set_message(&mut self, message: String) {
        self.message = Some(message);
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

pub fn run(store: &Store) -> Result<Option<String>, String> {
    let events = store.list(100, false)?;
    let mut app = App::new(events);
    let outcome = run_terminal(&mut app, store)?;
    Ok(outcome)
}

fn run_terminal(app: &mut App, store: &Store) -> Result<Option<String>, String> {
    enable_raw_mode().map_err(|error| format!("terminal error: {error}"))?;
    let mut stdout = io::stdout();
    if let Err(error) = execute!(stdout, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(format!("terminal error: {error}"));
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|error| format!("terminal error: {error}"))?;

    let result = event_loop(&mut terminal, app, store);
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    store: &Store,
) -> Result<Option<String>, String> {
    loop {
        terminal
            .draw(|frame| render(frame, app))
            .map_err(|error| format!("terminal error: {error}"))?;
        if !event::poll(Duration::from_millis(200))
            .map_err(|error| format!("input error: {error}"))?
        {
            continue;
        }
        let key = match event::read().map_err(|error| format!("input error: {error}"))? {
            TerminalEvent::Key(key) => key.code,
            _ => continue,
        };
        match app.handle_key(key) {
            Action::None => {}
            Action::Quit => return Ok(None),
            Action::Jump(id) => return Ok(Some(id)),
            Action::Read(id, read) => match store.set_read(&id, read) {
                Ok(true) => app.apply_read(&id, read),
                Ok(false) => app.set_message("event no longer exists".to_owned()),
                Err(error) => app.set_message(error),
            },
        }
    }
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
        "j/k navigate - Enter inspect - n next unread - r read - u unread - J jump - q quit"
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
            detail_line(
                "Workspace",
                event.herdr.workspace_id.as_deref().unwrap_or("-"),
            ),
            detail_line("Tab", event.herdr.tab_id.as_deref().unwrap_or("-")),
            detail_line("Pane", event.herdr.pane_id.as_deref().unwrap_or("-")),
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
    use super::{Action, App, View, render};
    use crate::event::{Event, HerdrContext};
    use crossterm::event::KeyCode;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::ffi::OsStr;

    fn events() -> Vec<Event> {
        ["first", "second", "third"]
            .into_iter()
            .map(|name| {
                Event::new(
                    OsStr::new(name),
                    &[],
                    HerdrContext {
                        pane_id: Some("w1:p1".to_owned()),
                        ..HerdrContext::default()
                    },
                )
                .completed(0, None)
            })
            .collect()
    }

    #[test]
    fn navigation_moves_selection() {
        let mut app = App::new(events());
        assert_eq!(app.handle_key(KeyCode::Down), Action::None);
        assert_eq!(app.selected_index(), 1);
        assert_eq!(app.handle_key(KeyCode::Char('k')), Action::None);
        assert_eq!(app.selected_index(), 0);
        assert_eq!(app.handle_key(KeyCode::Up), Action::None);
        assert_eq!(app.selected_index(), 2);
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
            app.handle_key(KeyCode::Char('r')),
            Action::Read(id.clone(), true)
        );
        app.apply_read(&id, true);
        assert_eq!(app.unread_count(), 2);
    }

    #[test]
    fn next_unread_skips_read_events() {
        let mut app = App::new(events());
        let read_id = app.events[1].id.clone();
        app.apply_read(&read_id, true);
        let _ = app.handle_key(KeyCode::Char('n'));
        assert_eq!(app.selected_index(), 2);
        assert_eq!(app.view, View::Detail);
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
        assert!(content.contains("w1:p1"));
        Ok(())
    }
}
