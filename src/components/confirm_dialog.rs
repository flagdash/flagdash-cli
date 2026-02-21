use crate::action::{Action, ConfirmAction};
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub struct ConfirmDialog {
    pub action: Option<ConfirmAction>,
    selected_yes: bool,
}

impl ConfirmDialog {
    pub fn new() -> Self {
        Self {
            action: None,
            selected_yes: false,
        }
    }

    pub fn show(&mut self, action: ConfirmAction) {
        self.action = Some(action);
        self.selected_yes = false;
    }

    pub fn is_visible(&self) -> bool {
        self.action.is_some()
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        self.action.as_ref()?;

        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }
            match key.code {
                KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                    self.selected_yes = !self.selected_yes;
                }
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.action = None;
                    return Some(Action::ConfirmAccepted);
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.action = None;
                    return Some(Action::ConfirmDismissed);
                }
                KeyCode::Enter => {
                    if self.selected_yes {
                        self.action = None;
                        return Some(Action::ConfirmAccepted);
                    } else {
                        self.action = None;
                        return Some(Action::ConfirmDismissed);
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let action = match &self.action {
            Some(a) => a,
            None => return,
        };

        let message = match action {
            ConfirmAction::DeleteFlag(key) => format!("Delete flag '{}'?", key),
            ConfirmAction::DeleteConfig(key) => format!("Delete config '{}'?", key),
            ConfirmAction::DeleteAiConfig(name) => format!("Delete AI config '{}'?", name),
            ConfirmAction::DeleteWebhook(id) => format!("Delete webhook '{}'?", id),
            ConfirmAction::CancelSchedule { schedule_id, .. } => {
                format!("Cancel schedule '{}'?", schedule_id)
            }
            ConfirmAction::DeleteVariations(key) => {
                format!("Delete all variations for '{}'?", key)
            }
        };

        // Center dialog
        let width = 50u16.min(area.width - 4);
        let height = 7u16;
        let dialog_area = centered_rect(width, height, area);

        let block = Block::default()
            .title(" Confirm ")
            .title_style(theme::heading())
            .borders(Borders::ALL)
            .border_style(theme::active_border());

        frame.render_widget(Clear, dialog_area);
        frame.render_widget(block, dialog_area);

        let inner = Rect {
            x: dialog_area.x + 2,
            y: dialog_area.y + 1,
            width: dialog_area.width.saturating_sub(4),
            height: dialog_area.height.saturating_sub(2),
        };

        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

        // Message
        let msg = Paragraph::new(message).alignment(Alignment::Center);
        frame.render_widget(msg, chunks[0]);

        // Buttons
        let yes_style = if self.selected_yes {
            theme::highlight()
        } else {
            theme::dim()
        };
        let no_style = if self.selected_yes {
            theme::dim()
        } else {
            theme::highlight()
        };

        let buttons = Paragraph::new(Line::from(vec![
            Span::raw("      "),
            Span::styled(" [Y]es ", yes_style),
            Span::raw("    "),
            Span::styled(" [N]o ", no_style),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(buttons, chunks[2]);
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
