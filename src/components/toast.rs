use crate::action::ToastLevel;
use crate::theme;
use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use std::time::Instant;

const TOAST_DURATION_SECS: u64 = 3;

pub struct Toast {
    message: Option<ToastState>,
}

struct ToastState {
    text: String,
    level: ToastLevel,
    shown_at: Instant,
}

impl Toast {
    pub fn new() -> Self {
        Self { message: None }
    }

    pub fn show(&mut self, text: String, level: ToastLevel) {
        self.message = Some(ToastState {
            text,
            level,
            shown_at: Instant::now(),
        });
    }

    /// Returns true if there is an active toast to display.
    pub fn is_visible(&self) -> bool {
        if let Some(state) = &self.message {
            state.shown_at.elapsed().as_secs() < TOAST_DURATION_SECS
        } else {
            false
        }
    }

    /// Dismiss expired toasts.
    pub fn tick(&mut self) {
        if let Some(state) = &self.message {
            if state.shown_at.elapsed().as_secs() >= TOAST_DURATION_SECS {
                self.message = None;
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let state = match &self.message {
            Some(s) if s.shown_at.elapsed().as_secs() < TOAST_DURATION_SECS => s,
            _ => return,
        };

        // Position toast at top-right
        let width = (state.text.len() as u16 + 6).min(area.width);
        let toast_area = Rect {
            x: area.x + area.width.saturating_sub(width + 2),
            y: area.y + 1,
            width,
            height: 3,
        };

        let (icon, border_style) = match state.level {
            ToastLevel::Success => ("✓ ", theme::status_on()),
            ToastLevel::Error => ("✗ ", theme::status_off()),
            ToastLevel::Info => ("ℹ ", theme::dim()),
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        let text = Paragraph::new(Line::from(vec![
            Span::styled(icon, border_style),
            Span::styled(&state.text, theme::normal()),
        ]))
        .alignment(Alignment::Center)
        .block(block);

        frame.render_widget(Clear, toast_area);
        frame.render_widget(text, toast_area);
    }
}
