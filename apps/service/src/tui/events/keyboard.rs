use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::database::models::Monitor;
use crate::database::{Database, DatabaseImpl};
use crate::tui::state::AppState;
use crate::tui::types::{Focus, ViewMode};

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

        // Help toggle with '?'
        KeyCode::Char('?') if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
            state.show_help = !state.show_help;
        }

        // View mode switching with number keys (1-4)
        KeyCode::Char('1') if key.modifiers.is_empty() => {
            state.view_mode = ViewMode::Dashboard;
            state.focus = Focus::Monitors;
        }
        KeyCode::Char('2') if key.modifiers.is_empty() => {
            state.view_mode = ViewMode::Distributed;
            state.focus = Focus::Distributed;
        }
        KeyCode::Char('3') if key.modifiers.is_empty() => {
            state.view_mode = ViewMode::Statistics;
            state.focus = Focus::Stats;
        }
        KeyCode::Char('4') if key.modifiers.is_empty() => {
            state.view_mode = ViewMode::Network;
            state.focus = Focus::Network;
        }
        KeyCode::Char('5') if key.modifiers.is_empty() => {
            state.view_mode = ViewMode::DhtDebug;
            state.focus = Focus::Network;
        }
        KeyCode::Char('6') if key.modifiers.is_empty() => {
            state.view_mode = ViewMode::AdminKeys;
            state.focus = Focus::Network;
        }

        // Tab cycles through view modes
        KeyCode::Tab if key.modifiers.is_empty() => {
            state.view_mode = state.view_mode.next();
            state.focus = match state.view_mode {
                ViewMode::Dashboard => Focus::Monitors,
                ViewMode::Distributed => Focus::Distributed,
                ViewMode::Statistics => Focus::Stats,
                ViewMode::Network => Focus::Network,
                ViewMode::DhtDebug => Focus::Network, // DHT debug uses network focus
                ViewMode::AdminKeys => Focus::Network, // AdminKeys uses network focus
            };
        }
        KeyCode::BackTab => {
            state.view_mode = state.view_mode.prev();
            state.focus = match state.view_mode {
                ViewMode::Dashboard => Focus::Monitors,
                ViewMode::Distributed => Focus::Distributed,
                ViewMode::Statistics => Focus::Stats,
                ViewMode::Network => Focus::Network,
                ViewMode::DhtDebug => Focus::Network,
                ViewMode::AdminKeys => Focus::Network,
            };
        }

        // Within Dashboard mode: cycle panes with Tab
        KeyCode::Tab if key.modifiers.is_empty() && state.view_mode == ViewMode::Dashboard => {
            state.focus = match state.focus {
                Focus::Monitors => Focus::Results,
                Focus::Results => Focus::Stats,
                Focus::Stats => Focus::Network,
                Focus::Network => Focus::Monitors,
                Focus::Distributed => Focus::Results,
            };
        }

        // Within Distributed mode: cycle tabs with Left/Right
        KeyCode::Left if key.modifiers.is_empty() && state.view_mode == ViewMode::Distributed => {
            state.prev_distributed_tab();
        }
        KeyCode::Right if key.modifiers.is_empty() && state.view_mode == ViewMode::Distributed => {
            state.next_distributed_tab();
        }

        // Within AdminKeys mode: cycle tabs with Left/Right
        KeyCode::Left if key.modifiers.is_empty() && state.view_mode == ViewMode::AdminKeys => {
            state.prev_admin_keys_tab();
        }
        KeyCode::Right if key.modifiers.is_empty() && state.view_mode == ViewMode::AdminKeys => {
            state.next_admin_keys_tab();
        }

        // Vim-style left/right to switch panes in Dashboard
        KeyCode::Char('h') | KeyCode::Left if key.modifiers.is_empty() && state.view_mode == ViewMode::Dashboard => {
            state.focus = Focus::Monitors;
        }
        KeyCode::Char('l') | KeyCode::Right if key.modifiers.is_empty() && state.view_mode == ViewMode::Dashboard => {
            state.focus = Focus::Results;
        }

        // Navigation - Down (j, Down arrow)
        KeyCode::Char('j') | KeyCode::Down if key.modifiers.is_empty() => match (state.view_mode, state.focus) {
            (ViewMode::Dashboard, Focus::Monitors) | (ViewMode::Distributed, Focus::Distributed) => {
                state.next_monitor();
                if let Some(m) = state.monitors.get(state.selected) {
                    state.results = db.get_recent_results(m.uuid, 50).await?;
                }
            }
            (ViewMode::Dashboard, Focus::Results) => {
                state.next_result();
            }
            (ViewMode::Distributed, _) => {
                if state.selected_public_monitor < state.public_monitors.len().saturating_sub(1) {
                    state.selected_public_monitor += 1;
                }
            }
            (ViewMode::DhtDebug, _) => {
                state.dht_cursor = state.dht_cursor.saturating_add(1);
            }
            _ => {}
        },

        // Navigation - Up (k, Up arrow)
        KeyCode::Char('k') | KeyCode::Up if key.modifiers.is_empty() => match (state.view_mode, state.focus) {
            (ViewMode::Dashboard, Focus::Monitors) | (ViewMode::Distributed, Focus::Distributed) => {
                state.prev_monitor();
                if let Some(m) = state.monitors.get(state.selected) {
                    state.results = db.get_recent_results(m.uuid, 50).await?;
                }
            }
            (ViewMode::Dashboard, Focus::Results) => {
                state.prev_result();
            }
            (ViewMode::Distributed, _) => {
                state.selected_public_monitor = state.selected_public_monitor.saturating_sub(1);
            }
            (ViewMode::DhtDebug, _) => {
                state.dht_cursor = state.dht_cursor.saturating_sub(1);
            }
            _ => {}
        },

        // DHT: Open query popup to type an arbitrary key (must be before 'g' jump-to-first)
        KeyCode::Char('g') if key.modifiers.is_empty() && state.view_mode == ViewMode::DhtDebug => {
            state.show_dht_query_popup = true;
            state.dht_query_input.clear();
        }

        // Jump to first (g, Home)
        KeyCode::Char('g') | KeyCode::Home if key.modifiers.is_empty() => match (state.view_mode, state.focus) {
            (ViewMode::Dashboard, Focus::Monitors) | (ViewMode::Distributed, Focus::Distributed) => {
                state.first_monitor();
                if let Some(m) = state.monitors.get(state.selected) {
                    state.results = db.get_recent_results(m.uuid, 50).await?;
                }
            }
            (ViewMode::Dashboard, Focus::Results) => {
                state.first_result();
            }
            (ViewMode::Distributed, _) => {
                state.selected_public_monitor = 0;
            }
            _ => {}
        },

        // Jump to last (G, End)
        KeyCode::Char('G') | KeyCode::End
            if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
        {
            match (state.view_mode, state.focus) {
                (ViewMode::Dashboard, Focus::Monitors) | (ViewMode::Distributed, Focus::Distributed) => {
                    state.last_monitor();
                    if let Some(m) = state.monitors.get(state.selected) {
                        state.results = db.get_recent_results(m.uuid, 50).await?;
                    }
                }
                (ViewMode::Dashboard, Focus::Results) => {
                    state.last_result();
                }
                (ViewMode::Distributed, _) => {
                    state.selected_public_monitor = state.public_monitors.len().saturating_sub(1);
                }
                _ => {}
            }
        }

        // View result detail
        KeyCode::Enter if key.modifiers.is_empty() => {
            if state.view_mode == ViewMode::Dashboard && state.focus == Focus::Results && !state.results.is_empty() {
                state.show_result_detail = true;
            }
        }

        // DHT: Query the selected row's key, or open custom query popup
        KeyCode::Char('x') if key.modifiers.is_empty() && state.view_mode == ViewMode::DhtDebug => {
            if state.dht_snapshot.is_none() {
                state.set_status("No P2P connection -- run orchestrator first", crate::tui::state::StatusLevel::Error);
            } else {
                // Build a key from the selected row context
                let key = dht_key_for_cursor(state);
                if let Some(key) = key {
                    state.dht_pending_queries += 1;
                    state.dht_last_query = Some((key.clone(), false));
                    #[allow(unused_must_use)] { crate::tui::bus::publish_dht_query(key.clone()); }
                    state.set_status(format!("DHT query: {}", key), crate::tui::state::StatusLevel::Info);
                } else {
                    // Nothing meaningful selected — open custom query popup
                    state.show_dht_query_popup = true;
                    state.dht_query_input.clear();
                }
            }
        }

        // Toggle auto-refresh
        KeyCode::Char('f') if key.modifiers.is_empty() => {
            state.auto_refresh = !state.auto_refresh;
            state.last_refresh = std::time::Instant::now();
            let label = if state.auto_refresh { "on" } else { "off" };
            state.set_status(format!("Auto-refresh: {}", label), crate::tui::state::StatusLevel::Info);
        }

        // Toggle enabled status
        KeyCode::Char(' ') | KeyCode::Char('t') if key.modifiers.is_empty() => {
            if state.focus == Focus::Monitors
                && let Some(mo) = state.monitors.get(state.selected).cloned()
            {
                let mut nm = mo;
                nm.enabled = !nm.enabled;
                let label = if nm.enabled { "enabled" } else { "disabled" };
                match db.save_monitor(&nm).await {
                    Ok(_) => {
                        let _ = state.refresh_monitors_and_results(db).await;
                        state.last_refresh = std::time::Instant::now();
                        state.set_status(format!("{}: {}", nm.name, label), crate::tui::state::StatusLevel::Info);
                    }
                    Err(e) => {
                        state.set_status(format!("Toggle failed: {}", e), crate::tui::state::StatusLevel::Error);
                    }
                }
            }
        }

        // Refresh data (Dashboard only)
        KeyCode::Char('r') if key.modifiers.is_empty() && state.view_mode == ViewMode::Dashboard => {
            match state.refresh_monitors_and_results(db).await {
                Ok(_) => state.set_status("Refreshed", crate::tui::state::StatusLevel::Info),
                Err(e) => state.set_status(format!("Refresh failed: {}", e), crate::tui::state::StatusLevel::Error),
            }
        }

        // Add monitor (Private by default) - Dashboard/Monitors only
        KeyCode::Char('a') if key.modifiers.is_empty() && state.view_mode == ViewMode::Dashboard && state.focus == Focus::Monitors => {
            let mut m = Monitor::new("".into(), "".into(), "http".into());
            m.interval_seconds = 30;
            m.timeout_seconds = 10;
            // Set owner_peer_id for private monitors
            m.owner_peer_id = Some(state.peer_id.clone());
            state.edit_monitor = Some(m);
            state.show_edit = true;
            state.is_add_form = true;
            state.edit_field_index = 0;
            state.text_cursor = 0;
        }

        // Add PUBLIC monitor (Shift+A) - Dashboard/Monitors only
        KeyCode::Char('A') if state.view_mode == ViewMode::Dashboard && state.focus == Focus::Monitors => {
            let mut m = Monitor::new_public(
                "".into(),
                "".into(),
                "".into(),
                "".into(),
                "http".into(),
            );
            m.interval_seconds = 30;
            m.timeout_seconds = 10;
            state.edit_monitor = Some(m);
            state.show_edit = true;
            state.is_add_form = true;
            state.edit_field_index = 0;
            state.text_cursor = 0;
        }

        // Edit monitor - Dashboard/Monitors only
        KeyCode::Char('e') if key.modifiers.is_empty() && state.view_mode == ViewMode::Dashboard && state.focus == Focus::Monitors => {
            if let Some(m) = state.monitors.get(state.selected).cloned() {
                state.edit_monitor = Some(m);
                state.show_edit = true;
                state.is_add_form = false;
                state.edit_field_index = 0;
                state.text_cursor = 0;
            }
        }

        // Delete monitor - Dashboard/Monitors only
        KeyCode::Char('d') if key.modifiers.is_empty() && state.view_mode == ViewMode::Dashboard && state.focus == Focus::Monitors => {
            if state.monitors.get(state.selected).is_some() {
                state.show_delete_confirm = true;
            }
        }

        _ => {}
    }

    Ok(false) // Don't quit
}

/// Derive a meaningful DHT key from the currently selected row in the DHT table.
/// Returns None if the selected row doesn't map to a queryable key.
fn dht_key_for_cursor(state: &AppState) -> Option<String> {
    // Rebuild the flat row list to find what's at dht_cursor.
    // Walk the same order as dht_debug::build_rows().
    let mut row_idx: usize = 0;
    let cursor = state.dht_cursor;

    // Section 1: DHT kbucket peers
    if let Some(snapshot) = &state.dht_snapshot {
        for bucket in &snapshot.buckets {
            if bucket.peers.is_empty() {
                continue;
            }
            // Bucket header row
            if row_idx == cursor {
                // Query for the first peer in the bucket
                if let Some(peer) = bucket.peers.first() {
                    return Some(format!("/peer/{}", peer.peer_id));
                }
                return None;
            }
            row_idx += 1;

            // Peer rows
            for peer in &bucket.peers {
                if row_idx == cursor {
                    return Some(format!("/peer/{}", peer.peer_id));
                }
                row_idx += 1;
            }
        }
    }

    // Section 2: Consensus records
    if !state.consensus_states.is_empty() {
        // Section header
        if row_idx == cursor {
            return None;
        }
        row_idx += 1;

        for (domain, _) in &state.consensus_states {
            if row_idx == cursor {
                return Some(format!("/uppe/public-monitor/{}", domain));
            }
            row_idx += 1;
        }
    }

    // Section 3: Known peers
    if !state.peers.is_empty() {
        // Section header
        if row_idx == cursor {
            return None;
        }
        row_idx += 1;

        for peer in &state.peers {
            if row_idx == cursor {
                return Some(format!("/peer/{}", peer.peer_id));
            }
            row_idx += 1;
        }
    }

    None
}
