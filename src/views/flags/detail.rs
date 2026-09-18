use crate::action::{Action, View};
use crate::api::types::ManagedFlag;
use crate::config::KeyTier;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

pub struct FlagDetailView {
    pub flag: Option<ManagedFlag>,
    pub key_tier: KeyTier,
}

impl FlagDetailView {
    pub fn new(key_tier: KeyTier) -> Self {
        Self {
            flag: None,
            key_tier,
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }
            let flag = self.flag.as_ref()?;
            match key.code {
                KeyCode::Esc | KeyCode::Backspace => {
                    return Some(Action::Navigate(View::FlagList));
                }
                KeyCode::Char('e') if self.key_tier.can_mutate() => {
                    return Some(Action::Navigate(View::FlagEdit(flag.key.clone())));
                }
                KeyCode::Char('t') if self.key_tier.can_mutate() => {
                    return Some(Action::Navigate(View::FlagToggle(flag.key.clone())));
                }
                KeyCode::Char('r') if self.key_tier.can_mutate() => {
                    return Some(Action::Navigate(View::FlagRollout(flag.key.clone())));
                }
                KeyCode::Char('u') if self.key_tier.can_mutate() => {
                    return Some(Action::Navigate(View::FlagRules(flag.key.clone())));
                }
                KeyCode::Char('v') if self.key_tier.can_mutate() => {
                    return Some(Action::Navigate(View::FlagVariations(flag.key.clone())));
                }
                KeyCode::Char('s') => {
                    return Some(Action::Navigate(View::FlagSchedules(flag.key.clone())));
                }
                _ => {}
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let flag = match &self.flag {
            Some(f) => f,
            None => {
                let loading = Paragraph::new("Loading...").style(theme::dim());
                frame.render_widget(loading, area);
                return;
            }
        };

        let chunks = Layout::vertical([
            Constraint::Length(2), // Title
            Constraint::Length(6), // Info
            Constraint::Min(5),    // Environments table
            Constraint::Length(1), // Shortcuts
        ])
        .split(area);

        // Title
        let title = Paragraph::new(Line::from(vec![
            Span::styled("← ", theme::dim()),
            Span::styled(&flag.name, theme::heading()),
            Span::styled(format!("  ({})", flag.key), theme::dim()),
        ]));
        frame.render_widget(title, chunks[0]);

        // Info section
        let info_block = Block::default()
            .title(" Details ")
            .title_style(theme::heading())
            .borders(Borders::ALL)
            .border_style(theme::border());
        let info_inner = info_block.inner(chunks[1]);
        frame.render_widget(info_block, chunks[1]);

        let info_rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(info_inner);

        let info_lines = [
            ("Type: ", flag.flag_type.as_str()),
            ("Default: ", &format!("{}", flag.default_value)),
            (
                "Description: ",
                if flag.description.is_empty() {
                    "—"
                } else {
                    &flag.description
                },
            ),
        ];

        for (i, (label, value)) in info_lines.iter().enumerate() {
            if i < info_rows.len() {
                let line = Paragraph::new(Line::from(vec![
                    Span::styled(*label, theme::dim()),
                    Span::styled(*value, theme::normal()),
                ]));
                frame.render_widget(line, info_rows[i]);
            }
        }

        // Environments table
        let env_rows: Vec<Row> = flag
            .environments
            .iter()
            .map(|env| {
                let status = if env.enabled { "ON" } else { "OFF" };
                let status_style = if env.enabled {
                    theme::status_on()
                } else {
                    theme::status_off()
                };
                Row::new(vec![
                    Cell::from(env.environment_id.as_str()).style(theme::normal()),
                    Cell::from(status).style(status_style),
                    Cell::from(format!("{}%", env.rollout_percentage)).style(theme::normal()),
                    Cell::from(format!("{}", env.value)).style(theme::dim()),
                ])
            })
            .collect();

        let env_table = Table::new(
            env_rows,
            [
                Constraint::Percentage(30),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(40),
            ],
        )
        .header(Row::new(vec!["Environment", "Status", "Rollout", "Value"]).style(theme::heading()))
        .block(
            Block::default()
                .title(" Environments ")
                .title_style(theme::heading())
                .borders(Borders::ALL)
                .border_style(theme::border()),
        );
        frame.render_widget(env_table, chunks[2]);

        // Shortcuts
        let mut spans = vec![
            Span::styled("[Esc]", theme::title()),
            Span::styled("Back ", theme::dim()),
        ];
        if self.key_tier.can_mutate() {
            spans.extend([
                Span::styled("[e]", theme::title()),
                Span::styled("Edit ", theme::dim()),
                Span::styled("[t]", theme::title()),
                Span::styled("Toggle ", theme::dim()),
                Span::styled("[r]", theme::title()),
                Span::styled("Rollout ", theme::dim()),
                Span::styled("[u]", theme::title()),
                Span::styled("Rules ", theme::dim()),
                Span::styled("[v]", theme::title()),
                Span::styled("Variations ", theme::dim()),
            ]);
        }
        spans.extend([
            Span::styled("[s]", theme::title()),
            Span::styled("Schedules", theme::dim()),
        ]);
        frame.render_widget(Paragraph::new(Line::from(spans)), chunks[3]);
    }
}
