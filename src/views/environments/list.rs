use crate::api::types::Environment;
use crate::components::table_view::TableView;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct EnvironmentListView {
    pub environments: Vec<Environment>,
    pub table: TableView,
}

impl EnvironmentListView {
    pub fn new() -> Self {
        Self {
            environments: Vec::new(),
            table: TableView::new(),
        }
    }

    pub fn set_environments(&mut self, envs: Vec<Environment>) {
        self.environments = envs;
        self.table.set_items(self.environments.len());
    }

    pub fn handle_event(&mut self, event: &Event) {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return;
            }
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => self.table.select_next(),
                KeyCode::Up | KeyCode::Char('k') => self.table.select_prev(),
                _ => {}
            }
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                "Environments",
                theme::heading(),
            )])),
            chunks[0],
        );

        let rows: Vec<Vec<String>> = self
            .environments
            .iter()
            .map(|e| {
                vec![
                    e.name.clone(),
                    e.slug.clone(),
                    e.id.clone(),
                    if e.is_default {
                        "Default".to_string()
                    } else {
                        "-".to_string()
                    },
                ]
            })
            .collect();

        self.table.render(
            frame,
            chunks[1],
            "Environments",
            &["Name", "Slug", "ID", "Default"],
            &[
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(35),
                Constraint::Percentage(15),
            ],
            rows,
        );

        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                "Read-only view",
                theme::dim(),
            )])),
            chunks[2],
        );
    }
}
