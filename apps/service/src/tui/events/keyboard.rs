use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::database::{Database, DatabaseImpl};
use crate::database::models::Monitor;
use crate::tui::state::AppState;
use crate::tui::types::Focus;

/// Handle keyboard events in main view (no popups open)
pub async fn handle_main_view(
    state: &mut AppState,
    key: KeyEvent,
    db: &DatabaseImpl,
) -> Result<bool> {
    match key.code {
        // Quit
        KeyCode::Char('q') | KeyCode::Esc if key.modifiers.is_empty() => {
            return Ok(true); // Signal to quit
        }

        // Help toggle
        KeyCode::Char('h') | KeyCode::Char('?') if key.modifiers.is_empty() => {
            state.show_help = !state.show_help;
        }

        // Focus switching - Tab/Shift-Tab
        KeyCode::Tab if key.modifiers.is_empty() => {
            state.focus = match state.focus {
                Focus::Monitors => Focus::Results,
                Focus::Results => Focus::Stats,
                Focus::Stats => Focus::Network,
                Focus::Network => Focus::Monitors,
            };
        }
        KeyCode::BackTab => {
            state.focus = match state.focus {
                Focus::Monitors => Focus::Network,
                Focus::Network => Focus::Stats,
                Focus::Stats => Focus::Results,
                Focus::Results => Focus::Monitors,
            };
        }

        // Vim-style left/right focus switching
        KeyCode::Char('h') | KeyCode::Left if key.modifiers.is_empty() => {
            state.focus = Focus::Monitors;
        }
        KeyCode::Char('l') | KeyCode::Right if key.modifiers.is_empty() => {
            state.focus = Focus::Results;
        }

        // Navigation - Down (j, Down arrow)
        KeyCode::Char('j') | KeyCode::Down if key.modifiers.is_empty() => {
            match state.focus {
                Focus::Monitors => {
                    state.next_monitor();
                    if let Some(m) = state.monitors.get(state.selected) {
                        state.results = db.get_recent_results(m.uuid, 50).await?;
                    }
                }
                Focus::Results => {
                    state.next_result();
                }
                Focus::Stats | Focus::Network => {}
            }
        }

        // Navigation - Up (k, Up arrow)
        KeyCode::Char('k') | KeyCode::Up if key.modifiers.is_empty() => {
            match state.focus {
                Focus::Monitors => {
                    state.prev_monitor();
                    if let Some(m) = state.monitors.get(state.selected) {
                        state.results = db.get_recent_results(m.uuid, 50).await?;
                    }
                }
                Focus::Results => {
                    state.prev_result();
                }
                Focus::Stats | Focus::Network => {}
            }
        }

        // Jump to first (g, Home)
        KeyCode::Char('g') | KeyCode::Home if key.modifiers.is_empty() => {
            match state.focus {
                Focus::Monitors => {
                    state.first_monitor();
                    if let Some(m) = state.monitors.get(state.selected) {
                        state.results = db.get_recent_results(m.uuid, 50).await?;
                    }
                }
                Focus::Results => {
                    state.first_result();
                }
                Focus::Stats | Focus::Network => {}
            }
        }

        // Jump to last (G, End)
        KeyCode::Char('G') | KeyCode::End if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
            match state.focus {
                Focus::Monitors => {
                    state.last_monitor();
                    if let Some(m) = state.monitors.get(state.selected) {
                        state.results = db.get_recent_results(m.uuid, 50).await?;
                    }
                }
                Focus::Results => {
                    state.last_result();
                }
                Focus::Stats | Focus::Network => {}
            }
        }

        // View result detail
        KeyCode::Enter if key.modifiers.is_empty() => {
            if state.focus == Focus::Results && !state.results.is_empty() {
                state.show_result_detail = true;
            }
        }

        // Toggle auto-refresh
        KeyCode::Char('f') if key.modifiers.is_empty() => {
            state.auto_refresh = !state.auto_refresh;
            state.last_refresh = std::time::Instant::now();
        }

        // Toggle enabled status
        KeyCode::Char(' ') | KeyCode::Char('t') if key.modifiers.is_empty() => {
            if state.focus == Focus::Monitors {
                if let Some(mo) = state.monitors.get(state.selected).cloned() {
                    let mut nm = mo;
                    nm.enabled = !nm.enabled;
                    db.save_monitor(&nm).await?;
                    state.monitors = db.get_enabled_monitors().await?;
                    if let Some(m) = state.monitors.get(state.selected) {
                        state.results = db.get_recent_results(m.uuid, 50).await?;
                    } else {
                        state.results.clear();
                    }
                    state.last_refresh = std::time::Instant::now();
                }
            }
        }

        // Refresh data
        KeyCode::Char('r') if key.modifiers.is_empty() => {
            state.monitors = db.get_enabled_monitors().await?;
            if !state.monitors.is_empty() {
                let uuid = state.monitors[state.selected].uuid;
                state.results = db.get_recent_results(uuid, 50).await?;
            } else {
                state.results.clear();
            }
        }

        // Add monitor
        KeyCode::Char('a') if key.modifiers.is_empty() => {
            let mut m = Monitor::new("".into(), "".into(), "http".into());
            m.interval_seconds = 30;
            m.timeout_seconds = 10;
            state.edit_monitor = Some(m);
            state.show_edit = true;
            state.is_add_form = true;
            state.edit_field_index = 0;
            state.text_cursor = 0;
        }

        // Edit monitor
        KeyCode::Char('e') if key.modifiers.is_empty() => {
            if let Some(m) = state.monitors.get(state.selected).cloned() {
                state.edit_monitor = Some(m);
                state.show_edit = true;
                state.is_add_form = false;
                state.edit_field_index = 0;
                state.text_cursor = 0;
            }
        }

        // Delete monitor
        KeyCode::Char('d') if key.modifiers.is_empty() => {
            if state.monitors.get(state.selected).is_some() {
                state.show_delete_confirm = true;
            }
        }

        _ => {}
    }

    Ok(false) // Don't quit
}
