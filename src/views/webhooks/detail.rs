use crate::action::{Action, View};
use crate::api::types::{WebhookDelivery, WebhookEndpoint};
use crate::config::KeyTier;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

pub struct WebhookDetailView {
    pub webhook: Option<WebhookEndpoint>,
    pub deliveries: Vec<WebhookDelivery>,
    pub key_tier: KeyTier,
}

impl WebhookDetailView {
    pub fn new(key_tier: KeyTier) -> Self {
        Self {
            webhook: None,
            deliveries: Vec::new(),
            key_tier,
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }
            let webhook = self.webhook.as_ref()?;
            match key.code {
                KeyCode::Esc | KeyCode::Backspace => {
                    return Some(Action::Navigate(View::WebhookList))
                }
                KeyCode::Char('e') if self.key_tier.can_mutate() => {
                    return Some(Action::Navigate(View::WebhookEdit(webhook.id.clone())));
                }
                _ => {}
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let webhook = match &self.webhook {
            Some(w) => w,
            None => {
                frame.render_widget(Paragraph::new("Loading...").style(theme::dim()), area);
                return;
            }
        };

        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(7),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("← ", theme::dim()),
                Span::styled(&webhook.url, theme::heading()),
            ])),
            chunks[0],
        );

        // Info
        let info_block = Block::default()
            .title(" Details ")
            .title_style(theme::heading())
            .borders(Borders::ALL)
            .border_style(theme::border());
        let inner = info_block.inner(chunks[1]);
        frame.render_widget(info_block, chunks[1]);
        let info_rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

        let status_str = if webhook.is_active {
            "Active"
        } else {
            "Disabled"
        };
        let fields: Vec<(&str, String)> = vec![
            ("Status: ", status_str.to_string()),
            ("Events: ", webhook.event_types.join(", ")),
            ("Failures: ", format!("{}", webhook.consecutive_failures)),
            ("Description: ", webhook.description.clone()),
        ];
        for (i, (label, val)) in fields.iter().enumerate() {
            if i < info_rows.len() {
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled(*label, theme::dim()),
                        Span::styled(val.as_str(), theme::normal()),
                    ])),
                    info_rows[i],
                );
            }
        }

        // Deliveries
        let del_rows: Vec<Row> = self
            .deliveries
            .iter()
            .map(|d| {
                let status_style = match d.status.as_str() {
                    "success" => theme::status_on(),
                    "failed" | "error" => theme::status_off(),
                    _ => theme::dim(),
                };
                Row::new(vec![
                    Cell::from(d.event_type.as_str()).style(theme::normal()),
                    Cell::from(d.status.as_str()).style(status_style),
                    Cell::from(format!("{}", d.http_status)).style(theme::normal()),
                    Cell::from(format!("{}/{}", d.attempt_count, d.max_attempts))
                        .style(theme::dim()),
                    Cell::from(d.created_at.format("%m-%d %H:%M").to_string()).style(theme::dim()),
                ])
            })
            .collect();

        let del_table = Table::new(
            del_rows,
            [
                Constraint::Percentage(25),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(30),
            ],
        )
        .header(
            Row::new(vec!["Event", "Status", "HTTP", "Attempts", "Created"])
                .style(theme::heading()),
        )
        .block(
            Block::default()
                .title(format!(" Deliveries ({}) ", self.deliveries.len()))
                .title_style(theme::heading())
                .borders(Borders::ALL)
                .border_style(theme::border()),
        );
        frame.render_widget(del_table, chunks[2]);

        let mut spans = vec![
            Span::styled("[Esc]", theme::title()),
            Span::styled("Back ", theme::dim()),
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
