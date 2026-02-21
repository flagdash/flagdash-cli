use crate::action::{Action, View};
use crate::api::types::{Environment, Variation};
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;

pub struct FlagVariationsView {
    pub flag_key: String,
    pub environments: Vec<Environment>,
    pub selected_env: usize,
    pub variations: Vec<Variation>,
    state: TableState,
}

impl FlagVariationsView {
    pub fn new(flag_key: &str) -> Self {
        Self {
            flag_key: flag_key.to_string(),
            environments: Vec::new(),
            selected_env: 0,
            variations: Vec::new(),
            state: TableState::default(),
        }
    }

    pub fn set_variations(&mut self, variations: Vec<Variation>) {
        self.variations = variations;
        if !self.variations.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn selected_environment_id(&self) -> Option<&str> {
        self.environments
            .get(self.selected_env)
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
                    if !self.variations.is_empty() {
                        let i = self.state.selected().unwrap_or(0);
                        self.state.select(Some((i + 1) % self.variations.len()));
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if !self.variations.is_empty() {
                        let i = self.state.selected().unwrap_or(0);
                        let new = if i == 0 {
                            self.variations.len() - 1
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
            Span::styled(format!("Variations: {}", self.flag_key), theme::heading()),
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

        // Variations table
        let rows: Vec<Row> = self
            .variations
            .iter()
            .map(|v| {
                Row::new(vec![
                    Cell::from(v.key.as_str()).style(theme::normal()),
                    Cell::from(v.name.as_str()).style(theme::normal()),
                    Cell::from(format!("{}", v.value)).style(theme::dim()),
                    Cell::from(format!("{}%", v.weight)).style(theme::normal()),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(20),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
                Constraint::Percentage(20),
            ],
        )
        .header(Row::new(vec!["Key", "Name", "Value", "Weight"]).style(theme::heading()))
        .block(
            Block::default()
                .title(format!(" Variations ({}) ", self.variations.len()))
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
