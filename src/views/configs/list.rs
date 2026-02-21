use crate::action::{Action, ConfirmAction, View};
use crate::api::types::ManagedConfig;
use crate::components::search_bar::SearchBar;
use crate::components::table_view::TableView;
use crate::config::KeyTier;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct ConfigListView {
    pub configs: Vec<ManagedConfig>,
    pub table: TableView,
    pub search: SearchBar,
    pub key_tier: KeyTier,
    filtered_indices: Vec<usize>,
}

impl ConfigListView {
    pub fn new(key_tier: KeyTier) -> Self {
        Self {
            configs: Vec::new(),
            table: TableView::new(),
            search: SearchBar::new(),
            key_tier,
            filtered_indices: Vec::new(),
        }
    }

    pub fn set_configs(&mut self, configs: Vec<ManagedConfig>) {
        self.configs = configs;
        self.update_filter();
    }

    fn update_filter(&mut self) {
        self.filtered_indices = if self.search.query.is_empty() {
            (0..self.configs.len()).collect()
        } else {
            let q = self.search.query.to_lowercase();
            self.configs
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    c.key.to_lowercase().contains(&q) || c.name.to_lowercase().contains(&q)
                })
                .map(|(i, _)| i)
                .collect()
        };
        self.table.set_items(self.filtered_indices.len());
    }

    pub fn selected_config(&self) -> Option<&ManagedConfig> {
        self.table
            .selected_index()
            .and_then(|i| self.filtered_indices.get(i))
            .and_then(|&idx| self.configs.get(idx))
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        if self.search.active && self.search.handle_event(event) {
            self.update_filter();
            return None;
        }

        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }
            match key.code {
                KeyCode::Char('/') if !self.search.active => {
                    self.search.activate();
                }
                KeyCode::Down | KeyCode::Char('j') => self.table.select_next(),
                KeyCode::Up | KeyCode::Char('k') => self.table.select_prev(),
                KeyCode::Enter => {
                    if let Some(config) = self.selected_config() {
                        return Some(Action::Navigate(View::ConfigDetail(config.key.clone())));
                    }
                }
                KeyCode::Char('c') if self.key_tier.can_mutate() => {
                    return Some(Action::Navigate(View::ConfigCreate));
                }
                KeyCode::Char('d') if self.key_tier.can_mutate() => {
                    if let Some(config) = self.selected_config() {
                        return Some(Action::ShowConfirm(ConfirmAction::DeleteConfig(
                            config.key.clone(),
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

        let header_chunks =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(30)]).split(chunks[0]);

        let title = Paragraph::new(Line::from(vec![Span::styled("Configs", theme::heading())]));
        frame.render_widget(title, header_chunks[0]);
        self.search.render(frame, header_chunks[1]);

        let rows: Vec<Vec<String>> = self
            .filtered_indices
            .iter()
            .filter_map(|&idx| self.configs.get(idx))
            .map(|c| {
                let value_preview = format_value_preview(&c.default_value, 30);
                vec![
                    c.key.clone(),
                    truncate(&c.name, 20),
                    c.config_type.clone(),
                    value_preview,
                ]
            })
            .collect();

        self.table.render(
            frame,
            chunks[1],
            "Configs",
            &["Key", "Name", "Type", "Default Value"],
            &[
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(12),
                Constraint::Percentage(38),
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
                Span::styled("Delete ", theme::dim()),
            ]);
        }
        spans.extend([
            Span::styled("[/]", theme::title()),
            Span::styled("Search", theme::dim()),
        ]);
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

fn format_value_preview(value: &serde_json::Value, max: usize) -> String {
    let s = match value {
        serde_json::Value::String(s) => format!("\"{}\"", s),
        serde_json::Value::Null => "null".to_string(),
        other => other.to_string(),
    };
    truncate(&s, max)
}
