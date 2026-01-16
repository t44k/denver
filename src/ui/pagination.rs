use ratatui::{
    text::{Line, Span},
    widgets::ListItem,
};

use crate::ui::styles::*;

/// Calculate the visible range and adjust scroll to keep selection visible
pub fn calculate_visible_range(
    total_items: usize,
    selected: usize,
    scroll: &mut usize,
    visible_height: usize,
) -> (usize, usize) {
    if visible_height == 0 || total_items == 0 {
        return (0, 0);
    }

    // Adjust scroll to keep selection visible
    if selected < *scroll {
        *scroll = selected;
    } else if selected >= *scroll + visible_height {
        *scroll = selected - visible_height + 1;
    }

    // Clamp scroll to valid range
    let max_scroll = total_items.saturating_sub(visible_height);
    if *scroll > max_scroll {
        *scroll = max_scroll;
    }

    let start = *scroll;
    let end = (start + visible_height).min(total_items);

    (start, end)
}

/// Create a scroll indicator showing position in list
pub fn scroll_indicator<'a>(
    scroll: usize,
    total_items: usize,
    visible_height: usize,
) -> Option<ListItem<'a>> {
    if total_items <= visible_height {
        return None;
    }

    let has_above = scroll > 0;
    let has_below = scroll + visible_height < total_items;

    let mut parts = Vec::new();

    if has_above {
        let above_count = scroll;
        parts.push(Span::styled(format!("  {} more above", above_count), style_muted()));
    }

    if has_above && has_below {
        parts.push(Span::raw(" | "));
    }

    if has_below {
        let below_count = total_items - scroll - visible_height;
        parts.push(Span::styled(format!("{} more below  ", below_count), style_muted()));
    }

    if parts.is_empty() {
        None
    } else {
        Some(ListItem::new(Line::from(parts)))
    }
}

/// Create a compact scroll bar indicator (e.g., "[1-10/50]")
pub fn scroll_status(scroll: usize, visible_count: usize, total_items: usize) -> String {
    if total_items <= visible_count {
        return String::new();
    }
    let start = scroll + 1;
    let end = (scroll + visible_count).min(total_items);
    format!(" [{}-{}/{}]", start, end, total_items)
}

/// Calculate the visible range for multi-line items where each item can have a different height.
/// Takes a slice of line counts per item and returns (start_index, end_index) of items to display.
pub fn calculate_visible_range_multiline(
    item_heights: &[usize],
    selected: usize,
    scroll: &mut usize,
    visible_height: usize,
) -> (usize, usize) {
    let total_items = item_heights.len();
    if visible_height == 0 || total_items == 0 {
        return (0, 0);
    }

    // Calculate cumulative heights for binary search
    let mut cumulative: Vec<usize> = Vec::with_capacity(total_items + 1);
    cumulative.push(0);
    for &h in item_heights {
        cumulative.push(cumulative.last().unwrap() + h);
    }
    let total_lines = *cumulative.last().unwrap();

    // Ensure scroll doesn't start past where selected item can be visible
    // First, find where selected item starts
    let selected_start_line = cumulative[selected];
    let selected_height = item_heights[selected];

    // Adjust scroll so selected item is fully visible
    // scroll is in terms of "lines from top"
    if selected_start_line < *scroll {
        // Selected item is above visible area - scroll up to show it
        *scroll = selected_start_line;
    } else if selected_start_line + selected_height > *scroll + visible_height {
        // Selected item's bottom is below visible area - scroll down
        *scroll = (selected_start_line + selected_height).saturating_sub(visible_height);
    }

    // Clamp scroll to valid range
    let max_scroll = total_lines.saturating_sub(visible_height);
    if *scroll > max_scroll {
        *scroll = max_scroll;
    }

    // Find start item: first item that has any part visible
    let start = cumulative
        .iter()
        .enumerate()
        .take(total_items)
        .find(|(i, cum)| **cum + item_heights[*i] > *scroll)
        .map(|(i, _)| i)
        .unwrap_or(0);

    // Find end item: first item that is completely below visible area
    let visible_end_line = *scroll + visible_height;
    let end = cumulative
        .iter()
        .skip(1) // skip the initial 0
        .position(|&cum| cum >= visible_end_line)
        .map(|i| i + 1)
        .unwrap_or(total_items);

    (start, end.min(total_items))
}
