use crate::theme;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub struct Header {
    pub project_name: String,
    pub environment_name: String,
    pub connected: bool,
}

impl Header {
    pub fn new() -> Self {
        Self {
            project_name: String::new(),
            environment_name: String::new(),
            connected: false,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(theme::border());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::horizontal([Constraint::Min(0), Constraint::Length(26)]).split(inner);

        // Left: ◆ FlagDash  │  project > environment
        let mut left_spans = vec![
            Span::styled(" ◆ ", theme::title()),
            Span::styled("FlagDash", theme::title()),
        ];
        if !self.project_name.is_empty() {
            left_spans.push(Span::styled("  |  ", theme::dim()));
            left_spans.push(Span::styled(self.project_name.clone(), theme::normal()));
            if !self.environment_name.is_empty() {
                left_spans.push(Span::styled(" › ", theme::dim()));
                left_spans.push(Span::styled(self.environment_name.clone(), theme::title()));
            }
        }
        let left = Paragraph::new(Line::from(left_spans));
        frame.render_widget(left, chunks[0]);

        // Right: v0.1.0  ● connected
        let version = env!("CARGO_PKG_VERSION");
        let (status_label, status_style) = if self.connected {
            ("● connected", theme::status_on())
        } else {
            ("○ disconnected", theme::status_off())
        };
        let right = Paragraph::new(Line::from(vec![
            Span::styled(format!("v{}  ", version), theme::dim()),
            Span::styled(status_label, status_style),
            Span::raw(" "),
        ]))
        .alignment(ratatui::layout::Alignment::Right);
        frame.render_widget(right, chunks[1]);
    }
}
