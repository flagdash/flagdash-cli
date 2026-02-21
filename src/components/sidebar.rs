use crate::action::{Action, SidebarSection};
use crate::event::Event;
use crate::theme;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Tabs};
use ratatui::Frame;

const SECTIONS: &[SidebarSection] = &[
    SidebarSection::Dashboard,
    SidebarSection::Flags,
    SidebarSection::Configs,
    SidebarSection::AiConfigs,
    SidebarSection::Webhooks,
    SidebarSection::Environments,
];

const TAB_TITLES: &[&str] = &[
    "[1] Dashboard",
    "[2] Flags",
    "[3] Config",
    "[4] AI Config",
    "[5] Webhooks",
    "[6] Environments",
];

pub struct Sidebar {
    pub selected: SidebarSection,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            selected: SidebarSection::Dashboard,
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return None;
            }
            match key.code {
                KeyCode::Char('1') => return self.select_index(0),
                KeyCode::Char('2') => return self.select_index(1),
                KeyCode::Char('3') => return self.select_index(2),
                KeyCode::Char('4') => return self.select_index(3),
                KeyCode::Char('5') => return self.select_index(4),
                KeyCode::Char('6') => return self.select_index(5),
                KeyCode::Left => return self.select_prev(),
                KeyCode::Right => return self.select_next(),
                _ => {}
            }
        }
        None
    }

    fn select_index(&mut self, idx: usize) -> Option<Action> {
        if idx < SECTIONS.len() {
            self.selected = SECTIONS[idx].clone();
            Some(Action::SelectSection(self.selected.clone()))
        } else {
            None
        }
    }

    fn select_prev(&mut self) -> Option<Action> {
        let idx = SECTIONS
            .iter()
            .position(|s| *s == self.selected)
            .unwrap_or(0);
        let new_idx = if idx == 0 {
            SECTIONS.len() - 1
        } else {
            idx - 1
        };
        self.select_index(new_idx)
    }

    fn select_next(&mut self) -> Option<Action> {
        let idx = SECTIONS
            .iter()
            .position(|s| *s == self.selected)
            .unwrap_or(0);
        let new_idx = (idx + 1) % SECTIONS.len();
        self.select_index(new_idx)
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let selected_idx = SECTIONS
            .iter()
            .position(|s| *s == self.selected)
            .unwrap_or(0);

        let tabs = Tabs::new(TAB_TITLES.to_vec())
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(theme::border()),
            )
            .select(selected_idx)
            .style(theme::dim())
            .highlight_style(
                Style::default()
                    .fg(theme::SUCCESS)
                    .bg(Color::Rgb(15, 40, 30))
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )
            .divider("    ")
            .padding("   ", "   ");

        frame.render_widget(tabs, area);
    }
}
