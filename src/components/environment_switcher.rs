use crate::action::Action;
use crate::api::types::Environment;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub struct EnvironmentSwitcher {
    visible: bool,
    environments: Vec<Environment>,
    selected_idx: usize,
    current_env_id: String,
    loading: bool,
}

impl EnvironmentSwitcher {
    pub fn new() -> Self {
        Self {
            visible: false,
            environments: Vec::new(),
            selected_idx: 0,
            current_env_id: String::new(),
            loading: false,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self, current_env_id: &str) {
        self.visible = true;
        self.loading = true;
        self.current_env_id = current_env_id.to_string();
        self.environments.clear();
        self.selected_idx = 0;
    }

    pub fn set_environments(&mut self, environments: Vec<Environment>) {
        // Pre-select the current environment
        self.selected_idx = environments
            .iter()
            .position(|e| e.id == self.current_env_id)
            .unwrap_or(0);
        self.environments = environments;
        self.loading = false;
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        if !self.visible {
            return None;
        }

        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }

            if self.loading {
                if key.code == KeyCode::Esc {
                    self.visible = false;
                    return Some(Action::EnvironmentSwitcherDismissed);
                }
                return None;
            }

            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected_idx > 0 {
                        self.selected_idx -= 1;
                    }
                    None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.environments.is_empty()
                        && self.selected_idx < self.environments.len() - 1
                    {
                        self.selected_idx += 1;
                    }
                    None
                }
                KeyCode::Enter => {
                    if let Some(env) = self.environments.get(self.selected_idx) {
                        let env_id = env.id.clone();
                        let env_name = env.name.clone();
                        self.visible = false;
                        Some(Action::EnvironmentSwitched {
                            environment_id: env_id,
                            environment_name: env_name,
                        })
                    } else {
                        None
                    }
                }
                KeyCode::Esc => {
                    self.visible = false;
                    Some(Action::EnvironmentSwitcherDismissed)
                }
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let height = if self.loading {
            5
        } else {
            // header(1) + border(2) + rows + instructions(1) + padding(1)
            (self.environments.len() as u16 + 5).min(area.height - 4)
        };
        let width = 50u16.min(area.width - 4);
        let dialog_area = centered_rect(width, height, area);

        let block = Block::default()
            .title(" Switch Environment ")
            .title_style(theme::heading())
            .borders(Borders::ALL)
            .border_style(theme::active_border());

        frame.render_widget(Clear, dialog_area);
        frame.render_widget(block, dialog_area);

        let inner = Rect {
            x: dialog_area.x + 2,
            y: dialog_area.y + 1,
            width: dialog_area.width.saturating_sub(4),
            height: dialog_area.height.saturating_sub(2),
        };

        if self.loading {
            let loading = Paragraph::new(Line::from(Span::styled("Loading...", theme::dim())))
                .alignment(Alignment::Center);
            frame.render_widget(loading, inner);
            return;
        }

        if self.environments.is_empty() {
            let empty = Paragraph::new(Line::from(Span::styled(
                "No environments found",
                theme::dim(),
            )))
            .alignment(Alignment::Center);
            frame.render_widget(empty, inner);
            return;
        }

        let chunks = Layout::vertical([
            Constraint::Min(1),    // environment list
            Constraint::Length(1), // instructions
        ])
        .split(inner);

        // Environment list
        let rows: Vec<Line> = self
            .environments
            .iter()
            .enumerate()
            .map(|(i, env)| {
                let is_current = env.id == self.current_env_id;
                let is_selected = i == self.selected_idx;

                let marker = if is_selected { ">" } else { " " };
                let current_badge = if is_current { " ●" } else { "" };

                let style = if is_selected {
                    theme::title()
                } else if is_current {
                    theme::highlight()
                } else {
                    theme::normal()
                };

                Line::from(vec![
                    Span::styled(format!(" {} ", marker), style),
                    Span::styled(env.name.clone(), style),
                    Span::styled(current_badge, theme::status_on()),
                ])
            })
            .collect();

        let list = Paragraph::new(rows);
        frame.render_widget(list, chunks[0]);

        // Instructions
        let instructions = Paragraph::new(Line::from(vec![
            Span::styled("j/k", theme::title()),
            Span::styled(" navigate  ", theme::dim()),
            Span::styled("Enter", theme::title()),
            Span::styled(" select  ", theme::dim()),
            Span::styled("Esc", theme::title()),
            Span::styled(" cancel", theme::dim()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[1]);
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
