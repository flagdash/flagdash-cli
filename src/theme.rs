use ratatui::style::{Color, Modifier, Style};

// Brand colors
pub const PRIMARY: Color = Color::Rgb(0, 200, 200); // Cyan
pub const SECONDARY: Color = Color::Rgb(16, 185, 129); // Emerald
pub const ACCENT: Color = Color::Rgb(139, 92, 246); // Purple
pub const SUCCESS: Color = Color::Rgb(34, 197, 94); // Green
pub const ERROR: Color = Color::Rgb(239, 68, 68); // Red
pub const WARNING: Color = Color::Rgb(245, 158, 11); // Amber
pub const INFO: Color = Color::Rgb(59, 130, 246); // Blue
pub const MUTED: Color = Color::Rgb(107, 114, 128); // Gray
pub const BG: Color = Color::Rgb(15, 15, 20); // Near-black
pub const SURFACE: Color = Color::Rgb(30, 33, 46); // Card/panel background
pub const BORDER: Color = Color::Rgb(55, 55, 70); // Subtle border
pub const TEXT: Color = Color::Rgb(229, 231, 235); // Light text
pub const TEXT_DIM: Color = Color::Rgb(156, 163, 175); // Dimmed text

pub fn title() -> Style {
    Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
}

pub fn heading() -> Style {
    Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
}

pub fn normal() -> Style {
    Style::default().fg(TEXT)
}

pub fn dim() -> Style {
    Style::default().fg(TEXT_DIM)
}

pub fn highlight() -> Style {
    Style::default().bg(Color::Rgb(35, 35, 50)).fg(PRIMARY)
}

pub fn selected() -> Style {
    Style::default().bg(Color::Rgb(30, 30, 45)).fg(TEXT)
}

pub fn status_on() -> Style {
    Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD)
}

pub fn status_off() -> Style {
    Style::default().fg(ERROR)
}

pub fn badge_management() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn badge_server() -> Style {
    Style::default().fg(INFO).add_modifier(Modifier::BOLD)
}

pub fn badge_client() -> Style {
    Style::default().fg(SECONDARY).add_modifier(Modifier::BOLD)
}

pub fn border() -> Style {
    Style::default().fg(BORDER)
}

pub fn active_border() -> Style {
    Style::default().fg(PRIMARY)
}

pub const LOGO: &str = r#"
  ╔═╗╦  ╔═╗╔═╗╔╦╗╔═╗╔═╗╦ ╦
  ╠╣ ║  ╠═╣║ ╦ ║║╠═╣╚═╗╠═╣
  ╚  ╩═╝╩ ╩╚═╝═╩╝╩ ╩╚═╝╩ ╩"#;

pub const LOGO_SMALL: &str = "◆ FlagDash";
