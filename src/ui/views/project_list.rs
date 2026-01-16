use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding},
    Frame,
};

use crate::app::AppState;
use crate::ui::styles::*;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(Span::styled(
            format!(" Projects ({} found) ", state.projects.len()),
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

    let items: Vec<ListItem> = state
        .projects
        .iter()
        .enumerate()
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

    let list = List::new(items);
    frame.render_widget(list, inner);
}
