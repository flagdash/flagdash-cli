pub mod confirm_dialog;
pub mod environment_switcher;
pub mod header;
pub mod input_field;
pub mod search_bar;
pub mod sidebar;
pub mod status_bar;
pub mod table_view;
pub mod text_area;
pub mod toast;

use crate::action::Action;
use crate::event::Event;
use anyhow::Result;
use ratatui::layout::Rect;
use ratatui::Frame;

/// Trait for all TUI components. Each component handles events,
/// processes actions, and renders itself.
pub trait Component {
    /// Initialize the component (called once when first shown).
    fn init(&mut self) -> Result<Option<Action>> {
        Ok(None)
    }

    /// Handle a terminal event (key press, resize, tick).
    /// Returns an optional action to dispatch.
    fn handle_event(&mut self, event: &Event) -> Result<Option<Action>> {
        let _ = event;
        Ok(None)
    }

    /// Process an action from the action bus.
    /// Returns an optional follow-up action.
    fn update(&mut self, action: &Action) -> Result<Option<Action>> {
        let _ = action;
        Ok(None)
    }

    /// Render the component into the given area.
    fn render(&self, frame: &mut Frame, area: Rect);
}
