mod events;
mod state;
mod types;
mod ui;
pub mod bus;

use anyhow::Result;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::time::Duration;

use crate::database::{Database, DatabaseImpl};
use crate::pool::LibsqlPool;

use state::AppState;

/// Run TUI with P2P information
pub async fn run_tui_with_p2p(pool: LibsqlPool, peer_id: String, p2p_enabled: bool) -> Result<()> {
    // Prepare DB
    let conn = pool.get().await?;
    crate::database::initialize_database(&conn).await?;
    drop(conn);
    let db = DatabaseImpl::new_from_pool(pool);

    // Load initial data
    let mut state = AppState::new();
    state.set_peer_info(peer_id, p2p_enabled);
    state.monitors = db.get_enabled_monitors().await?;
    if !state.monitors.is_empty() {
        let uuid = state.monitors[state.selected].uuid;
        state.results = db.get_recent_results(uuid, 50).await?;
    }

    if let Ok(Some(stats)) = db.get_latest_network_stats().await {
        state.update_peer_stats(
            stats.online_peers as usize,
            stats.total_peers as usize,
            stats.checks_performed as usize,
            stats.checks_received as usize,
        );
    }

    // Load known peers for DHT/peer visibility
    if let Ok(peers) = db.list_peers(200).await {
        state.peers = peers;
    }

    // Load persisted DHT snapshot (so the DHT view has data before the first live update)
    if let Ok(Some(json)) = db.get_setting("dht_snapshot").await {
        match serde_json::from_str::<crate::p2p::messages::DhtSnapshot>(&json) {
            Ok(snapshot) => {
                tracing::debug!(buckets = snapshot.buckets.len(), "Loaded persisted DHT snapshot");
                state.dht_snapshot = Some(snapshot);
            }
            Err(e) => tracing::warn!("Failed to parse persisted DHT snapshot: {}", e),
        }
    }



    // Init terminal in alternate screen
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Subscribe to live bus
    tracing::info!("TUI bus: subscribing for live updates");
    let mut bus_rx = bus::subscribe();

    loop {
        // Drain TUI bus (non-blocking) for live updates
        while let Ok(ev) = bus_rx.try_recv() {
            match ev {
                bus::TuiEvent::Peers(peers) => {
                    tracing::debug!(count = peers.len(), "TUI bus: received peers update");
                    state.peers = peers;
                }
                bus::TuiEvent::NetworkStats(stats) => {
                    state.update_peer_stats(
                        stats.online_peers as usize,
                        stats.total_peers as usize,
                        stats.checks_performed as usize,
                        stats.checks_received as usize,
                    );
                }
                bus::TuiEvent::DhtSnapshot(snapshot) => {
                    tracing::debug!(buckets = snapshot.buckets.len(), "TUI bus: received DHT snapshot");
                    state.dht_snapshot = Some(snapshot);
                    // dht_cursor will be clamped during rendering
                }
                bus::TuiEvent::DhtQueryResult { key, ok, bytes: _ } => {
                    tracing::debug!(%key, ok, "TUI bus: received DHT GET result");
                    // Update query stats for footer display
                    if ok { state.dht_successful_queries += 1; } else { state.dht_failed_queries += 1; }
                    state.dht_pending_queries = state.dht_pending_queries.saturating_sub(1);
                    state.dht_last_query = Some((key, ok));
                }
                _ => {}
            }
        }
        // Clear expired status notifications
        state.clear_expired_status();

        // Periodic auto-refresh if enabled and no overlay is active
        if state.auto_refresh
            && state.last_refresh.elapsed() >= Duration::from_secs(state.refresh_interval_secs)
            && !state.any_popup_open()
        {
            match db.get_enabled_monitors().await {
                Ok(monitors) => {
                    state.monitors = monitors;
                    if let Some(m) = state.monitors.get(state.selected) {
                        match db.get_recent_results(m.uuid, 50).await {
                            Ok(results) => state.results = results,
                            Err(e) => tracing::warn!("Auto-refresh results failed: {}", e),
                        }
                    } else {
                        state.results.clear();
                    }
                }
                Err(e) => {
                    tracing::warn!("Auto-refresh monitors failed: {}", e);
                }
            }

            if let Ok(Some(stats)) = db.get_latest_network_stats().await {
                state.update_peer_stats(
                    stats.online_peers as usize,
                    stats.total_peers as usize,
                    stats.checks_performed as usize,
                    stats.checks_received as usize,
                );
            }

            if state.peers.is_empty() {
                if let Ok(peers) = db.list_peers(200).await {
                    state.peers = peers;
                }
            }
            state.last_refresh = std::time::Instant::now();
        }

        // Render UI
        terminal.draw(|f| {
            ui::render(f, &mut state);
        })?;

        // Poll for events
        if event::poll(Duration::from_millis(250))? {
            let ev = event::read()?;
            let should_quit = events::handle_event(&mut state, ev, &db).await?;

            if should_quit {
                break;
            }
        }
    }

    // Cleanup terminal
    drop(terminal);
    let exec_result = execute!(stdout, DisableMouseCapture, Show, LeaveAlternateScreen);
    let raw_mode_result = disable_raw_mode();
    exec_result.and(raw_mode_result)?;
    Ok(())
}

/// Backward compatible wrapper
#[allow(dead_code)] // Backward compatibility API
pub async fn run_tui(pool: LibsqlPool) -> Result<()> {
    run_tui_with_p2p(pool, "unknown".into(), false).await
}
