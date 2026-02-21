use crate::action::Action;
use crate::api::types::{CreateWebhookRequest, UpdateWebhookRequest, WebhookEndpoint};
use crate::components::input_field::InputField;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct WebhookFormView {
    pub is_edit: bool,
    pub project_id: String,
    pub environment_id: String,
    url_input: InputField,
    description_input: InputField,
    events_input: InputField,
    focused_field: usize,
    pub original_id: Option<String>,
}

impl WebhookFormView {
    pub fn new_create(project_id: &str, environment_id: &str) -> Self {
        Self {
            is_edit: false,
            project_id: project_id.to_string(),
            environment_id: environment_id.to_string(),
            url_input: InputField::new("URL").with_placeholder("https://example.com/webhook"),
            description_input: InputField::new("Description").with_placeholder("Optional"),
            events_input: InputField::new("Event Types")
                .with_placeholder("flag.updated, config.updated"),
            focused_field: 0,
            original_id: None,
        }
    }

    pub fn new_edit(project_id: &str, environment_id: &str, webhook: &WebhookEndpoint) -> Self {
        let mut view = Self::new_create(project_id, environment_id);
        view.is_edit = true;
        view.original_id = Some(webhook.id.clone());
        view.url_input.set_value(&webhook.url);
        view.description_input.set_value(&webhook.description);
        view.events_input.set_value(&webhook.event_types.join(", "));
        view
    }

    fn update_focus(&mut self) {
        self.url_input.focused = self.focused_field == 0;
        self.description_input.focused = self.focused_field == 1;
        self.events_input.focused = self.focused_field == 2;
    }

    pub fn create_request(&self) -> CreateWebhookRequest {
        CreateWebhookRequest {
            project_id: self.project_id.clone(),
            environment_id: self.environment_id.clone(),
            url: self.url_input.value.clone(),
            description: self.description_input.value.clone(),
            event_types: parse_events(&self.events_input.value),
        }
    }

    pub fn update_request(&self) -> UpdateWebhookRequest {
        UpdateWebhookRequest {
            url: Some(self.url_input.value.clone()),
            description: Some(self.description_input.value.clone()),
            event_types: Some(parse_events(&self.events_input.value)),
            is_active: None,
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
                    self.focused_field = (self.focused_field + 1) % 3;
                    self.update_focus();
                }
                KeyCode::BackTab | KeyCode::Up => {
                    self.focused_field = if self.focused_field == 0 {
                        2
                    } else {
                        self.focused_field - 1
                    };
                    self.update_focus();
                }
                KeyCode::Enter => {
                    if self.url_input.value.is_empty() {
                        return None;
                    }
                    if self.is_edit {
                        if let Some(id) = &self.original_id {
                            return Some(Action::SubmitWebhookUpdate(id.clone()));
                        }
                    } else {
                        return Some(Action::SubmitWebhookCreate);
                    }
                }
                _ => match self.focused_field {
                    0 => {
                        self.url_input.handle_event(event);
                    }
                    1 => {
                        self.description_input.handle_event(event);
                    }
                    2 => {
                        self.events_input.handle_event(event);
                    }
                    _ => {}
                },
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let title_text = if self.is_edit {
            "Edit Webhook"
        } else {
            "Create Webhook"
        };
        let chunks = Layout::vertical([
            Constraint::Length(2),
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

        self.url_input.render(frame, chunks[1]);
        self.description_input.render(frame, chunks[2]);
        self.events_input.render(frame, chunks[3]);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("[Enter]", theme::title()),
                Span::styled(if self.is_edit { " Save" } else { " Create" }, theme::dim()),
                Span::raw("   "),
                Span::styled("[Esc]", theme::title()),
                Span::styled(" Cancel", theme::dim()),
            ])),
            chunks[4],
        );
    }
}

fn parse_events(s: &str) -> Vec<String> {
    s.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}
