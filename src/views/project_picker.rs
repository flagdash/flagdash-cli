use crate::action::Action;
use crate::api::types::{Environment, Project};
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    SelectProject,
    SelectEnvironment,
}

pub struct ProjectPickerView {
    phase: Phase,
    projects: Vec<Project>,
    environments: Vec<Environment>,
    selected_project_idx: usize,
    selected_env_idx: usize,
    chosen_project_id: String,
    loading: bool,
    saved_project_id: String,
    saved_environment_id: String,
    has_saved_project: bool,
}

impl ProjectPickerView {
    pub fn new() -> Self {
        Self {
            phase: Phase::SelectProject,
            projects: Vec::new(),
            environments: Vec::new(),
            selected_project_idx: 0,
            selected_env_idx: 0,
            chosen_project_id: String::new(),
            loading: true,
            saved_project_id: String::new(),
            saved_environment_id: String::new(),
            has_saved_project: false,
        }
    }

    pub fn set_saved_defaults(&mut self, project_id: &str, environment_id: &str) {
        self.saved_project_id = project_id.to_string();
        self.saved_environment_id = environment_id.to_string();
        self.has_saved_project = !project_id.is_empty();
    }

    pub fn set_projects(&mut self, projects: Vec<Project>) {
        // Pre-select the saved project if it exists
        self.selected_project_idx = if !self.saved_project_id.is_empty() {
            projects
                .iter()
                .position(|p| p.id == self.saved_project_id)
                .unwrap_or(0)
        } else {
            0
        };
        self.projects = projects;
        self.loading = false;
    }

    pub fn set_environments(&mut self, environments: Vec<Environment>) {
        // Pre-select saved environment, then fall back to default environment
        self.selected_env_idx = if !self.saved_environment_id.is_empty() {
            environments
                .iter()
                .position(|e| e.id == self.saved_environment_id)
                .unwrap_or_else(|| environments.iter().position(|e| e.is_default).unwrap_or(0))
        } else {
            environments.iter().position(|e| e.is_default).unwrap_or(0)
        };
        self.environments = environments;
        self.phase = Phase::SelectEnvironment;
        self.loading = false;
    }

    pub fn reset(&mut self) {
        self.phase = Phase::SelectProject;
        self.projects.clear();
        self.environments.clear();
        self.selected_project_idx = 0;
        self.selected_env_idx = 0;
        self.chosen_project_id.clear();
        self.loading = true;
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }

            if self.loading {
                if key.code == KeyCode::Esc {
                    if self.has_saved_project {
                        return Some(Action::Back);
                    }
                    return Some(Action::Quit);
                }
                return None;
            }

            match self.phase {
                Phase::SelectProject => self.handle_project_selection(key.code),
                Phase::SelectEnvironment => self.handle_env_selection(key.code),
            }
        } else {
            None
        }
    }

    fn handle_project_selection(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_project_idx > 0 {
                    self.selected_project_idx -= 1;
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.projects.is_empty() && self.selected_project_idx < self.projects.len() - 1
                {
                    self.selected_project_idx += 1;
                }
                None
            }
            KeyCode::Enter => {
                if let Some(project) = self.projects.get(self.selected_project_idx) {
                    self.chosen_project_id = project.id.clone();
                    self.loading = true;
                    Some(Action::PickerProjectChosen(project.id.clone()))
                } else {
                    None
                }
            }
            KeyCode::Esc => {
                if self.has_saved_project {
                    Some(Action::Back)
                } else {
                    Some(Action::Quit)
                }
            }
            _ => None,
        }
    }

    fn handle_env_selection(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_env_idx > 0 {
                    self.selected_env_idx -= 1;
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.environments.is_empty()
                    && self.selected_env_idx < self.environments.len() - 1
                {
                    self.selected_env_idx += 1;
                }
                None
            }
            KeyCode::Enter => {
                if let Some(env) = self.environments.get(self.selected_env_idx) {
                    let project_name = self
                        .projects
                        .iter()
                        .find(|p| p.id == self.chosen_project_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_default();
                    Some(Action::ProjectSelected {
                        project_id: self.chosen_project_id.clone(),
                        environment_id: env.id.clone(),
                        project_name,
                        environment_name: env.name.clone(),
                    })
                } else {
                    None
                }
            }
            KeyCode::Esc => {
                // Go back to project selection
                self.phase = Phase::SelectProject;
                self.environments.clear();
                self.selected_env_idx = 0;
                None
            }
            _ => None,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(6), // Logo
            Constraint::Length(2), // Title
            Constraint::Length(1), // Subtitle
            Constraint::Length(1), // Spacer
            Constraint::Min(8),    // Table
            Constraint::Length(2), // Instructions
            Constraint::Min(0),
        ])
        .split(area);

        let center = |r: Rect, w: u16| -> Rect {
            let pad = r.width.saturating_sub(w) / 2;
            Rect::new(r.x + pad, r.y, w.min(r.width), r.height)
        };

        // Logo
        let logo = Paragraph::new(theme::LOGO)
            .style(theme::title())
            .alignment(Alignment::Center);
        frame.render_widget(logo, chunks[1]);

        if self.loading {
            let loading = Paragraph::new(Line::from(Span::styled("Loading...", theme::dim())))
                .alignment(Alignment::Center);
            frame.render_widget(loading, chunks[2]);
            return;
        }

        match self.phase {
            Phase::SelectProject => self.render_project_phase(frame, &chunks, center),
            Phase::SelectEnvironment => self.render_env_phase(frame, &chunks, center),
        }
    }

    fn render_project_phase(
        &self,
        frame: &mut Frame,
        chunks: &[Rect],
        center: impl Fn(Rect, u16) -> Rect,
    ) {
        // Title
        let title = Paragraph::new(Line::from(Span::styled(
            "Select a Project",
            theme::heading(),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(title, chunks[2]);

        // Subtitle
        let subtitle = Paragraph::new(Line::from(Span::styled(
            "Choose which project to work with",
            theme::dim(),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(subtitle, chunks[3]);

        if self.projects.is_empty() {
            let empty = Paragraph::new(Line::from(Span::styled(
                "No projects found. Create one at flagdash.io",
                theme::dim(),
            )))
            .alignment(Alignment::Center);
            frame.render_widget(empty, chunks[5]);
            return;
        }

        // Projects table
        let table_area = center(chunks[5], 70);
        let header = Row::new(vec!["", "Name", "Slug"])
            .style(theme::dim())
            .height(1);

        let rows: Vec<Row> = self
            .projects
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let marker = if i == self.selected_project_idx {
                    ">"
                } else {
                    " "
                };
                let is_saved = p.id == self.saved_project_id;
                let saved_badge = if is_saved { " ●" } else { "" };
                let style = if i == self.selected_project_idx {
                    theme::title()
                } else {
                    theme::normal()
                };
                Row::new(vec![
                    marker.to_string(),
                    format!("{}{}", p.name, saved_badge),
                    p.slug.clone(),
                ])
                .style(style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(2),
                Constraint::Min(20),
                Constraint::Min(15),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border()),
        );

        frame.render_widget(table, table_area);

        // Instructions
        let esc_label = if self.has_saved_project {
            " back"
        } else {
            " quit"
        };
        let instructions = Paragraph::new(Line::from(vec![
            Span::styled("j/k", theme::title()),
            Span::styled(" navigate  ", theme::dim()),
            Span::styled("Enter", theme::title()),
            Span::styled(" select  ", theme::dim()),
            Span::styled("Esc", theme::title()),
            Span::styled(esc_label, theme::dim()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[6]);
    }

    fn render_env_phase(
        &self,
        frame: &mut Frame,
        chunks: &[Rect],
        center: impl Fn(Rect, u16) -> Rect,
    ) {
        // Title
        let project_name = self
            .projects
            .iter()
            .find(|p| p.id == self.chosen_project_id)
            .map(|p| p.name.as_str())
            .unwrap_or("Project");

        let title = Paragraph::new(Line::from(Span::styled(
            "Select an Environment",
            theme::heading(),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(title, chunks[2]);

        let subtitle = Paragraph::new(Line::from(vec![
            Span::styled("for ", theme::dim()),
            Span::styled(project_name, theme::title()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(subtitle, chunks[3]);

        if self.environments.is_empty() {
            let empty = Paragraph::new(Line::from(Span::styled(
                "No environments found",
                theme::dim(),
            )))
            .alignment(Alignment::Center);
            frame.render_widget(empty, chunks[5]);
            return;
        }

        // Environments table
        let table_area = center(chunks[5], 60);
        let header = Row::new(vec!["", "Name", "Slug", "Default"])
            .style(theme::dim())
            .height(1);

        let rows: Vec<Row> = self
            .environments
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let marker = if i == self.selected_env_idx { ">" } else { " " };
                let is_saved = e.id == self.saved_environment_id;
                let badge = if is_saved && e.is_default {
                    "● default"
                } else if is_saved {
                    "●"
                } else if e.is_default {
                    "default"
                } else {
                    ""
                };
                let style = if i == self.selected_env_idx {
                    theme::title()
                } else {
                    theme::normal()
                };
                Row::new(vec![
                    marker.to_string(),
                    e.name.clone(),
                    e.slug.clone(),
                    badge.to_string(),
                ])
                .style(style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(2),
                Constraint::Min(15),
                Constraint::Min(12),
                Constraint::Length(8),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border()),
        );

        frame.render_widget(table, table_area);

        // Instructions
        let instructions = Paragraph::new(Line::from(vec![
            Span::styled("j/k", theme::title()),
            Span::styled(" navigate  ", theme::dim()),
            Span::styled("Enter", theme::title()),
            Span::styled(" select  ", theme::dim()),
            Span::styled("Esc", theme::title()),
            Span::styled(" back", theme::dim()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[6]);
    }
}
