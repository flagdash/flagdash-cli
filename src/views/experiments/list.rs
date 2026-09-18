use crate::action::{Action, View};
use crate::api::types::ManagedExperiment;
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

pub struct ExperimentListView {
    pub experiments: Vec<ManagedExperiment>,
    pub table: TableView,
    pub search: SearchBar,
    pub key_tier: KeyTier,
    filtered_indices: Vec<usize>,
}

impl ExperimentListView {
    pub fn new(key_tier: KeyTier) -> Self {
        Self {
            experiments: Vec::new(),
            table: TableView::new(),
            search: SearchBar::new(),
            key_tier,
            filtered_indices: Vec::new(),
        }
    }

    pub fn set_experiments(&mut self, experiments: Vec<ManagedExperiment>) {
        self.experiments = experiments;
        self.update_filter();
    }

    fn update_filter(&mut self) {
        self.filtered_indices = if self.search.query.is_empty() {
            (0..self.experiments.len()).collect()
        } else {
            let query = self.search.query.to_lowercase();
            self.experiments
                .iter()
                .enumerate()
                .filter(|(_, experiment)| {
                    experiment.key.to_lowercase().contains(&query)
                        || experiment.name.to_lowercase().contains(&query)
                })
                .map(|(index, _)| index)
                .collect()
        };
        self.table.set_items(self.filtered_indices.len());
    }

    fn selected_experiment(&self) -> Option<&ManagedExperiment> {
        self.table
            .selected_index()
            .and_then(|index| self.filtered_indices.get(index))
            .and_then(|index| self.experiments.get(*index))
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
                KeyCode::Char('/') if !self.search.active => self.search.activate(),
                KeyCode::Down | KeyCode::Char('j') => self.table.select_next(),
                KeyCode::Up | KeyCode::Char('k') => self.table.select_prev(),
                KeyCode::Enter => {
                    if let Some(experiment) = self.selected_experiment() {
                        return Some(Action::Navigate(View::ExperimentDetail(
                            experiment.key.clone(),
                        )));
                    }
                }
                KeyCode::Char('c') if self.key_tier.can_mutate() => {
                    return Some(Action::Navigate(View::ExperimentCreate));
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
        let header =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(30)]).split(chunks[0]);

        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                "Experiments",
                theme::heading(),
            )])),
            header[0],
        );
        self.search.render(frame, header[1]);

        let rows = self
            .filtered_indices
            .iter()
            .filter_map(|index| self.experiments.get(*index))
            .map(|experiment| {
                vec![
                    experiment.key.clone(),
                    experiment.name.clone(),
                    experiment.status.clone(),
                    if experiment.decision.is_empty() {
                        "—".to_string()
                    } else {
                        experiment.decision.clone()
                    },
                ]
            })
            .collect();

        self.table.render(
            frame,
            chunks[1],
            "Experiments",
            &["Key", "Name", "Status", "Decision"],
            &[
                Constraint::Percentage(25),
                Constraint::Percentage(35),
                Constraint::Percentage(15),
                Constraint::Percentage(25),
            ],
            rows,
        );

        let mut help = vec![
            Span::styled("[Enter]", theme::title()),
            Span::styled("Detail ", theme::dim()),
        ];
        if self.key_tier.can_mutate() {
            help.extend([
                Span::styled("[c]", theme::title()),
                Span::styled("Create ", theme::dim()),
            ]);
        }
        help.extend([
            Span::styled("[/]", theme::title()),
            Span::styled("Search", theme::dim()),
        ]);
        frame.render_widget(Paragraph::new(Line::from(help)), chunks[2]);
    }
}
