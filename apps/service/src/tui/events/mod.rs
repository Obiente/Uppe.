pub mod edit;
pub mod keyboard;
pub mod mouse;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEventKind};

use crate::database::{Database, DatabaseImpl};
use crate::tui::state::AppState;

/// Handle all events and return true if should quit
pub async fn handle_event(state: &mut AppState, event: Event, db: &DatabaseImpl) -> Result<bool> {
    match event {
        Event::Key(k) => {
            // Only process key press events, ignore releases and repeats
            if k.kind != KeyEventKind::Press {
                return Ok(false);
            }

            // Handle popup-specific keyboard events first
            if state.show_help {
                match k.code {
                    KeyCode::Esc | KeyCode::Char('h') | KeyCode::Char('?') | KeyCode::Char('q') => {
                        state.show_help = false;
                    }
                    _ => {}
                }
                return Ok(false);
            }

            if state.show_delete_confirm {
                match k.code {
                    KeyCode::Char('y') => {
                        if let Some(m) = state.monitors.get(state.selected) {
                            db.delete_monitor(m.uuid).await?;
                            state.show_delete_confirm = false;
                            state.monitors = db.get_enabled_monitors().await?;
                            if state.selected >= state.monitors.len() {
                                state.selected = state.monitors.len().saturating_sub(1);
                            }
                            state.results.clear();
                            if let Some(m) = state.monitors.get(state.selected) {
                                state.results = db.get_recent_results(m.uuid, 50).await?;
                            }
                        } else {
                            state.show_delete_confirm = false;
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => {
                        state.show_delete_confirm = false;
                    }
                    _ => {}
                }
                return Ok(false);
            }

            if state.show_edit {
                edit::handle_edit_popup(state, k.code, db).await?;
                return Ok(false);
            }

            if state.show_result_detail {
                match k.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        state.show_result_detail = false;
                    }
                    _ => {}
                }
                return Ok(false);
            }

            // Handle main view keyboard events
            keyboard::handle_main_view(state, k, db).await
        }

        Event::Mouse(m) => {
            // Only handle mouse events if no popup is blocking
            if !state.show_help
                && !state.show_edit
                && !state.show_delete_confirm
                && !state.show_result_detail
            {
                mouse::handle_mouse(state, m, db).await
            } else {
                Ok(false)
            }
        }

        _ => Ok(false),
    }
}
