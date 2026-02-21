use crate::action::{Action, View};
use crate::api::types::ManagedConfig;
use crate::config::KeyTier;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

pub struct ConfigDetailView {
    pub config: Option<ManagedConfig>,
    pub key_tier: KeyTier,
}

impl ConfigDetailView {
    pub fn new(key_tier: KeyTier) -> Self {
        Self {
            config: None,
            key_tier,
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }
            let config = self.config.as_ref()?;
            match key.code {
                KeyCode::Esc | KeyCode::Backspace => {
                    return Some(Action::Navigate(View::ConfigList));
                }
                KeyCode::Char('e') if self.key_tier.can_mutate() => {
                    return Some(Action::Navigate(View::ConfigEdit(config.key.clone())));
                }
                KeyCode::Char('v') if self.key_tier.can_mutate() => {
                    return Some(Action::Navigate(View::ConfigValueEditor(
                        config.key.clone(),
                    )));
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
            Constraint::Length(6),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);

        let title = Paragraph::new(Line::from(vec![
            Span::styled("← ", theme::dim()),
            Span::styled(&config.name, theme::heading()),
            Span::styled(format!("  ({})", config.key), theme::dim()),
        ]));
        frame.render_widget(title, chunks[0]);

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
        ])
        .split(inner);

        let default_str = format_value(&config.default_value);
        let fields = [
            ("Type: ", config.config_type.as_str()),
            ("Default: ", &default_str),
            (
                "Description: ",
                if config.description.is_empty() {
                    "—"
                } else {
                    &config.description
                },
            ),
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

        // Environment values
        let env_rows: Vec<Row> = config
            .environments
            .iter()
            .map(|env| {
                let active = if env.is_active { "Active" } else { "Inactive" };
                let style = if env.is_active {
                    theme::status_on()
                } else {
                    theme::status_off()
                };
                let value_str = format_value(&env.value);
                Row::new(vec![
                    Cell::from(env.environment_id.as_str()).style(theme::normal()),
                    Cell::from(value_str).style(theme::normal()),
                    Cell::from(active).style(style),
                ])
            })
            .collect();

        let table = Table::new(
            env_rows,
            [
                Constraint::Percentage(25),
                Constraint::Percentage(55),
                Constraint::Percentage(20),
            ],
        )
        .header(Row::new(vec!["Environment", "Value", "Status"]).style(theme::heading()))
        .block(
            Block::default()
                .title(" Environment Values ")
                .title_style(theme::heading())
                .borders(Borders::ALL)
                .border_style(theme::border()),
        );
        frame.render_widget(table, chunks[2]);

        let mut spans = vec![
            Span::styled("[Esc]", theme::title()),
            Span::styled("Back ", theme::dim()),
        ];
        if self.key_tier.can_mutate() {
            spans.extend([
                Span::styled("[e]", theme::title()),
                Span::styled("Edit ", theme::dim()),
                Span::styled("[v]", theme::title()),
                Span::styled("Set Value", theme::dim()),
            ]);
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), chunks[3]);
    }
}

fn format_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => format!("\"{}\"", s),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
            serde_json::to_string(value).unwrap_or_else(|_| value.to_string())
        }
        other => other.to_string(),
    }
}
