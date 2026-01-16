use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{AppState, Dialog, EnvOption};
use crate::ui::pagination::{calculate_visible_range, scroll_status};
use crate::ui::styles::*;

pub fn render(frame: &mut Frame, dialog: &Dialog, state: &mut AppState) {
    let area = centered_rect(60, 40, frame.area());

    // Clear the area behind the dialog
    frame.render_widget(Clear, area);

    match dialog {
        Dialog::Confirm { title, message, .. } => {
            render_confirm(frame, area, title, message);
        }
        Dialog::Input { title, prompt, value, cursor_pos, .. } => {
            render_input(frame, area, title, prompt, value, *cursor_pos);
        }
        Dialog::AddKey { key, value, focus_on_value, key_cursor_pos, value_cursor_pos } => {
            render_add_key(frame, area, key, value, *focus_on_value, *key_cursor_pos, *value_cursor_pos);
        }
        Dialog::BulkSwitch { selected_env_index } => {
            render_bulk_switch(frame, area, state, *selected_env_index);
        }
        Dialog::SelectEnv { key, selected_index, current_index, options } => {
            render_select_env(frame, area, key, *selected_index, *current_index, options, state);
        }
        Dialog::SelectSectionEnv { section, selected_index, current_indices, options } => {
            render_select_section_env(frame, area, section, *selected_index, current_indices, options, state);
        }
        Dialog::UnsavedChanges => {
            render_unsaved_changes(frame, area);
        }
    }
}

fn render_confirm(frame: &mut Frame, area: Rect, title: &str, message: &str) {
    let block = Block::default()
        .title(Span::styled(format!(" {} ", title), style_title()))
        .borders(Borders::ALL)
        .border_style(style_border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Min(2),
        Constraint::Length(2),
    ])
    .split(inner);

    let msg = Paragraph::new(message).style(style_normal());
    frame.render_widget(msg, chunks[0]);

    let buttons = Paragraph::new(Line::from(vec![
        Span::styled(" [Enter] ", style_header()),
        Span::raw("Confirm  "),
        Span::styled(" [Esc] ", style_muted()),
        Span::raw("Cancel"),
    ]));
    frame.render_widget(buttons, chunks[1]);
}

fn render_input(frame: &mut Frame, area: Rect, title: &str, prompt: &str, value: &str, cursor_pos: usize) {
    let block = Block::default()
        .title(Span::styled(format!(" {} ", title), style_title()))
        .borders(Borders::ALL)
        .border_style(style_border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .split(inner);

    let prompt_widget = Paragraph::new(prompt).style(style_muted());
    frame.render_widget(prompt_widget, chunks[0]);

    // Insert cursor at correct position
    let before = &value[..cursor_pos];
    let after = &value[cursor_pos..];
    let display_value = format!("{}|{}", before, after);

    let input = Paragraph::new(display_value)
        .style(style_input_active())
        .block(Block::default().borders(Borders::ALL).border_style(style_border()));
    frame.render_widget(input, chunks[1]);
}

fn render_add_key(frame: &mut Frame, area: Rect, key: &str, value: &str, focus_on_value: bool, key_cursor_pos: usize, value_cursor_pos: usize) {
    let block = Block::default()
        .title(Span::styled(" Add New Key ", style_title()))
        .borders(Borders::ALL)
        .border_style(style_border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);

    // Key label
    let key_label = Paragraph::new("Key:").style(style_muted());
    frame.render_widget(key_label, chunks[0]);

    // Key input with cursor at correct position
    let key_style = if !focus_on_value { style_input_active() } else { style_input() };
    let key_border = if !focus_on_value { style_border_focused() } else { style_border() };
    let key_display = if !focus_on_value {
        let before = &key[..key_cursor_pos];
        let after = &key[key_cursor_pos..];
        format!("{}|{}", before, after)
    } else {
        key.to_string()
    };
    let key_input = Paragraph::new(key_display)
        .style(key_style)
        .block(Block::default().borders(Borders::ALL).border_style(key_border));
    frame.render_widget(key_input, chunks[1]);

    // Value label
    let value_label = Paragraph::new("Value:").style(style_muted());
    frame.render_widget(value_label, chunks[2]);

    // Value input with cursor at correct position
    let value_style = if focus_on_value { style_input_active() } else { style_input() };
    let value_border = if focus_on_value { style_border_focused() } else { style_border() };
    let value_display = if focus_on_value {
        let before = &value[..value_cursor_pos];
        let after = &value[value_cursor_pos..];
        format!("{}|{}", before, after)
    } else {
        value.to_string()
    };
    let value_input = Paragraph::new(value_display)
        .style(value_style)
        .block(Block::default().borders(Borders::ALL).border_style(value_border));
    frame.render_widget(value_input, chunks[3]);

    // Hint
    let hint = Paragraph::new(Line::from(vec![
        Span::styled("Tab", style_header()),
        Span::raw(" switch field  "),
        Span::styled("Enter", style_header()),
        Span::raw(" add  "),
        Span::styled("Esc", style_muted()),
        Span::raw(" cancel"),
    ]));
    frame.render_widget(hint, chunks[5]);
}

fn render_bulk_switch(frame: &mut Frame, area: Rect, state: &mut AppState, selected: usize) {
    // Use named_env_types to exclude Default (can't copy .env to itself)
    let env_types: Vec<_> = state
        .current_project()
        .map(|p| p.named_env_types().into_iter().cloned().collect())
        .unwrap_or_default();
    let total_items = env_types.len();

    let scroll_info = scroll_status(
        state.dialog_scroll,
        area.height.saturating_sub(8) as usize,
        total_items,
    );

    let block = Block::default()
        .title(Span::styled(format!(" Bulk Switch - Replace .env{} ", scroll_info), style_title()))
        .borders(Borders::ALL)
        .border_style(style_border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(inner);

    let info = Paragraph::new("Select environment to copy all values from:").style(style_muted());
    frame.render_widget(info, chunks[0]);

    if !env_types.is_empty() {
        let visible_height = chunks[1].height as usize;
        let (start, end) = calculate_visible_range(
            total_items,
            selected,
            &mut state.dialog_scroll,
            visible_height,
        );

        let items: Vec<ListItem> = env_types
            .iter()
            .enumerate()
            .skip(start)
            .take(end - start)
            .map(|(i, env_type)| {
                let style = if i == selected {
                    style_selected()
                } else {
                    style_normal()
                };
                let prefix = if i == selected { "> " } else { "  " };
                ListItem::new(format!("{}{}", prefix, env_type.filename())).style(style)
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, chunks[1]);
    }

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("j/k", style_header()),
        Span::raw(" navigate  "),
        Span::styled("Enter", style_header()),
        Span::raw(" confirm  "),
        Span::styled("Esc", style_muted()),
        Span::raw(" cancel"),
    ]));
    frame.render_widget(hint, chunks[2]);
}

fn render_select_env(frame: &mut Frame, area: Rect, key: &str, selected: usize, current_index: Option<usize>, options: &[EnvOption], state: &mut AppState) {
    let total_items = options.len();
    let scroll_info = scroll_status(
        state.dialog_scroll,
        area.height.saturating_sub(8) as usize,
        total_items,
    );

    let block = Block::default()
        .title(Span::styled(format!(" Select value for: {}{} ", key, scroll_info), style_title()))
        .borders(Borders::ALL)
        .border_style(style_border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(inner);

    let info = Paragraph::new("Choose which environment's value to use:").style(style_muted());
    frame.render_widget(info, chunks[0]);

    let visible_height = chunks[1].height as usize;
    let (start, end) = calculate_visible_range(
        total_items,
        selected,
        &mut state.dialog_scroll,
        visible_height,
    );

    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .skip(start)
        .take(end - start)
        .map(|(i, opt)| {
            let is_selected = i == selected;
            let is_current = current_index == Some(i);

            // Build styled line with colored env name
            let prefix = if is_selected { "> " } else { "  " };
            let current_marker = if is_current { "* " } else { "" };

            let line = Line::from(vec![
                Span::raw(prefix),
                Span::styled(current_marker, style_success()),
                Span::styled(&opt.label, style_header()),
                Span::raw(": "),
                Span::styled(&opt.value, style_muted()),
            ]);

            let item = ListItem::new(line);
            if is_selected {
                item.style(style_selected())
            } else {
                item
            }
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("j/k", style_header()),
        Span::raw(" navigate  "),
        Span::styled("Enter", style_header()),
        Span::raw(" select  "),
        Span::styled("Esc", style_muted()),
        Span::raw(" cancel  "),
        Span::styled("*", style_success()),
        Span::raw(" current"),
    ]));
    frame.render_widget(hint, chunks[2]);
}

fn render_select_section_env(frame: &mut Frame, area: Rect, section: &str, selected: usize, current_indices: &[usize], options: &[EnvOption], state: &mut AppState) {
    let total_items = options.len();
    let scroll_info = scroll_status(
        state.dialog_scroll,
        area.height.saturating_sub(8) as usize,
        total_items,
    );

    let block = Block::default()
        .title(Span::styled(format!(" Switch section: [{}]{} ", section, scroll_info), style_title()))
        .borders(Borders::ALL)
        .border_style(style_border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(inner);

    let info = Paragraph::new("Select environment to switch all keys in this section:").style(style_muted());
    frame.render_widget(info, chunks[0]);

    let visible_height = chunks[1].height as usize;
    let (start, end) = calculate_visible_range(
        total_items,
        selected,
        &mut state.dialog_scroll,
        visible_height,
    );

    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .skip(start)
        .take(end - start)
        .map(|(i, opt)| {
            let is_selected = i == selected;
            let is_current = current_indices.contains(&i);

            let prefix = if is_selected { "> " } else { "  " };
            let current_marker = if is_current { "* " } else { "" };

            let line = Line::from(vec![
                Span::raw(prefix),
                Span::styled(current_marker, style_success()),
                Span::styled(&opt.label, style_header()),
            ]);

            let item = ListItem::new(line);
            if is_selected {
                item.style(style_selected())
            } else {
                item
            }
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("j/k", style_header()),
        Span::raw(" navigate  "),
        Span::styled("Enter", style_header()),
        Span::raw(" switch section  "),
        Span::styled("Esc", style_muted()),
        Span::raw(" cancel  "),
        Span::styled("*", style_success()),
        Span::raw(" current"),
    ]));
    frame.render_widget(hint, chunks[2]);
}

fn render_unsaved_changes(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" Unsaved Changes ", style_title()))
        .borders(Borders::ALL)
        .border_style(style_border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Min(2),
        Constraint::Length(2),
    ])
    .split(inner);

    let msg = Paragraph::new("You have unsaved changes. Save before quitting?").style(style_normal());
    frame.render_widget(msg, chunks[0]);

    let buttons = Paragraph::new(Line::from(vec![
        Span::styled(" [y] ", style_header()),
        Span::raw("Save and quit  "),
        Span::styled(" [n] ", style_header()),
        Span::raw("Discard and quit  "),
        Span::styled(" [Esc] ", style_muted()),
        Span::raw("Cancel"),
    ]));
    frame.render_widget(buttons, chunks[1]);
}

/// Create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [area] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .areas(area);
    area
}
