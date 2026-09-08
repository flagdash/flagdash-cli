use crate::action::Action;
use crate::api::types::{CreateAiConfigRequest, ManagedAiConfig, UpdateAiConfigRequest};
use crate::components::input_field::InputField;
use crate::components::text_area::TextArea;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

const FILE_TYPES: &[&str] = &["skill", "rule", "agent"];

pub struct AiConfigFormView {
    pub is_edit: bool,
    pub project_id: String,
    pub environment_id: String,
    file_name_input: InputField,
    folder_input: InputField,
    content_editor: TextArea,
    file_type_index: usize,
    focused_field: usize, // 0=filename, 1=folder, 2=type, 3=content
    pub original_file_name: Option<String>,
}

impl AiConfigFormView {
    pub fn new_create(project_id: &str, environment_id: &str) -> Self {
        Self {
            is_edit: false,
            project_id: project_id.to_string(),
            environment_id: environment_id.to_string(),
            file_name_input: InputField::new("File Name").with_placeholder("my-skill.md"),
            folder_input: InputField::new("Folder").with_placeholder("optional"),
            content_editor: TextArea::new("Content (Markdown)"),
            file_type_index: 0,
            focused_field: 0,
            original_file_name: None,
        }
    }

    pub fn new_edit(project_id: &str, environment_id: &str, config: &ManagedAiConfig) -> Self {
        let mut view = Self::new_create(project_id, environment_id);
        view.is_edit = true;
        view.original_file_name = Some(config.file_name.clone());
        view.file_name_input.set_value(&config.file_name);
        view.folder_input.set_value(&config.folder);
        view.content_editor.set_content(&config.content);
        view.file_type_index = FILE_TYPES
            .iter()
            .position(|t| *t == config.file_type)
            .unwrap_or(0);
        view
    }

    fn update_focus(&mut self) {
        self.file_name_input.focused = self.focused_field == 0;
        self.folder_input.focused = self.focused_field == 1;
        self.content_editor.focused = self.focused_field == 3;
    }

    pub fn create_request(&self) -> CreateAiConfigRequest {
        CreateAiConfigRequest {
            project_id: self.project_id.clone(),
            environment_id: self.environment_id.clone(),
            file_name: self.file_name_input.value.clone(),
            file_type: FILE_TYPES[self.file_type_index].to_string(),
            content: self.content_editor.content(),
            folder: self.folder_input.value.clone(),
            is_active: Some(true),
            metadata: None,
        }
    }

    pub fn update_request(&self) -> UpdateAiConfigRequest {
        UpdateAiConfigRequest {
            content: Some(self.content_editor.content()),
            is_active: None,
            metadata: None,
            folder: Some(self.folder_input.value.clone()),
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }

            // Ctrl+S saves from any field
            if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
                if self.file_name_input.value.is_empty() {
                    return None;
                }
                if self.is_edit {
                    if let Some(name) = &self.original_file_name {
                        return Some(Action::SubmitAiConfigUpdate(name.clone()));
                    }
                } else {
                    return Some(Action::SubmitAiConfigCreate);
                }
                return None;
            }

            // Esc escapes content editor to field navigation, or cancels form
            if key.code == KeyCode::Esc {
                if self.focused_field == 3 {
                    self.focused_field = 0;
                    self.update_focus();
                    return None;
                }
                return Some(Action::Back);
            }

            // Tab cycles fields when not in content editor
            if key.code == KeyCode::Tab && self.focused_field != 3 {
                self.focused_field = (self.focused_field + 1) % 4;
                self.update_focus();
                return None;
            }

            // Type selector arrows
            if self.focused_field == 2 {
                match key.code {
                    KeyCode::Left => {
                        if self.file_type_index > 0 {
                            self.file_type_index -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if self.file_type_index < FILE_TYPES.len() - 1 {
                            self.file_type_index += 1;
                        }
                    }
                    KeyCode::Tab => {
                        self.focused_field = 3;
                        self.update_focus();
                    }
                    _ => {}
                }
                return None;
            }

            // Delegate to focused widget
            match self.focused_field {
                0 if !self.is_edit => {
                    self.file_name_input.handle_event(event);
                }
                1 => {
                    self.folder_input.handle_event(event);
                }
                3 => {
                    self.content_editor.handle_event(event);
                }
                _ => {}
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let title_text = if self.is_edit {
            "Edit AI Config"
        } else {
            "Create AI Config"
        };
        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("← ", theme::dim()),
                Span::styled(title_text, theme::heading()),
            ])),
            chunks[0],
        );

        self.file_name_input.render(frame, chunks[1]);
        self.folder_input.render(frame, chunks[2]);

        // Type selector
        let type_spans: Vec<Span> = FILE_TYPES
            .iter()
            .enumerate()
            .flat_map(|(i, t)| {
                let style = if i == self.file_type_index {
                    theme::highlight()
                } else {
                    theme::dim()
                };
                vec![Span::styled(format!(" {} ", t), style), Span::raw(" ")]
            })
            .collect();
        let type_label = if self.focused_field == 2 {
            theme::title()
        } else {
            theme::dim()
        };
        frame.render_widget(
            Paragraph::new(Line::from(
                [vec![Span::styled("Type: ", type_label)], type_spans].concat(),
            )),
            chunks[3],
        );

        self.content_editor.render(frame, chunks[4]);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("[Ctrl+S]", theme::title()),
                Span::styled(" Save  ", theme::dim()),
                Span::styled("[Tab]", theme::title()),
                Span::styled(" Next field  ", theme::dim()),
                Span::styled("[Esc]", theme::title()),
                Span::styled(" Back", theme::dim()),
            ])),
            chunks[5],
        );
    }
}
