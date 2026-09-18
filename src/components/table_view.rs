use crate::theme;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};
use ratatui::Frame;

pub struct TableView {
    pub state: TableState,
    pub row_count: usize,
}

impl TableView {
    pub fn new() -> Self {
        Self {
            state: TableState::default(),
            row_count: 0,
        }
    }

    pub fn select_next(&mut self) {
        if self.row_count == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => (i + 1) % self.row_count,
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn select_prev(&mut self) {
        if self.row_count == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.row_count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn set_items(&mut self, count: usize) {
        self.row_count = count;
        if count == 0 {
            self.state.select(None);
        } else if self.state.selected().is_none() {
            self.state.select(Some(0));
        } else if let Some(i) = self.state.selected() {
            if i >= count {
                self.state.select(Some(count - 1));
            }
        }
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        title: &str,
        headers: &[&str],
        widths: &[Constraint],
        rows: Vec<Vec<String>>,
    ) {
        let header_cells: Vec<Cell> = headers
            .iter()
            .map(|h| Cell::from(*h).style(theme::heading()))
            .collect();
        let header = Row::new(header_cells).height(1);

        let table_rows: Vec<Row> = rows
            .iter()
            .map(|row| {
                let cells: Vec<Cell> = row
                    .iter()
                    .map(|c| Cell::from(c.as_str()).style(theme::normal()))
                    .collect();
                Row::new(cells).height(1)
            })
            .collect();

        let block = Block::default()
            .title(format!(" {} ({}) ", title, self.row_count))
            .title_style(theme::heading())
            .borders(Borders::ALL)
            .border_style(theme::border());

        let table = Table::new(table_rows, widths)
            .header(header)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(22, 72, 45))
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ");

        frame.render_stateful_widget(table, area, &mut self.state);
    }
}
