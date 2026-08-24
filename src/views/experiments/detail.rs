use crate::action::{Action, View};
use crate::api::types::ManagedExperiment;
use crate::config::KeyTier;
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub struct ExperimentDetailView {
    pub experiment: Option<ManagedExperiment>,
    pub key_tier: KeyTier,
}

impl ExperimentDetailView {
    pub fn new(key_tier: KeyTier) -> Self {
        Self {
            experiment: None,
            key_tier,
        }
    }

    pub fn handle_event(&self, event: &Event) -> Option<Action> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }
            let experiment = self.experiment.as_ref()?;
            match key.code {
                KeyCode::Esc | KeyCode::Backspace => {
                    return Some(Action::Navigate(View::ExperimentList));
                }
                KeyCode::Char('e') if self.key_tier.can_mutate() => {
                    return Some(Action::Navigate(View::ExperimentEdit(
                        experiment.key.clone(),
                    )));
                }
                _ => {}
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let Some(experiment) = &self.experiment else {
            frame.render_widget(Paragraph::new("Loading...").style(theme::dim()), area);
            return;
        };

        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(8),
            Constraint::Min(7),
            Constraint::Length(1),
        ])
        .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("← ", theme::dim()),
                Span::styled(&experiment.name, theme::heading()),
                Span::styled(format!("  ({})", experiment.key), theme::dim()),
            ])),
            chunks[0],
        );

        let details = vec![
            Line::from(vec![
                Span::styled("Status: ", theme::dim()),
                Span::styled(&experiment.status, theme::normal()),
            ]),
            Line::from(vec![
                Span::styled("Randomization: ", theme::dim()),
                Span::styled(&experiment.randomization_unit, theme::normal()),
            ]),
            Line::from(vec![
                Span::styled("Layer: ", theme::dim()),
                Span::styled(value_or_dash(&experiment.layer_key), theme::normal()),
            ]),
            Line::from(vec![
                Span::styled("Hypothesis: ", theme::dim()),
                Span::styled(value_or_dash(&experiment.hypothesis), theme::normal()),
            ]),
            Line::from(vec![
                Span::styled("Decision: ", theme::dim()),
                Span::styled(value_or_dash(&experiment.decision), theme::normal()),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(details).block(
                Block::default()
                    .title(" Experiment brief ")
                    .title_style(theme::heading())
                    .borders(Borders::ALL)
                    .border_style(theme::border()),
            ),
            chunks[1],
        );

        let evidence = format!(
            "Variants\n{}\n\nMetrics\n{}\n\nDecision notes\n{}",
            pretty(&experiment.variants),
            pretty(&experiment.metrics),
            value_or_dash(&experiment.decision_notes)
        );
        frame.render_widget(
            Paragraph::new(evidence).wrap(Wrap { trim: false }).block(
                Block::default()
                    .title(" Configuration & evidence ")
                    .title_style(theme::heading())
                    .borders(Borders::ALL)
                    .border_style(theme::border()),
            ),
            chunks[2],
        );

        let mut help = vec![
            Span::styled("[Esc]", theme::title()),
            Span::styled("Back ", theme::dim()),
        ];
        if self.key_tier.can_mutate() {
            help.extend([
                Span::styled("[e]", theme::title()),
                Span::styled("Edit", theme::dim()),
            ]);
        }
        frame.render_widget(Paragraph::new(Line::from(help)), chunks[3]);
    }
}

fn value_or_dash(value: &str) -> &str {
    if value.is_empty() {
        "—"
    } else {
        value
    }
}

fn pretty(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}
