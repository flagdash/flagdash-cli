use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub struct TextArea {
    pub lines: Vec<String>,
    pub label: String,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub focused: bool,
    pub scroll_offset: usize,
}

impl TextArea {
    pub fn new(label: &str) -> Self {
        Self {
            lines: vec![String::new()],
            label: label.to_string(),
            cursor_row: 0,
            cursor_col: 0,
            focused: false,
            scroll_offset: 0,
        }
    }

    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    pub fn set_content(&mut self, content: &str) {
        self.lines = content.lines().map(String::from).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
    }

    pub fn handle_event(&mut self, event: &Event) -> bool {
        if !self.focused {
            return false;
        }

        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return false;
            }
            match key.code {
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.lines[self.cursor_row].insert(self.cursor_col, c);
                    self.cursor_col += 1;
                    return true;
                }
                KeyCode::Enter => {
                    let rest = self.lines[self.cursor_row].split_off(self.cursor_col);
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                    self.lines.insert(self.cursor_row, rest);
                    self.ensure_visible();
                    return true;
                }
                KeyCode::Backspace => {
                    if self.cursor_col > 0 {
                        self.cursor_col -= 1;
                        self.lines[self.cursor_row].remove(self.cursor_col);
                    } else if self.cursor_row > 0 {
                        let current_line = self.lines.remove(self.cursor_row);
                        self.cursor_row -= 1;
                        self.cursor_col = self.lines[self.cursor_row].len();
                        self.lines[self.cursor_row].push_str(&current_line);
                        self.ensure_visible();
                    }
                    return true;
                }
                KeyCode::Delete => {
                    if self.cursor_col < self.lines[self.cursor_row].len() {
                        self.lines[self.cursor_row].remove(self.cursor_col);
                    } else if self.cursor_row + 1 < self.lines.len() {
                        let next_line = self.lines.remove(self.cursor_row + 1);
                        self.lines[self.cursor_row].push_str(&next_line);
                    }
                    return true;
                }
                KeyCode::Left => {
                    if self.cursor_col > 0 {
                        self.cursor_col -= 1;
                    } else if self.cursor_row > 0 {
                        self.cursor_row -= 1;
                        self.cursor_col = self.lines[self.cursor_row].len();
                        self.ensure_visible();
                    }
                    return true;
                }
                KeyCode::Right => {
                    if self.cursor_col < self.lines[self.cursor_row].len() {
                        self.cursor_col += 1;
                    } else if self.cursor_row + 1 < self.lines.len() {
                        self.cursor_row += 1;
                        self.cursor_col = 0;
                        self.ensure_visible();
                    }
                    return true;
                }
                KeyCode::Up => {
                    if self.cursor_row > 0 {
                        self.cursor_row -= 1;
                        self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                        self.ensure_visible();
                    }
                    return true;
                }
                KeyCode::Down => {
                    if self.cursor_row + 1 < self.lines.len() {
                        self.cursor_row += 1;
                        self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                        self.ensure_visible();
                    }
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    fn ensure_visible(&mut self) {
        if self.cursor_row < self.scroll_offset {
            self.scroll_offset = self.cursor_row;
        }
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

        let inner = block.inner(area);
        let visible_height = inner.height as usize;

        // Adjust scroll offset
        let mut scroll = self.scroll_offset;
        if self.cursor_row >= scroll + visible_height {
            scroll = self.cursor_row - visible_height + 1;
        }
        if self.cursor_row < scroll {
            scroll = self.cursor_row;
        }

        let visible_lines: Vec<Line> = self
            .lines
            .iter()
            .skip(scroll)
            .take(visible_height)
            .map(|l| Line::from(Span::styled(l.as_str(), theme::normal())))
            .collect();

        let paragraph = Paragraph::new(visible_lines).block(block);
        frame.render_widget(paragraph, area);

        // Show cursor
        if self.focused {
            let cx = area.x + 1 + self.cursor_col as u16;
            let cy = area.y + 1 + (self.cursor_row - scroll) as u16;
            if cy < area.y + area.height - 1 {
                frame.set_cursor_position((cx, cy));
            }
        }
    }
}
