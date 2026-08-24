use crate::action::{Action, ConfirmAction, View};
use crate::api::types::ManagedAiConfig;
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

pub struct AiConfigListView {
    pub ai_configs: Vec<ManagedAiConfig>,
    pub table: TableView,
    pub search: SearchBar,
    pub key_tier: KeyTier,
    filtered_indices: Vec<usize>,
}

impl AiConfigListView {
    pub fn new(key_tier: KeyTier) -> Self {
        Self {
            ai_configs: Vec::new(),
            table: TableView::new(),
            search: SearchBar::new(),
            key_tier,
            filtered_indices: Vec::new(),
        }
    }

    pub fn set_ai_configs(&mut self, configs: Vec<ManagedAiConfig>) {
        self.ai_configs = configs;
        self.update_filter();
    }

    fn update_filter(&mut self) {
        self.filtered_indices = if self.search.query.is_empty() {
            (0..self.ai_configs.len()).collect()
        } else {
            let q = self.search.query.to_lowercase();
            self.ai_configs
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    c.file_name.to_lowercase().contains(&q)
                        || c.file_type.to_lowercase().contains(&q)
                        || c.folder.to_lowercase().contains(&q)
                })
                .map(|(i, _)| i)
                .collect()
        };
        self.table.set_items(self.filtered_indices.len());
    }

    pub fn selected_config(&self) -> Option<&ManagedAiConfig> {
        self.table
            .selected_index()
            .and_then(|i| self.filtered_indices.get(i))
            .and_then(|&idx| self.ai_configs.get(idx))
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
                    if let Some(c) = self.selected_config() {
                        return Some(Action::Navigate(View::AiConfigDetail(c.file_name.clone())));
                    }
                }
                KeyCode::Char('c') if self.key_tier.can_mutate() => {
                    return Some(Action::Navigate(View::AiConfigCreate));
                }
                KeyCode::Char('d') if self.key_tier.can_mutate() => {
                    if let Some(c) = self.selected_config() {
                        return Some(Action::ShowConfirm(ConfirmAction::DeleteAiConfig(
                            c.file_name.clone(),
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
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                "AI Configs",
                theme::heading(),
            )])),
            header_chunks[0],
        );
        self.search.render(frame, header_chunks[1]);

        let rows: Vec<Vec<String>> = self
            .filtered_indices
            .iter()
            .filter_map(|&idx| self.ai_configs.get(idx))
            .map(|c| {
                vec![
                    c.file_name.clone(),
                    c.file_type.clone(),
                    if c.folder.is_empty() {
                        "-".to_string()
                    } else {
                        c.folder.clone()
                    },
                    if c.is_active {
                        "Active".to_string()
                    } else {
                        "Inactive".to_string()
                    },
                    c.environment_id.clone(),
                ]
            })
            .collect();

        self.table.render(
            frame,
            chunks[1],
            "AI Configs",
            &["File", "Type", "Folder", "Status", "Environment"],
            &[
                Constraint::Percentage(25),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(30),
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
