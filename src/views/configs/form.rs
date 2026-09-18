use crate::action::Action;
use crate::api::types::{CreateConfigRequest, ManagedConfig, UpdateConfigRequest};
use crate::components::input_field::InputField;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

const CONFIG_TYPES: &[&str] = &["string", "number", "boolean", "json"];

pub struct ConfigFormView {
    pub is_edit: bool,
    pub project_id: String,
    key_input: InputField,
    name_input: InputField,
    description_input: InputField,
    config_type_index: usize,
    focused_field: usize,
    pub original_key: Option<String>,
}

impl ConfigFormView {
    pub fn new_create(project_id: &str) -> Self {
        Self {
            is_edit: false,
            project_id: project_id.to_string(),
            key_input: InputField::new("Key").with_placeholder("my-config"),
            name_input: InputField::new("Name").with_placeholder("My Config"),
            description_input: InputField::new("Description").with_placeholder("Optional"),
            config_type_index: 0,
            focused_field: 0,
            original_key: None,
        }
    }

    pub fn new_edit(project_id: &str, config: &ManagedConfig) -> Self {
        let mut view = Self::new_create(project_id);
        view.is_edit = true;
        view.original_key = Some(config.key.clone());
        view.key_input.set_value(&config.key);
        view.name_input.set_value(&config.name);
        view.description_input.set_value(&config.description);
        view.config_type_index = CONFIG_TYPES
            .iter()
            .position(|t| *t == config.config_type)
            .unwrap_or(0);
        view
    }

    fn update_focus(&mut self) {
        self.key_input.focused = self.focused_field == 0;
        self.name_input.focused = self.focused_field == 1;
        self.description_input.focused = self.focused_field == 2;
    }

    pub fn create_request(&self) -> CreateConfigRequest {
        CreateConfigRequest {
            project_id: self.project_id.clone(),
            key: self.key_input.value.clone(),
            name: self.name_input.value.clone(),
            description: self.description_input.value.clone(),
            config_type: CONFIG_TYPES[self.config_type_index].to_string(),
            default_value: None,
            tags: Vec::new(),
        }
    }

    pub fn update_request(&self) -> UpdateConfigRequest {
        UpdateConfigRequest {
            name: Some(self.name_input.value.clone()),
            description: Some(self.description_input.value.clone()),
            tags: None,
            default_value: None,
            is_archived: None,
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }
            match key.code {
                KeyCode::Esc => return Some(Action::Back),
                KeyCode::Tab | KeyCode::Down => {
                    self.focused_field = (self.focused_field + 1) % 4;
                    self.update_focus();
                }
                KeyCode::BackTab | KeyCode::Up => {
                    self.focused_field = if self.focused_field == 0 {
                        3
                    } else {
                        self.focused_field - 1
                    };
                    self.update_focus();
                }
                KeyCode::Left if self.focused_field == 3 => {
                    if self.config_type_index > 0 {
                        self.config_type_index -= 1;
                    }
                }
                KeyCode::Right if self.focused_field == 3 => {
                    if self.config_type_index < CONFIG_TYPES.len() - 1 {
                        self.config_type_index += 1;
                    }
                }
                KeyCode::Enter if self.focused_field == 3 => {
                    if self.key_input.value.is_empty() || self.name_input.value.is_empty() {
                        return None;
                    }
                    if self.is_edit {
                        if let Some(key) = &self.original_key {
                            return Some(Action::SubmitConfigUpdate(key.clone()));
                        }
                    } else {
                        return Some(Action::SubmitConfigCreate);
                    }
                }
                _ => match self.focused_field {
                    0 if !self.is_edit => {
                        self.key_input.handle_event(event);
                    }
                    1 => {
                        self.name_input.handle_event(event);
                    }
                    2 => {
                        self.description_input.handle_event(event);
                    }
                    _ => {}
                },
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let title_text = if self.is_edit {
            "Edit Config"
        } else {
            "Create Config"
        };
        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("← ", theme::dim()),
                Span::styled(title_text, theme::heading()),
            ])),
            chunks[0],
        );

        self.key_input.render(frame, chunks[1]);
        self.name_input.render(frame, chunks[2]);
        self.description_input.render(frame, chunks[3]);

        let type_spans: Vec<Span> = CONFIG_TYPES
            .iter()
            .enumerate()
            .flat_map(|(i, t)| {
                let style = if i == self.config_type_index {
                    theme::highlight()
                } else {
                    theme::dim()
                };
                vec![Span::styled(format!(" {} ", t), style), Span::raw(" ")]
            })
            .collect();
        let type_label = if self.focused_field == 3 {
            theme::title()
        } else {
            theme::dim()
        };
        frame.render_widget(
            Paragraph::new(Line::from(
                [vec![Span::styled("Type: ", type_label)], type_spans].concat(),
            )),
            chunks[4],
        );

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("[Enter]", theme::title()),
                Span::styled(if self.is_edit { " Save" } else { " Create" }, theme::dim()),
                Span::raw("   "),
                Span::styled("[Esc]", theme::title()),
                Span::styled(" Cancel", theme::dim()),
            ])),
            chunks[5],
        );
    }
}
