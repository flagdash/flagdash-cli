use crate::action::{Action, DashboardData, DashboardFlag, View};
use crate::event::Event;
use crate::theme;
use chrono::Utc;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub struct DashboardView {
    pub data: Option<DashboardData>,
    selected_row: usize,
}

impl DashboardView {
    pub fn new() -> Self {
        Self {
            data: None,
            selected_row: 0,
        }
    }

    pub fn handle_event(&self, event: &Event) -> Option<Action> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }
            if let Some(data) = &self.data {
                if !data.recent_flags.is_empty() {
                    if let KeyCode::Enter = key.code {
                        if let Some(flag) = data.recent_flags.get(self.selected_row) {
                            return Some(Action::Navigate(View::FlagDetail(flag.key.clone())));
                        }
                    }
                }
            }
        }
        None
    }

    pub fn select_next(&mut self) {
        if let Some(data) = &self.data {
            if !data.recent_flags.is_empty() {
                self.selected_row = (self.selected_row + 1) % data.recent_flags.len();
            }
        }
    }

    pub fn select_prev(&mut self) {
        if let Some(data) = &self.data {
            if !data.recent_flags.is_empty() {
                if self.selected_row == 0 {
                    self.selected_row = data.recent_flags.len() - 1;
                } else {
                    self.selected_row -= 1;
                }
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(2),  // top margin
            Constraint::Length(10), // stat cards
            Constraint::Length(2),  // gap
            Constraint::Min(0),     // recent flags
        ])
        .split(area);

        if let Some(data) = &self.data {
            // ── Stat cards ────────────────────────────────────────────
            let cards = Layout::horizontal([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .spacing(1)
            .split(chunks[1]);

            render_stat_card(
                frame,
                cards[0],
                "FLAGS",
                data.flag_count,
                &data.flag_subtitle,
                theme::SUCCESS,
            );
            render_stat_card(
                frame,
                cards[1],
                "CONFIGS",
                data.config_count,
                &data.config_subtitle,
                theme::INFO,
            );
            render_stat_card(
                frame,
                cards[2],
                "AI CONFIGS",
                data.ai_config_count,
                &data.ai_config_subtitle,
                theme::ACCENT,
            );
            render_stat_card(
                frame,
                cards[3],
                "WEBHOOKS",
                data.webhook_count,
                &data.webhook_subtitle,
                theme::WARNING,
            );

            // ── Recent flags table ────────────────────────────────────
            if !data.recent_flags.is_empty() {
                render_recent_flags(frame, chunks[3], &data.recent_flags, self.selected_row);
            }
        } else {
            let loading = Paragraph::new(Line::from(Span::styled("Loading...", theme::dim())))
                .alignment(Alignment::Center);
            frame.render_widget(loading, chunks[1]);
        }
    }
}

fn render_stat_card(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    count: usize,
    subtitle: &str,
    color: Color,
) {
    // Border only — no bg on the block itself (avoids bg bleeding into border cells)
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Fill inner area (inside border only) with a dark tint of the card's accent color
    let card_bg = match color {
        Color::Rgb(r, g, b) => Color::Rgb(15 + r / 12, 15 + g / 12, 15 + b / 12),
        _ => theme::SURFACE,
    };
    frame.render_widget(Block::default().style(Style::default().bg(card_bg)), inner);

    if inner.width < 4 || inner.height < 6 {
        return;
    }
    let padded = Rect {
        x: inner.x + 2,
        y: inner.y + 1,
        width: inner.width.saturating_sub(3),
        height: inner.height.saturating_sub(2),
    };

    let content = Layout::vertical([
        Constraint::Length(1), // label (small)
        Constraint::Length(1), // gap
        Constraint::Length(2), // number (big, 2 rows for visual weight)
        Constraint::Length(1), // subtitle (small)
        Constraint::Min(0),    // bottom padding
    ])
    .split(padded);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(label, theme::dim()))),
        content[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            count.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))),
        content[2],
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(subtitle, theme::dim()))),
        content[3],
    );
}

fn render_recent_flags(frame: &mut Frame, area: Rect, flags: &[DashboardFlag], selected: usize) {
    let rows = Layout::vertical([
        Constraint::Length(1), // section header
        Constraint::Length(1), // gap
        Constraint::Length(1), // column headers
        Constraint::Min(0),    // flag rows
    ])
    .split(area);

    // Section header
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::styled("RECENT FLAGS", theme::dim()),
            Span::styled("  ›", theme::dim()),
        ])),
        rows[0],
    );

    // Column widths — shared between header and rows
    let col_widths = [
        Constraint::Length(32), // dot + key
        Constraint::Length(14), // type
        Constraint::Length(12), // rollout
        Constraint::Min(0),     // value
        Constraint::Length(14), // updated (right-aligned)
    ];

    // Column headers
    let col_chunks = Layout::horizontal(col_widths).split(rows[2]);

    let col_style = Style::default().fg(theme::MUTED);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled("     KEY", col_style))),
        col_chunks[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled("TYPE", col_style))),
        col_chunks[1],
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled("ROLLOUT", col_style))),
        col_chunks[2],
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled("VALUE", col_style))),
        col_chunks[3],
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled("UPDATED", col_style))).alignment(Alignment::Right),
        col_chunks[4],
    );

    // Flag rows (single line each)
    let flags_area = rows[3];
    let max_rows = flags_area.height as usize;

    for (i, flag) in flags.iter().enumerate().take(max_rows) {
        let row_y = flags_area.y + i as u16;
        let row_area = Rect {
            x: flags_area.x,
            y: row_y,
            width: flags_area.width,
            height: 1,
        };

        let is_selected = i == selected;
        let bg = if is_selected {
            Color::Rgb(22, 72, 45)
        } else {
            theme::BG
        };
        let row_style = Style::default().bg(bg);

        // Fill row with bg
        frame.render_widget(Block::default().style(row_style), row_area);

        let content_rect = row_area;
        let col_chunks = Layout::horizontal(col_widths).split(content_rect);

        let (dot, dot_color) = flag_dot(flag);
        let key_color = if flag.enabled {
            if is_selected {
                theme::SUCCESS
            } else {
                theme::TEXT
            }
        } else {
            theme::TEXT_DIM
        };

        let rollout_str = match flag.rollout {
            Some(p) if p > 0 => format!("{}%", p),
            _ => "—".to_string(),
        };
        let rollout_color = match flag.rollout {
            Some(100) => theme::SUCCESS,
            Some(p) if p > 0 => theme::WARNING,
            _ => theme::TEXT_DIM,
        };

        let value_color = if flag.value == "true" {
            theme::SUCCESS
        } else {
            theme::TEXT_DIM
        };

        // Dot + Key
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  ", row_style),
                Span::styled(dot, Style::default().fg(dot_color).bg(bg)),
                Span::styled(" ", row_style),
                Span::styled(
                    &flag.key,
                    Style::default()
                        .fg(key_color)
                        .bg(bg)
                        .add_modifier(if is_selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
            ]))
            .style(row_style),
            col_chunks[0],
        );

        // Type
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                &flag.flag_type,
                Style::default().fg(theme::TEXT_DIM).bg(bg),
            )))
            .style(row_style),
            col_chunks[1],
        );

        // Rollout
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                rollout_str,
                Style::default().fg(rollout_color).bg(bg),
            )))
            .style(row_style),
            col_chunks[2],
        );

        // Value
        let value_display = truncate_str(&flag.value, col_chunks[3].width as usize);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                value_display,
                Style::default().fg(value_color).bg(bg),
            )))
            .style(row_style),
            col_chunks[3],
        );

        // Updated (right-aligned)
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                relative_time(&flag.updated_at),
                Style::default().fg(theme::TEXT_DIM).bg(bg),
            )))
            .alignment(Alignment::Right)
            .style(row_style),
            col_chunks[4],
        );
    }
}

fn flag_dot(flag: &DashboardFlag) -> (&'static str, Color) {
    if !flag.enabled {
        return ("●", theme::MUTED);
    }
    match flag.rollout {
        Some(p) if p > 0 && p < 100 => ("●", theme::WARNING),
        _ => ("●", theme::SUCCESS),
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    let max = max.saturating_sub(1);
    if s.chars().count() > max && max > 1 {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    } else {
        s.to_string()
    }
}

fn relative_time(dt: &chrono::DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(*dt);
    let secs = diff.num_seconds().max(0);
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}
