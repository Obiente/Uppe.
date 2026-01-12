use anyhow::Result;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::database::models::Monitor;
use crate::database::{Database, DatabaseImpl};
use crate::tui::state::AppState;
use crate::tui::types::Focus;

/// Handle mouse events
pub async fn handle_mouse(
    state: &mut AppState,
    mouse: MouseEvent,
    db: &DatabaseImpl,
) -> Result<bool> {
    if let Some(areas) = &state.areas
        && let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            let x = mouse.column;
            let y = mouse.row;

            // Footer action buttons
            for (label, rect) in &areas.action_buttons {
                if is_in_rect(x, y, rect) {
                    match label.as_str() {
                        "Add" => {
                            let mut m = Monitor::new("".into(), "".into(), "http".into());
                            m.interval_seconds = 30;
                            m.timeout_seconds = 10;
                            state.edit_monitor = Some(m);
                            state.show_edit = true;
                            state.is_add_form = true;
                            state.edit_field_index = 0;
                            state.text_cursor = 0;
                        }
                        "Edit" => {
                            if let Some(mo) = state.monitors.get(state.selected).cloned() {
                                state.edit_monitor = Some(mo);
                                state.show_edit = true;
                                state.is_add_form = false;
                                state.edit_field_index = 0;
                                state.text_cursor = 0;
                            }
                        }
                        "Delete" => {
                            if state.monitors.get(state.selected).is_some() {
                                state.show_delete_confirm = true;
                            }
                        }
                        "Refresh" => {
                            state.monitors = db.get_enabled_monitors().await?;
                            if let Some(mo) = state.monitors.get(state.selected) {
                                state.results = db.get_recent_results(mo.uuid, 50).await?;
                            } else {
                                state.results.clear();
                            }
                        }
                        "Help" => {
                            state.show_help = true;
                        }
                        "Quit" => {
                            return Ok(true); // Signal to quit
                        }
                        _ => {}
                    }
                }
            }

            // Monitors area
            let mrect = areas.monitors;
            if is_in_rect(x, y, &mrect) {
                state.focus = Focus::Monitors;
                // Approximate row idx: account for border and title
                let inner_y = y.saturating_sub(mrect.y + 1);
                if inner_y < mrect.height.saturating_sub(2) {
                    let idx = inner_y as usize;
                    if idx < state.monitors.len() {
                        state.selected = idx;
                    }
                    if let Some(mo) = state.monitors.get(state.selected) {
                        state.results = db.get_recent_results(mo.uuid, 50).await?;
                    }
                }
            }

            // Results area
            let rrect = areas.results;
            if is_in_rect(x, y, &rrect) {
                state.focus = Focus::Results;
                let inner_y = y.saturating_sub(rrect.y + 2); // header row
                if inner_y < rrect.height.saturating_sub(3) {
                    let idx = inner_y as usize;
                    if idx < state.results.len() {
                        state.selected_result = idx;
                    }
                }
            }
        }

    Ok(false) // Don't quit
}

fn is_in_rect(x: u16, y: u16, rect: &ratatui::layout::Rect) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}
