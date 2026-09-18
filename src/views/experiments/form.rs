use crate::action::Action;
use crate::api::types::{CreateExperimentRequest, ManagedExperiment, UpdateExperimentRequest};
use crate::components::input_field::InputField;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use serde_json::json;

pub struct ExperimentFormView {
    pub is_edit: bool,
    key_input: InputField,
    name_input: InputField,
    hypothesis_input: InputField,
    description_input: InputField,
    decision_input: InputField,
    decision_notes_input: InputField,
    focused_field: usize,
    pub original_key: Option<String>,
}

impl ExperimentFormView {
    pub fn new_create() -> Self {
        let mut view = Self {
            is_edit: false,
            key_input: InputField::new("Stable key").with_placeholder("checkout-flow"),
            name_input: InputField::new("Name").with_placeholder("Checkout flow"),
            hypothesis_input: InputField::new("Hypothesis")
                .with_placeholder("Treatment improves completion"),
            description_input: InputField::new("Description").with_placeholder("Optional"),
            decision_input: InputField::new("Decision").with_placeholder("Optional"),
            decision_notes_input: InputField::new("Decision notes").with_placeholder("Optional"),
            focused_field: 0,
            original_key: None,
        };
        view.update_focus();
        view
    }

    pub fn new_edit(experiment: &ManagedExperiment) -> Self {
        let mut view = Self::new_create();
        view.is_edit = true;
        view.original_key = Some(experiment.key.clone());
        view.key_input.set_value(&experiment.key);
        view.name_input.set_value(&experiment.name);
        view.hypothesis_input.set_value(&experiment.hypothesis);
        view.description_input.set_value(&experiment.description);
        view.decision_input.set_value(&experiment.decision);
        view.decision_notes_input
            .set_value(&experiment.decision_notes);
        view
    }

    fn field_count(&self) -> usize {
        if self.is_edit {
            6
        } else {
            4
        }
    }

    fn update_focus(&mut self) {
        self.key_input.focused = self.focused_field == 0;
        self.name_input.focused = self.focused_field == 1;
        self.hypothesis_input.focused = self.focused_field == 2;
        self.description_input.focused = self.focused_field == 3;
        self.decision_input.focused = self.focused_field == 4;
        self.decision_notes_input.focused = self.focused_field == 5;
    }

    pub fn create_request(&self) -> CreateExperimentRequest {
        CreateExperimentRequest {
            key: self.key_input.value.clone(),
            name: self.name_input.value.clone(),
            hypothesis: self.hypothesis_input.value.clone(),
            description: self.description_input.value.clone(),
            variants: json!({
                "variants": [
                    {"key": "control", "name": "Control", "weight": 50},
                    {"key": "treatment", "name": "Treatment", "weight": 50}
                ]
            }),
        }
    }

    pub fn update_request(&self) -> UpdateExperimentRequest {
        UpdateExperimentRequest {
            name: Some(self.name_input.value.clone()),
            hypothesis: Some(self.hypothesis_input.value.clone()),
            description: Some(self.description_input.value.clone()),
            decision: Some(self.decision_input.value.clone()),
            decision_notes: Some(self.decision_notes_input.value.clone()),
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
                    self.focused_field = (self.focused_field + 1) % self.field_count();
                    self.update_focus();
                }
                KeyCode::BackTab | KeyCode::Up => {
                    self.focused_field = if self.focused_field == 0 {
                        self.field_count() - 1
                    } else {
                        self.focused_field - 1
                    };
                    self.update_focus();
                }
                KeyCode::Enter => {
                    if self.key_input.value.is_empty() || self.name_input.value.is_empty() {
                        return None;
                    }
                    return if self.is_edit {
                        self.original_key
                            .clone()
                            .map(Action::SubmitExperimentUpdate)
                    } else {
                        Some(Action::SubmitExperimentCreate)
                    };
                }
                _ => match self.focused_field {
                    0 if !self.is_edit => {
                        self.key_input.handle_event(event);
                    }
                    1 => {
                        self.name_input.handle_event(event);
                    }
                    2 => {
                        self.hypothesis_input.handle_event(event);
                    }
                    3 => {
                        self.description_input.handle_event(event);
                    }
                    4 if self.is_edit => {
                        self.decision_input.handle_event(event);
                    }
                    5 if self.is_edit => {
                        self.decision_notes_input.handle_event(event);
                    }
                    _ => {}
                },
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(3),
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
                Span::styled(
                    if self.is_edit {
                        "Edit Experiment"
                    } else {
                        "Create Experiment"
                    },
                    theme::heading(),
                ),
            ])),
            chunks[0],
        );
        self.key_input.render(frame, chunks[1]);
        self.name_input.render(frame, chunks[2]);
        self.hypothesis_input.render(frame, chunks[3]);
        self.description_input.render(frame, chunks[4]);
        if self.is_edit {
            self.decision_input.render(frame, chunks[5]);
            self.decision_notes_input.render(frame, chunks[6]);
        } else {
            frame.render_widget(
                Paragraph::new("Creates a safe 50/50 control/treatment draft.").style(theme::dim()),
                chunks[5],
            );
        }

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("[Enter]", theme::title()),
                Span::styled(if self.is_edit { " Save" } else { " Create" }, theme::dim()),
                Span::raw("   "),
                Span::styled("[Esc]", theme::title()),
                Span::styled(" Cancel", theme::dim()),
            ])),
            chunks[7],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_request_uses_safe_balanced_defaults() {
        let mut form = ExperimentFormView::new_create();
        form.key_input.set_value("checkout-flow");
        form.name_input.set_value("Checkout flow");

        let request = form.create_request();
        let variants = request.variants["variants"].as_array().unwrap();

        assert_eq!(request.key, "checkout-flow");
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0]["key"], "control");
        assert_eq!(variants[0]["weight"], 50);
        assert_eq!(variants[1]["key"], "treatment");
        assert_eq!(variants[1]["weight"], 50);
    }

    #[test]
    fn edit_request_preserves_decision_fields() {
        let experiment: ManagedExperiment = serde_json::from_value(json!({
            "id": "exp_123",
            "project_id": "prj_123",
            "key": "checkout-flow",
            "name": "Checkout flow",
            "hypothesis": "Faster checkout converts",
            "description": "",
            "status": "completed",
            "randomization_unit": "user_id",
            "layer_key": "checkout",
            "variants": {"variants": []},
            "parameters": {"parameters": []},
            "metrics": {"primary": []},
            "decision": "ship",
            "decision_notes": "Treatment won",
            "inserted_at": "2026-08-21T12:00:00Z",
            "updated_at": "2026-08-21T13:00:00Z"
        }))
        .unwrap();

        let request = ExperimentFormView::new_edit(&experiment).update_request();

        assert_eq!(request.decision.as_deref(), Some("ship"));
        assert_eq!(request.decision_notes.as_deref(), Some("Treatment won"));
    }
}
