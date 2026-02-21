use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct SearchBar {
    pub query: String,
    pub active: bool,
    cursor: usize,
}

impl SearchBar {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            active: false,
            cursor: 0,
        }
    }

    pub fn activate(&mut self) {
        self.active = true;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.query.clear();
        self.cursor = 0;
    }

    pub fn handle_event(&mut self, event: &Event) -> bool {
        if !self.active {
            return false;
        }

        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return false;
            }
            match key.code {
                KeyCode::Char(c) => {
                    self.query.insert(self.cursor, c);
                    self.cursor += 1;
                    return true;
                }
                KeyCode::Backspace => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                        self.query.remove(self.cursor);
                        return true;
                    }
                }
                KeyCode::Esc => {
                    self.deactivate();
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    /// Filter a list of items by the current search query.
    pub fn filter<'a, T, F>(&self, items: &'a [T], get_text: F) -> Vec<&'a T>
    where
        F: Fn(&T) -> String,
    {
        if self.query.is_empty() {
            return items.iter().collect();
        }
        let q = self.query.to_lowercase();
        items
            .iter()
            .filter(|item| get_text(item).to_lowercase().contains(&q))
            .collect()
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.active {
            let hint = Paragraph::new(Line::from(vec![
                Span::styled("[/]", theme::title()),
                Span::styled(" Search", theme::dim()),
            ]))
            .alignment(ratatui::layout::Alignment::Right);
            frame.render_widget(hint, area);
            return;
        }

        let text = Paragraph::new(Line::from(vec![
            Span::styled("/ ", theme::title()),
            Span::styled(&self.query, theme::normal()),
            Span::styled("█", theme::title()),
        ]))
        .alignment(ratatui::layout::Alignment::Right);
        frame.render_widget(text, area);
    }
}
