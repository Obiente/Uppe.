mod events;
mod state;
mod types;
mod ui;

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

    // Init terminal in alternate screen
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    loop {
        // Periodic auto-refresh if enabled and no overlay is active
        if state.auto_refresh
            && state.last_refresh.elapsed() >= Duration::from_secs(state.refresh_interval_secs)
            && !state.show_help
            && !state.show_edit
            && !state.show_delete_confirm
            && !state.show_result_detail
        {
            state.monitors = db.get_enabled_monitors().await?;
            if let Some(m) = state.monitors.get(state.selected) {
                state.results = db.get_recent_results(m.uuid, 50).await?;
            } else {
                state.results.clear();
            }

            if let Ok(Some(stats)) = db.get_latest_network_stats().await {
                state.update_peer_stats(
                    stats.online_peers as usize,
                    stats.total_peers as usize,
                    stats.checks_performed as usize,
                    stats.checks_received as usize,
                );
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
