use crate::theme;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub struct StatusBar {
    pub connected: bool,
    pub base_url: String,
    pub loading: bool,
}

impl StatusBar {
    pub fn new(base_url: &str) -> Self {
        Self {
            connected: false,
            base_url: base_url.to_string(),
            loading: false,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(theme::border());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::horizontal([Constraint::Min(0), Constraint::Length(8)]).split(inner);

        // Left: key shortcut hints
        let left = if self.loading {
            Paragraph::new(Line::from(vec![
                Span::raw(" "),
                Span::styled("⟳ Loading...", theme::dim()),
            ]))
        } else {
            Paragraph::new(Line::from(vec![
                Span::raw(" "),
                Span::styled("↑↓", theme::title()),
                Span::styled("/", theme::dim()),
                Span::styled("j k", theme::title()),
                Span::styled(" navigate  ", theme::dim()),
                Span::styled("Enter", theme::title()),
                Span::styled(" open  ", theme::dim()),
                Span::styled("c", theme::title()),
                Span::styled(" create  ", theme::dim()),
                Span::styled("t", theme::title()),
                Span::styled(" toggle  ", theme::dim()),
                Span::styled("d", theme::title()),
                Span::styled(" delete  ", theme::dim()),
                Span::styled("/", theme::title()),
                Span::styled(" search  ", theme::dim()),
                Span::styled("e", theme::title()),
                Span::styled(" env  ", theme::dim()),
                Span::styled("p", theme::title()),
                Span::styled(" project  ", theme::dim()),
                Span::styled("l", theme::title()),
                Span::styled(" logout  ", theme::dim()),
                Span::styled("1-6", theme::title()),
                Span::styled(" sections", theme::dim()),
            ]))
        };
        frame.render_widget(left, chunks[0]);

        // Right: q quit
        let right = Paragraph::new(Line::from(vec![
            Span::styled("q", theme::title()),
            Span::styled(" quit", theme::dim()),
        ]))
        .alignment(ratatui::layout::Alignment::Right);
        frame.render_widget(right, chunks[1]);
    }
}
