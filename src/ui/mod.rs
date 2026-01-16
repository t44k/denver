mod components;
mod styles;
mod views;

use ratatui::Frame;

use crate::app::AppState;

pub use styles::*;

/// Main render function
pub fn render(frame: &mut Frame, state: &AppState) {
    use ratatui::layout::{Constraint, Layout};

    let chunks = Layout::vertical([
        Constraint::Length(3),  // Header
        Constraint::Min(0),     // Main content
        Constraint::Length(1),  // Footer
    ])
    .split(frame.area());

    // Render header
    components::header::render(frame, chunks[0], state);

    // Render main view
    match state.current_view {
        crate::app::View::ProjectList => {
            views::project_list::render(frame, chunks[1], state);
        }
        crate::app::View::ProjectDetail => {
            views::project_detail::render(frame, chunks[1], state);
        }
        crate::app::View::KeyEditor => {
            views::key_editor::render(frame, chunks[1], state);
        }
    }

    // Render footer
    components::footer::render(frame, chunks[2], state);

    // Render dialog if present
    if let Some(ref dialog) = state.dialog {
        components::dialog::render(frame, dialog, state);
    }

    // Render help overlay if showing
    if state.show_help {
        components::help::render(frame);
    }
}
