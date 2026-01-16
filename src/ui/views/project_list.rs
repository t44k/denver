use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::ui::pagination::{calculate_visible_range, scroll_status};
use crate::ui::styles::*;

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let total_projects = state.projects.len();
    let scroll_info = scroll_status(
        state.project_list_scroll,
        area.height.saturating_sub(4) as usize, // Account for borders and header
        total_projects,
    );

    let block = Block::default()
        .title(Span::styled(
            format!(" Projects ({} found){} ", total_projects, scroll_info),
            style_header(),
        ))
        .borders(Borders::ALL)
        .border_style(style_border())
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if state.projects.is_empty() {
        let empty = List::new(vec![ListItem::new(Span::styled(
            "No projects found. Make sure directories contain .env files.",
            style_muted(),
        ))]);
        frame.render_widget(empty, inner);
        return;
    }

    // Calculate visible area - reserve 1 line for scroll indicator if needed
    let visible_height = inner.height as usize;
    let needs_scroll_indicator = total_projects > visible_height;
    let list_height = if needs_scroll_indicator {
        visible_height.saturating_sub(1)
    } else {
        visible_height
    };

    // Calculate visible range and update scroll
    let (start, end) = calculate_visible_range(
        total_projects,
        state.selected_project_index,
        &mut state.project_list_scroll,
        list_height,
    );

    let items: Vec<ListItem> = state
        .projects
        .iter()
        .enumerate()
        .skip(start)
        .take(end - start)
        .map(|(i, project)| {
            let is_selected = i == state.selected_project_index;
            let style = if is_selected {
                style_selected()
            } else {
                style_normal()
            };

            let prefix = if is_selected { "> " } else { "  " };
            let env_count = project.environments.len();
            let key_count = project.all_keys().len();

            // Build env summary string showing which envs have values in current config
            let (matching_envs, has_custom) = project.env_value_summary();
            let env_summary = {
                let mut parts: Vec<String> = matching_envs;
                if has_custom {
                    parts.push("<custom>".to_string());
                }
                if parts.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", parts.join(", "))
                }
            };

            let status = if project.has_unsaved_changes() {
                Span::styled(" [modified]", style_modified())
            } else {
                Span::styled(" [synced]", style_success())
            };

            ListItem::new(Line::from(vec![
                Span::raw(prefix),
                Span::styled(&project.name, style_key()),
                Span::styled(format!("  {} envs{}, {} keys", env_count, env_summary, key_count), style_muted()),
                status,
            ]))
            .style(style)
        })
        .collect();

    if needs_scroll_indicator {
        let chunks = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

        let list = List::new(items);
        frame.render_widget(list, chunks[0]);

        // Scroll indicator
        let has_above = state.project_list_scroll > 0;
        let has_below = end < total_projects;
        let indicator = if has_above && has_below {
            "  more    more  "
        } else if has_above {
            "  more           "
        } else if has_below {
            "           more  "
        } else {
            ""
        };
        let indicator_widget = Paragraph::new(Span::styled(indicator, style_muted()));
        frame.render_widget(indicator_widget, chunks[1]);
    } else {
        let list = List::new(items);
        frame.render_widget(list, inner);
    }
}
