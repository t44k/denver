use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
    Frame,
};

use crate::app::{AppState, InputMode};
use crate::models::EnvironmentType;
use crate::ui::styles::*;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(project) = state.current_project() else {
        return;
    };

    let Some(ref key) = state.editing_key else {
        return;
    };

    let block = Block::default()
        .title(Span::styled(format!(" Edit: {} ", key), style_header()))
        .borders(Borders::ALL)
        .border_style(style_border_focused())
        .padding(Padding::uniform(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Get all named envs plus a "custom" option
    let env_types = project.named_env_types();
    let slot_count = env_types.len() + 1; // +1 for custom

    // Create constraints for each slot
    let constraints: Vec<Constraint> = (0..slot_count)
        .map(|_| Constraint::Length(4))
        .collect();

    let chunks = Layout::vertical(constraints).split(inner);

    // Render each env slot
    for (idx, env_type) in env_types.iter().enumerate() {
        let is_selected = idx == state.selected_key_index;
        let value = project.get_value(key, env_type).unwrap_or("");

        render_env_slot(
            frame,
            chunks[idx],
            &env_type.filename(),
            value,
            is_selected,
            state.input_mode == InputMode::Editing && is_selected,
            &state.input_buffer,
        );
    }

    // Render custom slot (last)
    let custom_idx = env_types.len();
    let is_custom_selected = custom_idx == state.selected_key_index;
    let current_value = project.get_value(key, &EnvironmentType::Default).unwrap_or("");

    // Check if current value is custom (doesn't match any env)
    let is_custom = project.find_matching_env(key).is_none();
    let custom_label = if is_custom {
        "custom (current)"
    } else {
        "custom"
    };

    render_env_slot(
        frame,
        chunks[custom_idx],
        custom_label,
        current_value,
        is_custom_selected,
        state.input_mode == InputMode::Editing && is_custom_selected,
        &state.input_buffer,
    );
}

fn render_env_slot(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    is_selected: bool,
    is_editing: bool,
    input_buffer: &str,
) {
    let border_style = if is_selected {
        style_border_focused()
    } else {
        style_border()
    };

    let title_style = if is_selected {
        style_header()
    } else {
        style_muted()
    };

    let display_value = if is_editing {
        format!("{}_", input_buffer)
    } else {
        value.to_string()
    };

    let value_style = if is_editing {
        style_input_active()
    } else if is_selected {
        style_value()
    } else {
        style_muted()
    };

    let block = Block::default()
        .title(Span::styled(format!(" {} ", label), title_style))
        .borders(Borders::ALL)
        .border_style(border_style);

    let content = Paragraph::new(Line::from(vec![Span::styled(display_value, value_style)]))
        .block(block);

    frame.render_widget(content, area);
}
