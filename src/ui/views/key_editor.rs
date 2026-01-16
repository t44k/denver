use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
    Frame,
};

use crate::app::{AppState, InputMode};
use crate::models::EnvironmentType;
use crate::ui::styles::*;

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let Some(project) = state.current_project() else {
        return;
    };

    let Some(ref key) = state.editing_key else {
        return;
    };

    // Determine target filename (output filename or default .env)
    let target_name = state
        .output_filename
        .as_deref()
        .unwrap_or(".env");

    // Build title with target indicator in green
    let title = Line::from(vec![
        Span::styled(format!(" Edit: {} ", key), style_header()),
        Span::styled("| Target: ", style_muted()),
        Span::styled(format!("{} ", target_name), style_target()),
    ]);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(style_border_focused())
        .padding(Padding::uniform(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Get all named envs: target (.env) first, then named envs
    let env_types = project.named_env_types();
    let slot_count = 1 + env_types.len(); // target + named envs

    // Create constraints for each slot
    let constraints: Vec<Constraint> = (0..slot_count)
        .map(|_| Constraint::Length(4))
        .collect();

    let chunks = Layout::vertical(constraints).split(inner);

    // Render target slot first (index 0)
    let is_target_selected = state.selected_key_index == 0;
    let target_value = project.get_value(key, &EnvironmentType::Default);
    let is_inactive = target_value.is_none();
    let target_label = if is_inactive {
        format!("{} (disabled)", target_name)
    } else {
        format!("{} (target)", target_name)
    };

    render_env_slot(
        frame,
        chunks[0],
        &target_label,
        target_value.unwrap_or("(not set)"),
        is_target_selected,
        state.input_mode == InputMode::Editing && is_target_selected,
        &state.input_buffer,
        true, // is_target
        is_inactive,
    );

    // Render each named env slot (indices 1..n)
    for (idx, env_type) in env_types.iter().enumerate() {
        let slot_idx = idx + 1; // offset by 1 for target slot
        let is_selected = slot_idx == state.selected_key_index;
        let value = project.get_value(key, env_type).unwrap_or("");

        render_env_slot(
            frame,
            chunks[slot_idx],
            &env_type.filename(),
            value,
            is_selected,
            state.input_mode == InputMode::Editing && is_selected,
            &state.input_buffer,
            false, // not target
            false, // not inactive (for named envs)
        );
    }
}

fn render_env_slot(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    is_selected: bool,
    is_editing: bool,
    input_buffer: &str,
    is_target: bool,
    is_inactive: bool,
) {
    let border_style = if is_inactive && is_target {
        // Inactive target: show in muted style
        if is_selected {
            style_warning()
        } else {
            style_muted()
        }
    } else if is_target {
        if is_selected {
            style_target_selected()
        } else {
            style_target()
        }
    } else if is_selected {
        style_border_focused()
    } else {
        style_border()
    };

    let title_style = if is_inactive && is_target {
        if is_selected {
            style_warning()
        } else {
            style_muted()
        }
    } else if is_target {
        if is_selected {
            style_target_selected()
        } else {
            style_target()
        }
    } else if is_selected {
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
    } else if is_inactive && is_target && !is_editing {
        style_muted() // Inactive value shown in muted
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
