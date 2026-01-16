use ratatui::style::{Color, Modifier, Style};

// Colors
pub const PRIMARY: Color = Color::Cyan;
pub const SECONDARY: Color = Color::Yellow;
pub const SUCCESS: Color = Color::Green;
pub const WARNING: Color = Color::Yellow;
pub const ERROR: Color = Color::Red;
pub const MUTED: Color = Color::DarkGray;
pub const HIGHLIGHT_BG: Color = Color::Rgb(40, 40, 60);

// Styles
pub fn style_normal() -> Style {
    Style::default()
}

pub fn style_selected() -> Style {
    Style::default()
        .bg(HIGHLIGHT_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn style_header() -> Style {
    Style::default()
        .fg(PRIMARY)
        .add_modifier(Modifier::BOLD)
}

pub fn style_title() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn style_key() -> Style {
    Style::default().fg(Color::White)
}

pub fn style_value() -> Style {
    Style::default().fg(Color::Gray)
}

pub fn style_modified() -> Style {
    Style::default()
        .fg(WARNING)
        .add_modifier(Modifier::ITALIC)
}

pub fn style_success() -> Style {
    Style::default().fg(SUCCESS)
}

pub fn style_error() -> Style {
    Style::default().fg(ERROR)
}

pub fn style_warning() -> Style {
    Style::default().fg(WARNING)
}

pub fn style_muted() -> Style {
    Style::default().fg(MUTED)
}

pub fn style_input() -> Style {
    Style::default()
        .fg(Color::White)
        .bg(Color::Rgb(30, 30, 40))
}

pub fn style_input_active() -> Style {
    Style::default()
        .fg(Color::White)
        .bg(Color::Rgb(40, 40, 60))
        .add_modifier(Modifier::BOLD)
}

pub fn style_border() -> Style {
    Style::default().fg(Color::Rgb(80, 80, 100))
}

pub fn style_border_focused() -> Style {
    Style::default().fg(PRIMARY)
}

pub fn style_env_separator() -> Style {
    Style::default().fg(Color::Magenta)
}

pub fn style_section_header() -> Style {
    Style::default()
        .fg(Color::Rgb(180, 180, 220))
        .add_modifier(Modifier::ITALIC)
}

pub fn style_duplicated() -> Style {
    Style::default()
        .fg(ERROR)
        .add_modifier(Modifier::BOLD)
}
