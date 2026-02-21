use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{COLOR_ACTIVE, COLOR_BRAND, COLOR_ERROR, COLOR_LABEL, COLOR_MUTED, COLOR_SUCCESS};
use crate::tui::state::AppState;
use crate::tui::types::Focus;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focus == Focus::Network;
    let border_style = if focused {
        Style::default().fg(COLOR_ACTIVE)
    } else {
        Style::default().fg(COLOR_BRAND)
    };

    let title = if focused { " Network (focused) " } else { " Network " };

    let mut lines = vec![];

    if state.p2p_enabled {
        // Node ID
        let peer_short: String = state.peer_id.chars().take(12).collect();
        lines.push(Line::from(vec![
            Span::styled("Node: ", Style::default().fg(COLOR_LABEL)),
            Span::styled(format!("{peer_short}.."), Style::default().fg(COLOR_SUCCESS)),
        ]));

        // Connection status
        if state.connected_peers == 0 {
            lines.push(Line::from(Span::styled(
                "  Connecting...",
                Style::default().fg(COLOR_ACTIVE),
            )));
        } else {
            let health_pct = if state.total_peers_seen > 0 {
                (state.connected_peers * 100) / state.total_peers_seen.max(1)
            } else {
                0
            };
            let health_color = if health_pct > 80 {
                COLOR_SUCCESS
            } else if health_pct > 50 {
                COLOR_ACTIVE
            } else {
                COLOR_ERROR
            };

            lines.push(Line::from(vec![
                Span::styled("  Peers: ", Style::default().fg(COLOR_LABEL)),
                Span::raw(format!("{} connected", state.connected_peers)),
                Span::styled(format!(" / {} seen", state.total_peers_seen), Style::default().fg(COLOR_MUTED)),
                Span::raw("  "),
                Span::styled(format!("{health_pct}%"), Style::default().fg(health_color)),
            ]));
        }

        // Activity
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Activity",
            Style::default().fg(COLOR_ACTIVE).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(vec![
            Span::styled("  Shared:   ", Style::default().fg(COLOR_LABEL)),
            Span::raw(format!("{} results", state.results_shared)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Received: ", Style::default().fg(COLOR_LABEL)),
            Span::raw(format!("{} results", state.results_received)),
        ]));

        if let Some(ref event) = state.last_peer_event {
            lines.push(Line::from(vec![
                Span::styled("  Last:     ", Style::default().fg(COLOR_LABEL)),
                Span::styled(event.clone(), Style::default().fg(COLOR_MUTED)),
            ]));
        }

        // Retention/Sync
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Maintenance",
            Style::default().fg(COLOR_ACTIVE).add_modifier(Modifier::BOLD),
        )));

        let (priv_days, pub_days, peer_days) = state.retention_policy_days;
        lines.push(Line::from(vec![
            Span::styled("  Retention: ", Style::default().fg(COLOR_LABEL)),
            Span::raw(format!("{priv_days}d prv / {pub_days}d pub / {peer_days}d peer")),
        ]));

        if let Some(last_cleanup) = state.last_retention_cleanup {
            let mins_ago = last_cleanup.elapsed().as_secs() / 60;
            lines.push(Line::from(vec![
                Span::styled("  Cleanup:   ", Style::default().fg(COLOR_LABEL)),
                Span::raw(format!("{}m ago", mins_ago)),
            ]));
        }

        if let Some(last_sync) = state.last_owner_sync {
            let mins_ago = last_sync.elapsed().as_secs() / 60;
            let time_str = if mins_ago >= 60 { format!("{}h ago", mins_ago / 60) } else { format!("{}m ago", mins_ago) };
            lines.push(Line::from(vec![
                Span::styled("  Sync:      ", Style::default().fg(COLOR_LABEL)),
                Span::raw(time_str),
            ]));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "P2P networking is disabled",
            Style::default().fg(COLOR_MUTED),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Enable P2P in your config to share",
            Style::default().fg(COLOR_LABEL),
        )));
        lines.push(Line::from(Span::styled(
            "and receive monitoring data",
            Style::default().fg(COLOR_LABEL),
        )));
    }

    let widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title).border_style(border_style));

    f.render_widget(widget, area);
}
