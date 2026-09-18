use crate::action::{Action, ConfirmAction, View};
use crate::api::types::ManagedFlag;
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

pub struct FlagListView {
    pub flags: Vec<ManagedFlag>,
    pub table: TableView,
    pub search: SearchBar,
    pub key_tier: KeyTier,
    filtered_indices: Vec<usize>,
}

impl FlagListView {
    pub fn new(key_tier: KeyTier) -> Self {
        Self {
            flags: Vec::new(),
            table: TableView::new(),
            search: SearchBar::new(),
            key_tier,
            filtered_indices: Vec::new(),
        }
    }

    pub fn set_flags(&mut self, flags: Vec<ManagedFlag>) {
        self.flags = flags;
        self.update_filter();
    }

    fn update_filter(&mut self) {
        self.filtered_indices = if self.search.query.is_empty() {
            (0..self.flags.len()).collect()
        } else {
            let q = self.search.query.to_lowercase();
            self.flags
                .iter()
                .enumerate()
                .filter(|(_, f)| {
                    f.key.to_lowercase().contains(&q) || f.name.to_lowercase().contains(&q)
                })
                .map(|(i, _)| i)
                .collect()
        };
        self.table.set_items(self.filtered_indices.len());
    }

    pub fn selected_flag(&self) -> Option<&ManagedFlag> {
        self.table
            .selected_index()
            .and_then(|i| self.filtered_indices.get(i))
            .and_then(|&idx| self.flags.get(idx))
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
                    return None;
                }
                KeyCode::Down | KeyCode::Char('j') => self.table.select_next(),
                KeyCode::Up | KeyCode::Char('k') => self.table.select_prev(),
                KeyCode::Enter => {
                    if let Some(flag) = self.selected_flag() {
                        return Some(Action::Navigate(View::FlagDetail(flag.key.clone())));
                    }
                }
                KeyCode::Char('c') if self.key_tier.can_mutate() => {
                    return Some(Action::Navigate(View::FlagCreate));
                }
                KeyCode::Char('t') if self.key_tier.can_mutate() => {
                    if let Some(flag) = self.selected_flag() {
                        return Some(Action::Navigate(View::FlagToggle(flag.key.clone())));
                    }
                }
                KeyCode::Char('d') if self.key_tier.can_mutate() => {
                    if let Some(flag) = self.selected_flag() {
                        return Some(Action::ShowConfirm(ConfirmAction::DeleteFlag(
                            flag.key.clone(),
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
            Constraint::Length(2), // Header + search
            Constraint::Min(0),    // Table
            Constraint::Length(1), // Shortcuts
        ])
        .split(area);

        // Header with search
        let header_chunks =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(30)]).split(chunks[0]);

        let title = Paragraph::new(Line::from(vec![Span::styled("Flags", theme::heading())]));
        frame.render_widget(title, header_chunks[0]);
        self.search.render(frame, header_chunks[1]);

        // Table
        let rows: Vec<Vec<String>> = self
            .filtered_indices
            .iter()
            .filter_map(|&idx| self.flags.get(idx))
            .map(|f| {
                let enabled_count = f.environments.iter().filter(|e| e.enabled).count();
                let env_count = f.environments.len();
                let status = if env_count == 0 {
                    "—".to_string()
                } else if enabled_count == env_count {
                    "ON".to_string()
                } else if enabled_count == 0 {
                    "OFF".to_string()
                } else {
                    format!("{}/{}", enabled_count, env_count)
                };
                vec![
                    f.key.clone(),
                    truncate(&f.name, 25),
                    f.flag_type.clone(),
                    status,
                ]
            })
            .collect();

        self.table.render(
            frame,
            chunks[1],
            "Flags",
            &["Key", "Name", "Type", "Status"],
            &[
                Constraint::Percentage(30),
                Constraint::Percentage(35),
                Constraint::Percentage(15),
                Constraint::Percentage(20),
            ],
            rows,
        );

        // Shortcuts
        let mut shortcut_spans = vec![
            Span::styled("[Enter]", theme::title()),
            Span::styled("Detail ", theme::dim()),
        ];
        if self.key_tier.can_mutate() {
            shortcut_spans.extend([
                Span::styled("[c]", theme::title()),
                Span::styled("Create ", theme::dim()),
                Span::styled("[t]", theme::title()),
                Span::styled("Toggle ", theme::dim()),
                Span::styled("[d]", theme::title()),
                Span::styled("Delete ", theme::dim()),
            ]);
        }
        shortcut_spans.extend([
            Span::styled("[/]", theme::title()),
            Span::styled("Search", theme::dim()),
        ]);
        let shortcuts = Paragraph::new(Line::from(shortcut_spans));
        frame.render_widget(shortcuts, chunks[2]);
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max - 1])
    } else {
        s.to_string()
    }
}
