pub mod edit;
pub mod keyboard;
pub mod mouse;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEventKind};

use crate::database::{Database, DatabaseImpl};
use crate::tui::state::{AppState, StatusLevel};

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

            if state.show_dht_query_popup {
                match k.code {
                    KeyCode::Esc => {
                        state.show_dht_query_popup = false;
                        state.dht_query_input.clear();
                    }
                    KeyCode::Enter => {
                        if !state.dht_query_input.is_empty() {
                            if state.dht_snapshot.is_none() {
                                state.set_status("No P2P connection -- run orchestrator first", StatusLevel::Error);
                            } else {
                                let key = state.dht_query_input.clone();
                                state.dht_pending_queries += 1;
                                state.dht_last_query = Some((key.clone(), false));
                                #[allow(unused_must_use)] { crate::tui::bus::publish_dht_query(key.clone()); }
                                state.set_status(format!("DHT query: {}", key), StatusLevel::Info);
                            }
                        }
                        state.show_dht_query_popup = false;
                    }
                    KeyCode::Backspace => {
                        state.dht_query_input.pop();
                    }
                    KeyCode::Char(c) => {
                        state.dht_query_input.push(c);
                    }
                    _ => {}
                }
                return Ok(false);
            }

            if state.show_delete_confirm {
                match k.code {
                    KeyCode::Char('y') => {
                        if let Some(m) = state.monitors.get(state.selected) {
                            let name = m.name.clone();
                            match db.delete_monitor(m.uuid).await {
                                Ok(_) => {
                                    state.show_delete_confirm = false;
                                    match db.get_enabled_monitors().await {
                                        Ok(monitors) => {
                                            state.monitors = monitors;
                                            if state.selected >= state.monitors.len() {
                                                state.selected = state.monitors.len().saturating_sub(1);
                                            }
                                            state.results.clear();
                                            if let Some(m) = state.monitors.get(state.selected) {
                                                if let Ok(results) = db.get_recent_results(m.uuid, 50).await {
                                                    state.results = results;
                                                }
                                            }
                                        }
                                        Err(e) => tracing::warn!("Failed to refresh after delete: {}", e),
                                    }
                                    state.set_status(format!("Deleted: {}", name), StatusLevel::Success);
                                }
                                Err(e) => {
                                    state.show_delete_confirm = false;
                                    state.set_status(format!("Delete failed: {}", e), StatusLevel::Error);
                                }
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
            if !state.any_popup_open() {
                mouse::handle_mouse(state, m, db).await
            } else {
                Ok(false)
            }
        }

        _ => Ok(false),
    }
}
