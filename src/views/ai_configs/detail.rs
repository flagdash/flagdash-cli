use crate::action::{Action, View};
use crate::api::types::ManagedAiConfig;
use crate::config::KeyTier;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub struct AiConfigDetailView {
    pub config: Option<ManagedAiConfig>,
    pub key_tier: KeyTier,
    scroll: u16,
}

impl AiConfigDetailView {
    pub fn new(key_tier: KeyTier) -> Self {
        Self {
            config: None,
            key_tier,
            scroll: 0,
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Backspace => {
                    return Some(Action::Navigate(View::AiConfigList));
                }
                KeyCode::Char('e') if self.key_tier.can_mutate() => {
                    if let Some(c) = &self.config {
                        return Some(Action::Navigate(View::AiConfigEdit(c.file_name.clone())));
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll = self.scroll.saturating_add(1);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scroll = self.scroll.saturating_sub(1);
                }
                _ => {}
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let config = match &self.config {
            Some(c) => c,
            None => {
                frame.render_widget(Paragraph::new("Loading...").style(theme::dim()), area);
                return;
            }
        };

        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(5),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("← ", theme::dim()),
                Span::styled(&config.file_name, theme::heading()),
                Span::styled(format!("  [{}]", config.file_type), theme::dim()),
            ])),
            chunks[0],
        );

        // Info
        let info_block = Block::default()
            .title(" Info ")
            .title_style(theme::heading())
            .borders(Borders::ALL)
            .border_style(theme::border());
        let inner = info_block.inner(chunks[1]);
        frame.render_widget(info_block, chunks[1]);

        let info_rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

        let fields = [
            (
                "Folder: ",
                if config.folder.is_empty() {
                    "(root)"
                } else {
                    &config.folder
                },
            ),
            (
                "Status: ",
                if config.is_active {
                    "Active"
                } else {
                    "Inactive"
                },
            ),
            ("Environment: ", &config.environment_id),
        ];
        for (i, (label, val)) in fields.iter().enumerate() {
            if i < info_rows.len() {
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled(*label, theme::dim()),
                        Span::styled(*val, theme::normal()),
                    ])),
                    info_rows[i],
                );
            }
        }

        // Content viewer
        let content = Paragraph::new(config.content.as_str())
            .style(theme::normal())
            .block(
                Block::default()
                    .title(" Content ")
                    .title_style(theme::heading())
                    .borders(Borders::ALL)
                    .border_style(theme::border()),
            )
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));
        frame.render_widget(content, chunks[2]);

        let mut spans = vec![
            Span::styled("[Esc]", theme::title()),
            Span::styled("Back ", theme::dim()),
            Span::styled("[↑↓]", theme::title()),
            Span::styled("Scroll ", theme::dim()),
        ];
        if self.key_tier.can_mutate() {
            spans.extend([
                Span::styled("[e]", theme::title()),
                Span::styled("Edit", theme::dim()),
            ]);
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), chunks[3]);
    }
}
