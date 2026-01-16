use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::models::{DisplayItem, EnvironmentType};
use crate::ui::pagination::{calculate_visible_range_multiline, scroll_status};
use crate::ui::styles::*;

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState) {
    // Clone project data needed for rendering to avoid borrow conflicts
    let project = match state.current_project() {
        Some(p) => p.clone(),
        None => return,
    };

    // Render unified list (full height)
    render_unified_list(frame, area, state, &project);
}

fn render_unified_list(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    project: &crate::models::Project,
) {
    let border_style = style_border_focused();

    let display_items = project.unified_display_items();
    let total_items = display_items.len();

    // Pre-calculate the line height for each display item
    let item_heights: Vec<usize> = display_items
        .iter()
        .map(|item| match item {
            DisplayItem::Section(section_name) => {
                let section_matching_envs = project.find_section_matching_envs(section_name);
                if section_matching_envs.is_empty() {
                    1 // custom section - single line
                } else {
                    section_matching_envs.len() // one line per environment
                }
            }
            DisplayItem::Key(key) => {
                let has_duplicates = project
                    .environments
                    .get(&EnvironmentType::Default)
                    .map(|env| env.has_duplicates(key))
                    .unwrap_or(false);
                if has_duplicates {
                    1 // duplicated keys show single line
                } else {
                    let matching_envs = project.find_all_matching_envs(key);
                    if matching_envs.is_empty() {
                        1 // custom - single line
                    } else {
                        matching_envs.len() // one line per environment
                    }
                }
            }
            DisplayItem::DuplicatedKey(_, _, _) => 1, // always single line
            DisplayItem::InactiveKey(_) => 1,         // always single line
        })
        .collect();

    let total_lines: usize = item_heights.iter().sum();

    // Calculate scroll status for title
    let visible_height = area.height.saturating_sub(6) as usize; // Account for borders, header line, and hint
    let scroll_info = scroll_status(state.config_scroll, visible_height, total_lines);

    let block = Block::default()
        .title(Span::styled(
            format!(" Configuration{} ", scroll_info),
            style_header(),
        ))
        .borders(Borders::ALL)
        .border_style(border_style)
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if display_items.is_empty() {
        let empty_msg = Paragraph::new(Span::styled(
            "No environment variables found",
            style_muted(),
        ));
        frame.render_widget(empty_msg, inner);
        return;
    }

    // Split inner area for header, list, and hint
    let chunks = Layout::vertical([
        Constraint::Length(1), // Header row
        Constraint::Min(1),    // List
        Constraint::Length(1), // Hint
    ])
    .split(inner);

    // Create header
    let header = Line::from(vec![
        Span::styled(format!("{:<25}", "Key"), style_header()),
        Span::styled(format!("{:<15}", "Environment"), style_header()),
        Span::styled("Value", style_header()),
    ]);
    frame.render_widget(Paragraph::new(header), chunks[0]);

    // Calculate visible range for pagination using multiline-aware function
    let list_visible_height = chunks[1].height as usize;
    let (start, end) = if !item_heights.is_empty() {
        let selected = state.selected_key_index.min(total_items.saturating_sub(1));
        calculate_visible_range_multiline(
            &item_heights,
            selected,
            &mut state.config_scroll,
            list_visible_height,
        )
    } else {
        (0, 0)
    };

    // Create list items only for visible range
    let mut items: Vec<ListItem> = Vec::new();

    for (idx, item) in display_items.iter().enumerate().skip(start).take(end - start) {
        let is_selected = idx == state.selected_key_index;

        match item {
            DisplayItem::Section(section_name) => {
                // Calculate section's matching environments
                let section_matching_envs = project.find_section_matching_envs(section_name);
                let is_custom = section_matching_envs.is_empty();

                // Section header row - selectable
                let prefix = if is_selected { "> " } else { "  " };
                let row_style = if is_selected {
                    style_selected()
                } else {
                    style_normal()
                };

                // Calculate padding for env display
                let section_display = format!("[{}]", section_name);
                let section_width = section_display.len();
                let padding_after_section = if section_width < 23 {
                    " ".repeat(23 - section_width)
                } else {
                    " ".to_string()
                };

                // Build multiple lines for environments (each env on separate line)
                let mut lines: Vec<Line> = Vec::new();

                if is_custom {
                    // Custom section - show 'custom' on first line
                    let first_line = Line::from(vec![
                        Span::raw(prefix),
                        Span::styled(section_display.clone(), style_section_header()),
                        Span::raw(padding_after_section.clone()),
                        Span::styled("custom", style_warning()),
                    ]);
                    lines.push(first_line);
                } else {
                    // Multiple environments - each on its own line
                    for (i, env) in section_matching_envs.iter().enumerate() {
                        if i == 0 {
                            // First line: section + first env
                            let first_line = Line::from(vec![
                                Span::raw(prefix),
                                Span::styled(section_display.clone(), style_section_header()),
                                Span::raw(padding_after_section.clone()),
                                Span::styled(env.display_name().to_string(), style_success()),
                            ]);
                            lines.push(first_line);
                        } else {
                            // Continuation lines: padding + "=env"
                            let continuation_prefix = if is_selected { "> " } else { "  " };
                            let section_padding = " ".repeat(section_width + padding_after_section.len());
                            let continuation_line = Line::from(vec![
                                Span::raw(continuation_prefix),
                                Span::raw(section_padding),
                                Span::styled(" =", style_env_separator()),
                                Span::styled(env.display_name().to_string(), style_success()),
                            ]);
                            lines.push(continuation_line);
                        }
                    }
                }

                items.push(ListItem::new(lines).style(row_style));
            }
            DisplayItem::Key(key) => {
                // Get current value and check if key has a section
                let value = project
                    .get_value(key, &EnvironmentType::Default)
                    .unwrap_or("");

                // Check if this key belongs to a section (for indentation)
                let has_section = project
                    .environments
                    .get(&EnvironmentType::Default)
                    .and_then(|env| env.variables.get(key))
                    .and_then(|var| var.section.as_ref())
                    .is_some();

                // Check if this key has duplicates (making it a "winner")
                let has_duplicates = project
                    .environments
                    .get(&EnvironmentType::Default)
                    .map(|env| env.has_duplicates(key))
                    .unwrap_or(false);

                // Find all envs this value matches
                let matching_envs = project.find_all_matching_envs(key);
                let is_custom = matching_envs.is_empty();

                // Truncate value for display
                let truncated_value = if value.len() > 40 {
                    format!("{}...", &value[..37])
                } else {
                    value.to_string()
                };

                let prefix = if is_selected { "> " } else { "  " };
                let row_style = if is_selected {
                    style_selected()
                } else {
                    style_normal()
                };

                // Indent keys that belong to a section by 2 spaces
                let (indent, key_width) = if has_section {
                    ("  ", 21) // 2 space indent, shorter key width
                } else {
                    ("", 23) // No indent, full key width
                };

                // Build multiple lines for environments (each env on separate line)
                let mut lines: Vec<Line> = Vec::new();

                if has_duplicates {
                    // Winner key - show '*' marker in red on first line
                    let first_line = Line::from(vec![
                        Span::raw(prefix),
                        Span::raw(indent),
                        Span::styled(format!("{:<width$}", key, width = key_width), style_key()),
                        Span::styled("*", style_duplicated()),
                        Span::raw(" ".repeat(14)), // padding to align value
                        Span::styled(truncated_value.clone(), style_value()),
                    ]);
                    lines.push(first_line);
                } else if is_custom {
                    // Custom value - show 'custom' on first line
                    let first_line = Line::from(vec![
                        Span::raw(prefix),
                        Span::raw(indent),
                        Span::styled(format!("{:<width$}", key, width = key_width), style_key()),
                        Span::styled("custom", style_warning()),
                        Span::raw(" ".repeat(9)), // padding to align value
                        Span::styled(truncated_value.clone(), style_value()),
                    ]);
                    lines.push(first_line);
                } else {
                    // Multiple environments - each on its own line
                    for (i, env) in matching_envs.iter().enumerate() {
                        if i == 0 {
                            // First line: key + first env + value
                            let env_name = env.display_name().to_string();
                            let env_padding = if env_name.len() < 15 {
                                " ".repeat(15 - env_name.len())
                            } else {
                                String::new()
                            };
                            let first_line = Line::from(vec![
                                Span::raw(prefix),
                                Span::raw(indent),
                                Span::styled(format!("{:<width$}", key, width = key_width), style_key()),
                                Span::styled(env_name, style_success()),
                                Span::raw(env_padding),
                                Span::styled(truncated_value.clone(), style_value()),
                            ]);
                            lines.push(first_line);
                        } else {
                            // Continuation lines: padding + " =env"
                            let continuation_prefix = if is_selected { "> " } else { "  " };
                            let key_padding = " ".repeat(indent.len() + key_width);
                            let continuation_line = Line::from(vec![
                                Span::raw(continuation_prefix),
                                Span::raw(key_padding),
                                Span::styled(" =", style_env_separator()),
                                Span::styled(env.display_name().to_string(), style_success()),
                            ]);
                            lines.push(continuation_line);
                        }
                    }
                }

                items.push(ListItem::new(lines).style(row_style));
            }
            DisplayItem::DuplicatedKey(key, value, line_num) => {
                // Duplicated key - shows earlier occurrence that was overwritten
                // Check if this key belongs to a section (for indentation)
                let has_section = project
                    .environments
                    .get(&EnvironmentType::Default)
                    .and_then(|env| {
                        env.duplicated_vars
                            .iter()
                            .find(|v| v.key == *key && v.line_number == Some(*line_num))
                            .and_then(|var| var.section.as_ref())
                    })
                    .is_some();

                let prefix = if is_selected { "> " } else { "  " };
                let row_style = if is_selected {
                    style_selected()
                } else {
                    style_normal()
                };

                // Truncate value for display
                let truncated_value = if value.len() > 40 {
                    format!("{}...", &value[..37])
                } else {
                    value.to_string()
                };

                // Indent keys that belong to a section by 2 spaces
                let (indent, key_width) = if has_section {
                    ("  ", 21) // 2 space indent, shorter key width
                } else {
                    ("", 23) // No indent, full key width
                };

                let line_spans = vec![
                    Span::raw(prefix),
                    Span::raw(indent),
                    Span::styled(format!("{:<width$}", key, width = key_width), style_key()),
                    Span::styled("!!duplicated!!", style_duplicated()),
                    Span::raw(" "),
                    Span::styled(truncated_value, style_muted()),
                ];

                let line = Line::from(line_spans);
                items.push(ListItem::new(line).style(row_style));
            }
            DisplayItem::InactiveKey(key) => {
                // Inactive key - exists in other envs but not in .env
                // Show in darker gray with "disabled" as the environment status
                let prefix = if is_selected { "> " } else { "  " };
                let row_style = if is_selected {
                    style_selected()
                } else {
                    style_normal()
                };

                // Get list of envs that have this key
                let envs_with_key = project.envs_with_key(key);
                let env_list: String = if envs_with_key.is_empty() {
                    String::new()
                } else {
                    let names: Vec<String> = envs_with_key
                        .iter()
                        .map(|e| e.display_name().to_string())
                        .collect();
                    format!("[{}]", names.join(", "))
                };

                let line_spans = vec![
                    Span::raw(prefix),
                    Span::styled(format!("{:<23}", key), style_muted()),
                    Span::styled("disabled", style_muted()),
                    Span::raw("       "), // padding to align env list
                    Span::styled(env_list, style_muted()),
                ];

                let line = Line::from(line_spans);
                items.push(ListItem::new(line).style(row_style));
            }
        }
    }

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);

    // Render hint at bottom (removed Tab since we have unified list)
    let hint = Line::from(vec![
        Span::styled("Enter", style_header()),
        Span::raw(" edit  "),
        Span::styled("h/l", style_header()),
        Span::raw(" cycle env  "),
        Span::styled("S", style_header()),
        Span::raw(" bulk switch  "),
        Span::styled("a", style_header()),
        Span::raw(" add key"),
    ]);
    frame.render_widget(Paragraph::new(hint), chunks[2]);
}
