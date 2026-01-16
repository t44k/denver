use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::styles::*;

pub fn render(frame: &mut Frame) {
    let area = centered_rect(70, 80, frame.area());

    // Clear the area behind
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(" Help - Key Bindings ", style_title()))
        .borders(Borders::ALL)
        .border_style(style_border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let help_text = vec![
        Line::from(vec![Span::styled("Navigation", style_header())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  j/Down  ", style_key()),
            Span::styled("Move down", style_muted()),
        ]),
        Line::from(vec![
            Span::styled("  k/Up    ", style_key()),
            Span::styled("Move up", style_muted()),
        ]),
        Line::from(vec![
            Span::styled("  h/Left  ", style_key()),
            Span::styled("Previous environment", style_muted()),
        ]),
        Line::from(vec![
            Span::styled("  l/Right ", style_key()),
            Span::styled("Next environment", style_muted()),
        ]),
        Line::from(vec![
            Span::styled("  Enter   ", style_key()),
            Span::styled("Select/Edit", style_muted()),
        ]),
        Line::from(vec![
            Span::styled("  Esc     ", style_key()),
            Span::styled("Go back/Cancel", style_muted()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Actions", style_header())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  a       ", style_key()),
            Span::styled("Add new key", style_muted()),
        ]),
        Line::from(vec![
            Span::styled("  d       ", style_key()),
            Span::styled("Delete key", style_muted()),
        ]),
        Line::from(vec![
            Span::styled("  y       ", style_key()),
            Span::styled("Copy value to .env (yank)", style_muted()),
        ]),
        Line::from(vec![
            Span::styled("  S       ", style_key()),
            Span::styled("Bulk switch - replace .env with env file", style_muted()),
        ]),
        Line::from(vec![
            Span::styled("  Tab     ", style_key()),
            Span::styled("Select env (for key or entire section)", style_muted()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Reordering", style_header())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Shift+Up   ", style_key()),
            Span::styled("Move item up (within same level)", style_muted()),
        ]),
        Line::from(vec![
            Span::styled("  Shift+Down ", style_key()),
            Span::styled("Move item down (within same level)", style_muted()),
        ]),
        Line::from(vec![
            Span::styled("  Shift+Right", style_key()),
            Span::styled("Move key into section above", style_muted()),
        ]),
        Line::from(vec![
            Span::styled("  Shift+Left ", style_key()),
            Span::styled("Move key out of section", style_muted()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("File Operations", style_header())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  s       ", style_key()),
            Span::styled("Save all changes", style_muted()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Other", style_header())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ?       ", style_key()),
            Span::styled("Toggle this help", style_muted()),
        ]),
        Line::from(vec![
            Span::styled("  q       ", style_key()),
            Span::styled("Quit", style_muted()),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+c  ", style_key()),
            Span::styled("Force quit", style_muted()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press any key to close", style_muted()),
        ]),
    ];

    let help = Paragraph::new(help_text);
    frame.render_widget(help, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [area] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .areas(area);
    area
}
