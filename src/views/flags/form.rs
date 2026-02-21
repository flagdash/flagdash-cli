use crate::action::Action;
use crate::api::types::{CreateFlagRequest, ManagedFlag, UpdateFlagRequest};
use crate::components::input_field::InputField;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

const FLAG_TYPES: &[&str] = &["boolean", "string", "number", "json"];

pub struct FlagFormView {
    pub is_edit: bool,
    pub project_id: String,
    key_input: InputField,
    name_input: InputField,
    description_input: InputField,
    flag_type_index: usize,
    focused_field: usize,
    pub original_key: Option<String>,
}

impl FlagFormView {
    pub fn new_create(project_id: &str) -> Self {
        Self {
            is_edit: false,
            project_id: project_id.to_string(),
            key_input: InputField::new("Key").with_placeholder("my-flag"),
            name_input: InputField::new("Name").with_placeholder("My Feature Flag"),
            description_input: InputField::new("Description")
                .with_placeholder("Optional description"),
            flag_type_index: 0,
            focused_field: 0,
            original_key: None,
        }
    }

    pub fn new_edit(project_id: &str, flag: &ManagedFlag) -> Self {
        let mut view = Self::new_create(project_id);
        view.is_edit = true;
        view.original_key = Some(flag.key.clone());
        view.key_input.set_value(&flag.key);
        view.name_input.set_value(&flag.name);
        view.description_input.set_value(&flag.description);
        view.flag_type_index = FLAG_TYPES
            .iter()
            .position(|t| *t == flag.flag_type)
            .unwrap_or(0);
        view
    }

    fn update_focus(&mut self) {
        self.key_input.focused = self.focused_field == 0;
        self.name_input.focused = self.focused_field == 1;
        self.description_input.focused = self.focused_field == 2;
    }

    pub fn create_request(&self) -> CreateFlagRequest {
        CreateFlagRequest {
            project_id: self.project_id.clone(),
            key: self.key_input.value.clone(),
            name: self.name_input.value.clone(),
            description: self.description_input.value.clone(),
            flag_type: FLAG_TYPES[self.flag_type_index].to_string(),
            tags: Vec::new(),
            default_value: None,
        }
    }

    pub fn update_request(&self) -> UpdateFlagRequest {
        UpdateFlagRequest {
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
                    return None;
                }
                KeyCode::BackTab | KeyCode::Up => {
                    self.focused_field = if self.focused_field == 0 {
                        3
                    } else {
                        self.focused_field - 1
                    };
                    self.update_focus();
                    return None;
                }
                KeyCode::Left if self.focused_field == 3 => {
                    if self.flag_type_index > 0 {
                        self.flag_type_index -= 1;
                    }
                    return None;
                }
                KeyCode::Right if self.focused_field == 3 => {
                    if self.flag_type_index < FLAG_TYPES.len() - 1 {
                        self.flag_type_index += 1;
                    }
                    return None;
                }
                KeyCode::Enter if self.focused_field == 3 => {
                    // Submit
                    if self.key_input.value.is_empty() || self.name_input.value.is_empty() {
                        return None;
                    }
                    if self.is_edit {
                        if let Some(key) = &self.original_key {
                            return Some(Action::SubmitFlagUpdate(key.clone()));
                        }
                    } else {
                        return Some(Action::SubmitFlagCreate);
                    }
                }
                _ => {
                    // Delegate to focused input
                    match self.focused_field {
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
                    }
                }
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let title_text = if self.is_edit {
            "Edit Flag"
        } else {
            "Create Flag"
        };

        let chunks = Layout::vertical([
            Constraint::Length(2), // Title
            Constraint::Length(3), // Key
            Constraint::Length(3), // Name
            Constraint::Length(3), // Description
            Constraint::Length(3), // Type selector
            Constraint::Length(2), // Submit hint
            Constraint::Min(0),
        ])
        .split(area);

        let title = Paragraph::new(Line::from(vec![
            Span::styled("← ", theme::dim()),
            Span::styled(title_text, theme::heading()),
        ]));
        frame.render_widget(title, chunks[0]);

        self.key_input.render(frame, chunks[1]);
        self.name_input.render(frame, chunks[2]);
        self.description_input.render(frame, chunks[3]);

        // Type selector
        let type_spans: Vec<Span> = FLAG_TYPES
            .iter()
            .enumerate()
            .flat_map(|(i, t)| {
                let style = if i == self.flag_type_index {
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
        let type_line = Paragraph::new(Line::from(
            [vec![Span::styled("Type: ", type_label)], type_spans].concat(),
        ));
        frame.render_widget(type_line, chunks[4]);

        // Submit hint
        let hint = Paragraph::new(Line::from(vec![
            Span::styled("[Enter]", theme::title()),
            Span::styled(if self.is_edit { " Save" } else { " Create" }, theme::dim()),
            Span::raw("   "),
            Span::styled("[Esc]", theme::title()),
            Span::styled(" Cancel", theme::dim()),
        ]));
        frame.render_widget(hint, chunks[5]);
    }
}
