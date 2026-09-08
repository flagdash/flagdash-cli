use crate::action::{Action, ConfirmAction, View};
use crate::api::types::WebhookEndpoint;
use crate::components::table_view::TableView;
use crate::config::KeyTier;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct WebhookListView {
    pub webhooks: Vec<WebhookEndpoint>,
    pub table: TableView,
    pub key_tier: KeyTier,
}

impl WebhookListView {
    pub fn new(key_tier: KeyTier) -> Self {
        Self {
            webhooks: Vec::new(),
            table: TableView::new(),
            key_tier,
        }
    }

    pub fn set_webhooks(&mut self, webhooks: Vec<WebhookEndpoint>) {
        self.webhooks = webhooks;
        self.table.set_items(self.webhooks.len());
    }

    pub fn selected_webhook(&self) -> Option<&WebhookEndpoint> {
        self.table
            .selected_index()
            .and_then(|i| self.webhooks.get(i))
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => self.table.select_next(),
                KeyCode::Up | KeyCode::Char('k') => self.table.select_prev(),
                KeyCode::Enter => {
                    if let Some(w) = self.selected_webhook() {
                        return Some(Action::Navigate(View::WebhookDetail(w.id.clone())));
                    }
                }
                KeyCode::Char('c') if self.key_tier.can_mutate() => {
                    return Some(Action::Navigate(View::WebhookCreate));
                }
                KeyCode::Char('d') if self.key_tier.can_mutate() => {
                    if let Some(w) = self.selected_webhook() {
                        return Some(Action::ShowConfirm(ConfirmAction::DeleteWebhook(
                            w.id.clone(),
                        )));
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled("Webhooks", theme::heading())])),
            chunks[0],
        );

        let rows: Vec<Vec<String>> = self
            .webhooks
            .iter()
            .map(|w| {
                let status = if w.is_active { "Active" } else { "Disabled" };
                vec![
                    truncate(&w.url, 35),
                    w.event_types.join(", "),
                    status.to_string(),
                    format!("{}", w.consecutive_failures),
                ]
            })
            .collect();

        self.table.render(
            frame,
            chunks[1],
            "Webhooks",
            &["URL", "Events", "Status", "Failures"],
            &[
                Constraint::Percentage(35),
                Constraint::Percentage(30),
                Constraint::Percentage(15),
                Constraint::Percentage(20),
            ],
            rows,
        );

        let mut spans = vec![
            Span::styled("[Enter]", theme::title()),
            Span::styled("Detail ", theme::dim()),
        ];
        if self.key_tier.can_mutate() {
            spans.extend([
                Span::styled("[c]", theme::title()),
                Span::styled("Create ", theme::dim()),
                Span::styled("[d]", theme::title()),
                Span::styled("Delete", theme::dim()),
            ]);
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), chunks[2]);
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max - 1])
    } else {
        s.to_string()
    }
}
