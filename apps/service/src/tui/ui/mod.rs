pub mod dht_debug;
pub mod distributed;
pub mod footer;
pub mod header;
pub mod monitors;
pub mod network;
pub mod popups;
pub mod results;
pub mod stats;
pub mod admin_keys;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Color;

// Semantic color palette — use these instead of hardcoded colors
pub const COLOR_BRAND: Color = Color::Cyan;
pub const COLOR_ACTIVE: Color = Color::Yellow;
pub const COLOR_SUCCESS: Color = Color::Green;
pub const COLOR_ERROR: Color = Color::Red;
pub const COLOR_MUTED: Color = Color::DarkGray;
pub const COLOR_LABEL: Color = Color::Gray;
pub const COLOR_INFO: Color = Color::Blue;

use crate::tui::state::AppState;
use crate::tui::types::{FrameAreas, ViewMode};

/// Render the entire UI
pub fn render(f: &mut Frame, state: &mut AppState) {
    let size = f.size();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)])
        .split(size);

    // Render header (3 lines: brand+tabs, status bar, separator)
    header::render(f, chunks[0], state);

    // Render main content based on view mode
    match state.view_mode {
        ViewMode::Dashboard => {
            render_dashboard(f, chunks[1], state, chunks[0]);
        }
        ViewMode::Distributed => {
            distributed::render_distributed_overview(f, chunks[1], state);
        }
        ViewMode::Statistics => {
            stats::render(f, chunks[1], state);
        }
        ViewMode::Network => {
            network::render(f, chunks[1], state);
        }
        ViewMode::DhtDebug => {
            dht_debug::render(f, chunks[1], state);
        }
        ViewMode::AdminKeys => {
            admin_keys::render(f, chunks[1], state);
        }
    }

    // Render footer (dynamic per view)
    footer::render(f, chunks[2], state);

    // Render popups (overlays) - these appear on top of any view
    if state.show_help {
        popups::help::render(f, size);
    }

    if state.show_edit {
        popups::edit::render(f, size, state);
    }

    if state.show_delete_confirm {
        popups::delete::render(f, size, state);
    }

    if state.show_result_detail {
        popups::result_detail::render(f, size, state);
    }

    if state.show_dht_query_popup {
        popups::dht_query::render(f, size, state);
    }
}

/// Render the dashboard 2x2 grid layout
fn render_dashboard(f: &mut Frame, area: ratatui::layout::Rect, state: &mut AppState, header_area: ratatui::layout::Rect) {
    let grid = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let top_panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(grid[0]);

    let bottom_panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(grid[1]);

    // Render all 4 panes
    monitors::render(f, top_panes[0], state);
    results::render(f, top_panes[1], state);
    stats::render(f, bottom_panes[0], state);
    network::render(f, bottom_panes[1], state);

    // Store frame areas for mouse hit-testing
    state.areas = Some(FrameAreas {
        header: header_area,
        monitors: top_panes[0],
        results: top_panes[1],
        footer: ratatui::layout::Rect::default(),
        action_buttons: vec![],
    });
}
