use crate::action::{Action, View};
use crate::api::types::{Environment, ManagedFlag};
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;

pub struct FlagToggleView {
    pub flag: Option<ManagedFlag>,
    pub environments: Vec<Environment>,
    pub flag_key: String,
    state: TableState,
}

impl FlagToggleView {
    pub fn new(flag_key: &str) -> Self {
        let mut state = TableState::default();
        state.select(Some(0));
        Self {
            flag: None,
            environments: Vec::new(),
            flag_key: flag_key.to_string(),
            state,
        }
    }

    pub fn selected_environment_id(&self) -> Option<&str> {
        self.state
            .selected()
            .and_then(|i| self.environments.get(i))
            .map(|e| e.id.as_str())
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
                    let i = self.state.selected().unwrap_or(0);
                    let new = (i + 1) % self.environments.len().max(1);
                    self.state.select(Some(new));
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    let i = self.state.selected().unwrap_or(0);
                    let new = if i == 0 {
                        self.environments.len().saturating_sub(1)
                    } else {
                        i - 1
                    };
                    self.state.select(Some(new));
                }
                KeyCode::Enter | KeyCode::Char('t') => {
                    if !self.environments.is_empty() {
                        return Some(Action::SubmitFlagToggle(self.flag_key.clone()));
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
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

        let title = Paragraph::new(Line::from(vec![
            Span::styled("← ", theme::dim()),
            Span::styled(format!("Toggle: {}", self.flag_key), theme::heading()),
        ]));
        frame.render_widget(title, chunks[0]);

        let flag = &self.flag;
        let rows: Vec<Row> = self
            .environments
            .iter()
            .map(|env| {
                let is_enabled = flag
                    .as_ref()
                    .and_then(|f| f.environments.iter().find(|e| e.environment_id == env.id))
                    .map(|e| e.enabled)
                    .unwrap_or(false);

                let status = if is_enabled { "ON" } else { "OFF" };
                let style = if is_enabled {
                    theme::status_on()
                } else {
                    theme::status_off()
                };

                Row::new(vec![
                    Cell::from(env.name.as_str()).style(theme::normal()),
                    Cell::from(status).style(style),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [Constraint::Percentage(60), Constraint::Percentage(40)],
        )
        .header(Row::new(vec!["Environment", "Status"]).style(theme::heading()))
        .block(
            Block::default()
                .title(" Select environment to toggle ")
                .title_style(theme::heading())
                .borders(Borders::ALL)
                .border_style(theme::border()),
        )
        .highlight_style(theme::highlight());

        frame.render_stateful_widget(table, chunks[1], &mut self.state);

        let hint = Paragraph::new(Line::from(vec![
            Span::styled("[Enter]", theme::title()),
            Span::styled(" Toggle  ", theme::dim()),
            Span::styled("[Esc]", theme::title()),
            Span::styled(" Back", theme::dim()),
        ]));
        frame.render_widget(hint, chunks[2]);
    }
}
