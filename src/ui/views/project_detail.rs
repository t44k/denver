use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
    Frame,
};

use crate::app::{AppState, Section};
use crate::models::{DisplayItem, EnvironmentType};
use crate::ui::pagination::{calculate_visible_range, scroll_status};
use crate::ui::styles::*;

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState) {
    // Split area for current config and missing sections
    let sections = state.get_sections();
    let has_missing = sections.len() > 1;

    let chunks = if has_missing {
        Layout::vertical([
            Constraint::Percentage(60), // Current config
            Constraint::Percentage(40), // Missing keys
        ])
        .split(area)
    } else {
        Layout::vertical([Constraint::Percentage(100)]).split(area)
    };

    // Clone project data needed for rendering to avoid borrow conflicts
    let project = match state.current_project() {
        Some(p) => p.clone(),
        None => return,
    };

    // Render current config section
    render_current_config(frame, chunks[0], state, &project);

    // Render missing sections if any
    if has_missing {
        render_missing_sections(frame, chunks[1], state, &project, &sections);
    }
}

fn render_current_config(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    project: &crate::models::Project,
) {
    let is_active = state.selected_section == Section::CurrentConfig;
    let border_style = if is_active {
        style_border_focused()
    } else {
        style_border()
    };

    let display_items = project.display_items();
    let total_items = display_items.len();

    // Calculate scroll status for title
    let scroll_info = if is_active {
        scroll_status(
            state.config_scroll,
            area.height.saturating_sub(6) as usize, // Account for borders, header line, and hint
            total_items,
        )
    } else {
        String::new()
    };

    let block = Block::default()
        .title(Span::styled(
            format!(" Current Configuration (.env){} ", scroll_info),
            style_header(),
        ))
        .borders(Borders::ALL)
        .border_style(border_style)
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if display_items.is_empty() {
        let empty_msg = Paragraph::new(Span::styled(
            "No environment variables in .env",
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

    // Calculate visible range for pagination
    let visible_height = chunks[1].height as usize;
    let (start, end) = if is_active {
        calculate_visible_range(
            total_items,
            state.selected_key_index,
            &mut state.config_scroll,
            visible_height,
        )
    } else {
        (0, visible_height.min(total_items))
    };

    // Create list items only for visible range
    let mut items: Vec<ListItem> = Vec::new();

    for (idx, item) in display_items.iter().enumerate().skip(start).take(end - start) {
        let is_selected = is_active && idx == state.selected_key_index;

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
        }
    }

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);

    // Render hint at bottom
    let hint = Line::from(vec![
        Span::styled("Tab", style_header()),
        Span::raw(" select env  "),
        Span::styled("Enter", style_header()),
        Span::raw(" edit  "),
        Span::styled("h/l", style_header()),
        Span::raw(" cycle env  "),
        Span::styled("S", style_header()),
        Span::raw(" bulk switch"),
    ]);
    frame.render_widget(Paragraph::new(hint), chunks[2]);
}

fn render_missing_sections(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    project: &crate::models::Project,
    sections: &[Section],
) {
    let missing_sections: Vec<_> = sections
        .iter()
        .filter_map(|s| {
            if let Section::MissingFrom(env) = s {
                Some(env)
            } else {
                None
            }
        })
        .collect();

    if missing_sections.is_empty() {
        return;
    }

    // Calculate space for each missing section
    let constraint = Constraint::Ratio(1, missing_sections.len() as u32);
    let constraints: Vec<_> = missing_sections.iter().map(|_| constraint).collect();
    let chunks = Layout::horizontal(constraints).split(area);

    for (idx, env_type) in missing_sections.iter().enumerate() {
        let section = Section::MissingFrom((*env_type).clone());
        let is_active = state.selected_section == section;

        render_missing_section(frame, chunks[idx], state, project, env_type, is_active);
    }
}

fn render_missing_section(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    project: &crate::models::Project,
    env_type: &EnvironmentType,
    is_active: bool,
) {
    let border_style = if is_active {
        style_border_focused()
    } else {
        style_border()
    };

    let missing_keys = project.missing_keys_for_env(env_type);
    let total_items = missing_keys.len();

    // Calculate scroll status for title
    let scroll_info = if is_active {
        let current_scroll = state.get_missing_scroll(env_type);
        scroll_status(
            current_scroll,
            area.height.saturating_sub(4) as usize, // Account for borders
            total_items,
        )
    } else {
        String::new()
    };

    let block = Block::default()
        .title(Span::styled(
            format!(" Missing from {}{} ", env_type.filename(), scroll_info),
            if is_active { style_header() } else { style_muted() },
        ))
        .borders(Borders::ALL)
        .border_style(border_style)
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if missing_keys.is_empty() {
        return;
    }

    // Calculate visible range for pagination
    let visible_height = inner.height as usize;
    let (start, end) = if is_active {
        let mut scroll = state.get_missing_scroll(env_type);
        let range = calculate_visible_range(
            total_items,
            state.selected_key_index,
            &mut scroll,
            visible_height,
        );
        state.set_missing_scroll(env_type, scroll);
        range
    } else {
        (0, visible_height.min(total_items))
    };

    let items: Vec<ListItem> = missing_keys
        .iter()
        .enumerate()
        .skip(start)
        .take(end - start)
        .map(|(idx, key)| {
            let is_selected = is_active && idx == state.selected_key_index;

            let value = project.get_value(key, env_type).unwrap_or("");
            let truncated = if value.len() > 25 {
                format!("{}...", &value[..22])
            } else {
                value.to_string()
            };

            let prefix = if is_selected { "> " } else { "  " };
            let style = if is_selected {
                style_selected()
            } else {
                style_normal()
            };

            let line = Line::from(vec![
                Span::raw(prefix),
                Span::styled(format!("{}: ", key), style_key()),
                Span::styled(truncated, style_muted()),
            ]);

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}
