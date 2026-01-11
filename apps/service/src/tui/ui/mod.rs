pub mod header;
pub mod monitors;
pub mod results;
pub mod footer;
pub mod popups;
pub mod stats;
pub mod network;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::tui::state::AppState;
use crate::tui::types::FrameAreas;

/// Render the entire UI
pub fn render(f: &mut Frame, state: &mut AppState) {
    let size = f.size();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(size);

    // Render header
    header::render(f, chunks[0], state);

    // Create 2x2 grid layout for main content
    let grid = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);
    
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

    // Render footer
    let action_buttons = footer::render(f, chunks[2], state.show_help);

    // Store frame areas for mouse hit-testing
    state.areas = Some(FrameAreas {
        header: chunks[0],
        monitors: top_panes[0],
        results: top_panes[1],
        footer: chunks[2],
        action_buttons,
    });

    // Render popups (overlays)
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
}
