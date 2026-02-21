use crate::action::{Action, View};
use crate::api::types::Environment;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use ratatui::Frame;

pub struct FlagRolloutView {
    pub flag_key: String,
    pub environments: Vec<Environment>,
    pub selected_env: usize,
    pub percentage: i32,
}

impl FlagRolloutView {
    pub fn new(flag_key: &str) -> Self {
        Self {
            flag_key: flag_key.to_string(),
            environments: Vec::new(),
            selected_env: 0,
            percentage: 0,
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
                KeyCode::Left => {
                    self.percentage = (self.percentage - 5).max(0);
                }
                KeyCode::Right => {
                    self.percentage = (self.percentage + 5).min(100);
                }
                KeyCode::Down => {
                    self.percentage = (self.percentage - 1).max(0);
                }
                KeyCode::Up => {
                    self.percentage = (self.percentage + 1).min(100);
                }
                KeyCode::Char('0') => self.percentage = 0,
                KeyCode::Char('5') => self.percentage = 50,
                KeyCode::Char('9') => self.percentage = 100,
                KeyCode::Tab => {
                    if !self.environments.is_empty() {
                        self.selected_env = (self.selected_env + 1) % self.environments.len();
                    }
                }
                KeyCode::Enter => {
                    if !self.environments.is_empty() {
                        return Some(Action::SubmitRolloutUpdate(self.flag_key.clone()));
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(area);

        let title = Paragraph::new(Line::from(vec![
            Span::styled("← ", theme::dim()),
            Span::styled(format!("Rollout: {}", self.flag_key), theme::heading()),
        ]));
        frame.render_widget(title, chunks[0]);

        // Environment selector
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

        // Gauge
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title(format!(" {}% ", self.percentage))
                    .title_style(theme::heading())
                    .borders(Borders::ALL)
                    .border_style(theme::border()),
            )
            .gauge_style(ratatui::style::Style::default().fg(theme::PRIMARY))
            .ratio(self.percentage as f64 / 100.0);
        frame.render_widget(gauge, chunks[2]);

        // Hints
        let hints = Paragraph::new(Line::from(vec![
            Span::styled("←→", theme::title()),
            Span::styled(" ±5%  ", theme::dim()),
            Span::styled("↑↓", theme::title()),
            Span::styled(" ±1%  ", theme::dim()),
            Span::styled("0", theme::title()),
            Span::styled("/", theme::dim()),
            Span::styled("5", theme::title()),
            Span::styled("/", theme::dim()),
            Span::styled("9", theme::title()),
            Span::styled(" 0/50/100%  ", theme::dim()),
            Span::styled("[Enter]", theme::title()),
            Span::styled(" Save  ", theme::dim()),
            Span::styled("[Esc]", theme::title()),
            Span::styled(" Back", theme::dim()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(hints, chunks[3]);
    }
}
