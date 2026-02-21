use crate::action::{Action, View};
use crate::api::types::{Environment, Schedule};
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;

pub struct FlagSchedulesView {
    pub flag_key: String,
    pub environments: Vec<Environment>,
    pub selected_env: usize,
    pub schedules: Vec<Schedule>,
    state: TableState,
}

impl FlagSchedulesView {
    pub fn new(flag_key: &str) -> Self {
        Self {
            flag_key: flag_key.to_string(),
            environments: Vec::new(),
            selected_env: 0,
            schedules: Vec::new(),
            state: TableState::default(),
        }
    }

    pub fn set_schedules(&mut self, schedules: Vec<Schedule>) {
        self.schedules = schedules;
        if !self.schedules.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn selected_environment_id(&self) -> Option<&str> {
        self.environments
            .get(self.selected_env)
            .map(|e| e.id.as_str())
    }

    pub fn selected_schedule(&self) -> Option<&Schedule> {
        self.state.selected().and_then(|i| self.schedules.get(i))
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Backspace => {
                    return Some(Action::Navigate(View::FlagDetail(self.flag_key.clone())));
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.schedules.is_empty() {
                        let i = self.state.selected().unwrap_or(0);
                        self.state.select(Some((i + 1) % self.schedules.len()));
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if !self.schedules.is_empty() {
                        let i = self.state.selected().unwrap_or(0);
                        let new = if i == 0 {
                            self.schedules.len() - 1
                        } else {
                            i - 1
                        };
                        self.state.select(Some(new));
                    }
                }
                KeyCode::Tab => {
                    if !self.environments.is_empty() {
                        self.selected_env = (self.selected_env + 1) % self.environments.len();
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);

        let title = Paragraph::new(Line::from(vec![
            Span::styled("← ", theme::dim()),
            Span::styled(format!("Schedules: {}", self.flag_key), theme::heading()),
        ]));
        frame.render_widget(title, chunks[0]);

        let env_name = self
            .environments
            .get(self.selected_env)
            .map(|e| e.name.as_str())
            .unwrap_or("(none)");
        let env_line = Paragraph::new(Line::from(vec![
            Span::styled("Environment: ", theme::dim()),
            Span::styled(env_name, theme::normal()),
            Span::styled("  [Tab] to switch", theme::dim()),
        ]));
        frame.render_widget(env_line, chunks[1]);

        let rows: Vec<Row> = self
            .schedules
            .iter()
            .map(|s| {
                let status_style = match s.status.as_str() {
                    "pending" => theme::dim(),
                    "executed" => theme::status_on(),
                    "cancelled" => theme::status_off(),
                    "failed" => theme::status_off(),
                    _ => theme::dim(),
                };
                Row::new(vec![
                    Cell::from(s.action.as_str()).style(theme::normal()),
                    Cell::from(s.scheduled_at.format("%Y-%m-%d %H:%M").to_string())
                        .style(theme::normal()),
                    Cell::from(s.status.as_str()).style(status_style),
                    Cell::from(
                        s.executed_at
                            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_else(|| "-".to_string()),
                    )
                    .style(theme::dim()),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(20),
                Constraint::Percentage(30),
                Constraint::Percentage(20),
                Constraint::Percentage(30),
            ],
        )
        .header(
            Row::new(vec!["Action", "Scheduled At", "Status", "Executed At"])
                .style(theme::heading()),
        )
        .block(
            Block::default()
                .title(format!(" Schedules ({}) ", self.schedules.len()))
                .title_style(theme::heading())
                .borders(Borders::ALL)
                .border_style(theme::border()),
        )
        .highlight_style(theme::highlight());

        frame.render_stateful_widget(table, chunks[2], &mut self.state);

        let hint = Paragraph::new(Line::from(vec![
            Span::styled("[Esc]", theme::title()),
            Span::styled(" Back", theme::dim()),
        ]));
        frame.render_widget(hint, chunks[3]);
    }
}
