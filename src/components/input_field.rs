use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub struct InputField {
    pub value: String,
    pub label: String,
    pub placeholder: String,
    pub cursor: usize,
    pub focused: bool,
    pub masked: bool,
}

impl InputField {
    pub fn new(label: &str) -> Self {
        Self {
            value: String::new(),
            label: label.to_string(),
            placeholder: String::new(),
            cursor: 0,
            focused: false,
            masked: false,
        }
    }

    pub fn with_placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = placeholder.to_string();
        self
    }

    pub fn with_masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    pub fn handle_event(&mut self, event: &Event) -> bool {
        if !self.focused {
            return false;
        }

        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return false;
            }
            // Ctrl+U clears line (must check before generic Char match)
            if key.code == KeyCode::Char('u') && key.modifiers.contains(KeyModifiers::CONTROL) {
                self.value.clear();
                self.cursor = 0;
                return true;
            }
            match key.code {
                KeyCode::Char(c) => {
                    self.value.insert(self.cursor, c);
                    self.cursor += 1;
                    return true;
                }
                KeyCode::Backspace => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                        self.value.remove(self.cursor);
                        return true;
                    }
                }
                KeyCode::Delete => {
                    if self.cursor < self.value.len() {
                        self.value.remove(self.cursor);
                        return true;
                    }
                }
                KeyCode::Left => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                        return true;
                    }
                }
                KeyCode::Right => {
                    if self.cursor < self.value.len() {
                        self.cursor += 1;
                        return true;
                    }
                }
                KeyCode::Home => {
                    self.cursor = 0;
                    return true;
                }
                KeyCode::End => {
                    self.cursor = self.value.len();
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    pub fn set_value(&mut self, value: &str) {
        self.value = value.to_string();
        self.cursor = self.value.len();
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focused {
            theme::active_border()
        } else {
            theme::border()
        };

        let block = Block::default()
            .title(format!(" {} ", self.label))
            .title_style(if self.focused {
                theme::title()
            } else {
                theme::dim()
            })
            .borders(Borders::ALL)
            .border_style(border_style);

        let display = if self.value.is_empty() {
            Line::from(Span::styled(&self.placeholder, theme::dim()))
        } else if self.masked {
            Line::from(Span::styled("•".repeat(self.value.len()), theme::normal()))
        } else {
            Line::from(Span::styled(&self.value, theme::normal()))
        };

        let paragraph = Paragraph::new(display).block(block);
        frame.render_widget(paragraph, area);

        // Show cursor
        if self.focused {
            frame.set_cursor_position((area.x + 1 + self.cursor as u16, area.y + 1));
        }
    }
}
