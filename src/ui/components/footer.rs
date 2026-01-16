use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{AppState, InputMode, MessageLevel, Section, View};
use crate::ui::styles::*;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    // Show message if present
    if let Some(ref message) = state.message {
        let style = match message.level {
            MessageLevel::Info => style_muted(),
            MessageLevel::Warning => style_warning(),
            MessageLevel::Error => style_error(),
            MessageLevel::Success => style_success(),
        };
        let msg = Paragraph::new(Span::styled(&message.text, style));
        frame.render_widget(msg, area);
        return;
    }

    // Show context-sensitive keybindings
    let hints = if state.dialog.is_some() {
        vec![
            ("Enter", "Confirm"),
            ("Esc", "Cancel"),
            ("Tab", "Switch field"),
        ]
    } else if state.input_mode == InputMode::Editing {
        vec![("Enter", "Save"), ("Esc", "Cancel")]
    } else {
        match state.current_view {
            View::ProjectList => vec![
                ("j/k", "Navigate"),
                ("Enter", "Select"),
                ("s", "Save all"),
                ("q", "Quit"),
            ],
            View::ProjectDetail => {
                match &state.selected_section {
                    Section::CurrentConfig => vec![
                        ("j/k", "Navigate"),
                        ("h/l", "Cycle env"),
                        ("Tab", "Next section"),
                        ("Enter", "Edit"),
                        ("a", "Add"),
                        ("d", "Delete"),
                        ("S", "Bulk switch"),
                        ("s", "Save"),
                        ("Esc", "Back"),
                    ],
                    Section::MissingFrom(_) => vec![
                        ("j/k", "Navigate"),
                        ("Tab", "Next section"),
                        ("Enter", "Add to .env"),
                        ("Esc", "Back"),
                    ],
                }
            }
            View::KeyEditor => vec![
                ("j/k", "Select env"),
                ("Enter", "Edit value"),
                ("s", "Save"),
                ("Esc", "Back"),
            ],
        }
    };

    let spans: Vec<Span> = hints
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(format!(" {} ", key), style_header()),
                Span::styled(format!("{} ", desc), style_muted()),
            ]
        })
        .collect();

    let footer = Paragraph::new(Line::from(spans));
    frame.render_widget(footer, area);
}
