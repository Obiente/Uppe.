use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::state::AppState;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let focus_style = if state.focus == crate::tui::types::Focus::Network {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };

    let title = if state.focus == crate::tui::types::Focus::Network {
        "Network (focused)"
    } else {
        "Network & P2P"
    };

    let mut lines = vec![];

    let node_status = if state.p2p_enabled {
        Span::styled(
            format!("Node ID: {}...", state.peer_id.chars().take(16).collect::<String>()),
            Style::default().fg(Color::Green),
        )
    } else {
        Span::styled("P2P: Disabled", Style::default().fg(Color::Red))
    };

    lines.push(Line::from(node_status));

    let status_text = if state.p2p_enabled { "✓ Connected" } else { "✗ Offline" };
    let status_color = if state.p2p_enabled { Color::Green } else { Color::Red };

    lines.push(Line::from(vec![
        Span::raw("Status:  "),
        Span::styled(status_text, Style::default().fg(status_color)),
    ]));

    if state.p2p_enabled {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Peers", Style::default().fg(Color::Yellow))));
        lines.push(Line::from(format!("  Connected: {}", state.connected_peers)));
        lines.push(Line::from(format!("  Total Seen: {}", state.total_peers_seen)));

        let health_pct = if state.total_peers_seen > 0 {
            (state.connected_peers * 100) / state.total_peers_seen.max(1)
        } else {
            0
        };
        let health_color = if health_pct > 80 {
            Color::Green
        } else if health_pct > 50 {
            Color::Yellow
        } else {
            Color::Red
        };
        lines.push(Line::from(vec![
            Span::raw("  Health:    "),
            Span::styled(format!("{health_pct}%"), Style::default().fg(health_color)),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Activity", Style::default().fg(Color::Yellow))));
        lines.push(Line::from(format!("  Shared:    {} results", state.results_shared)));
        lines.push(Line::from(format!("  Received:  {} results", state.results_received)));

        if let Some(ref event) = state.last_peer_event {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Last Event: ", Style::default().fg(Color::Cyan)),
                Span::raw(event),
            ]));
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "P2P networking is disabled",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(Span::raw("Enable in config to share")));
        lines.push(Line::from(Span::raw("and receive monitoring data")));
    }

    let widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title).border_style(focus_style));

    f.render_widget(widget, area);
}
