use crate::action::{Action, View};
use crate::api::types::Environment;
use crate::components::text_area::TextArea;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct ConfigValueEditorView {
    pub config_key: String,
    pub environments: Vec<Environment>,
    pub selected_env: usize,
    pub editor: TextArea,
}

impl ConfigValueEditorView {
    pub fn new(config_key: &str) -> Self {
        let mut editor = TextArea::new("Value (JSON)");
        editor.focused = true;
        Self {
            config_key: config_key.to_string(),
            environments: Vec::new(),
            selected_env: 0,
            editor,
        }
    }

    pub fn set_value(&mut self, value: &serde_json::Value) {
        let formatted = serde_json::to_string_pretty(value).unwrap_or_default();
        self.editor.set_content(&formatted);
    }

    pub fn selected_environment_id(&self) -> Option<&str> {
        self.environments
            .get(self.selected_env)
            .map(|e| e.id.as_str())
    }

    pub fn parse_value(&self) -> Result<serde_json::Value, String> {
        serde_json::from_str(&self.editor.content()).map_err(|e| e.to_string())
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }
            match key.code {
                KeyCode::Esc => {
                    return Some(Action::Navigate(View::ConfigDetail(
                        self.config_key.clone(),
                    )));
                }
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Some(Action::SubmitConfigValueUpdate(self.config_key.clone()));
                }
                KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    if !self.environments.is_empty() {
                        self.selected_env = (self.selected_env + 1) % self.environments.len();
                    }
                }
                _ => {
                    self.editor.handle_event(event);
                }
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("← ", theme::dim()),
                Span::styled(format!("Set Value: {}", self.config_key), theme::heading()),
            ])),
            chunks[0],
        );

        let env_name = self
            .environments
            .get(self.selected_env)
            .map(|e| e.name.as_str())
            .unwrap_or("(none)");
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("Environment: ", theme::dim()),
                Span::styled(env_name, theme::normal()),
                Span::styled("  [Shift+Tab] to switch", theme::dim()),
            ])),
            chunks[1],
        );

        self.editor.render(frame, chunks[2]);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("[Ctrl+S]", theme::title()),
                Span::styled(" Save  ", theme::dim()),
                Span::styled("[Esc]", theme::title()),
                Span::styled(" Back", theme::dim()),
            ])),
            chunks[3],
        );
    }
}
