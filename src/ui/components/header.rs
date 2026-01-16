use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{AppState, View};
use crate::ui::styles::*;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = match state.current_view {
        View::ProjectList => "DENVER - .env Manager".to_string(),
        View::ProjectDetail => {
            if let Some(project) = state.current_project() {
                format!("DENVER - {}", project.name)
            } else {
                "DENVER".to_string()
            }
        }
        View::KeyEditor => {
            if let Some(key) = state.current_key() {
                format!("DENVER - Edit: {}", key)
            } else {
                "DENVER - Edit".to_string()
            }
        }
    };

    let modified_indicator = if state.has_unsaved_changes() {
        Span::styled(" [modified]", style_modified())
    } else {
        Span::raw("")
    };

    let help_hint = Span::styled(" [?] Help", style_muted());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(title, style_title()),
        modified_indicator,
        Span::raw("  "),
        help_hint,
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(style_border()),
    );

    frame.render_widget(header, area);
}
