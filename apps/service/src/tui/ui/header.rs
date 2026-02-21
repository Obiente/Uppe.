use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use super::{COLOR_ACTIVE, COLOR_BRAND, COLOR_ERROR, COLOR_INFO, COLOR_LABEL, COLOR_MUTED, COLOR_SUCCESS};
use crate::tui::state::{AppState, StatusLevel};
use crate::tui::types::ViewMode;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    // Row 1: Brand + view tabs
    let mut tab_spans = vec![
        Span::styled("Uppe. ", Style::default().fg(COLOR_BRAND).add_modifier(Modifier::BOLD)),
        Span::styled(" ", Style::default()),
    ];

    let views = [
        (ViewMode::Dashboard, "1:Dashboard"),
        (ViewMode::Distributed, "2:Distributed"),
        (ViewMode::Statistics, "3:Stats"),
        (ViewMode::Network, "4:Network"),
        (ViewMode::DhtDebug, "5:DHT"),
        (ViewMode::AdminKeys, "6:Admin"),
    ];

    for (i, (mode, label)) in views.iter().enumerate() {
        if i > 0 {
            tab_spans.push(Span::styled(" | ", Style::default().fg(COLOR_MUTED)));
        }
        let style = if state.view_mode == *mode {
            Style::default().fg(COLOR_ACTIVE).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(COLOR_LABEL)
        };
        tab_spans.push(Span::styled(*label, style));
    }

    // Row 2: Status bar
    let mut status_spans = vec![];

    // P2P status
    if state.p2p_enabled {
        let peer_short = &state.peer_id[..state.peer_id.len().min(8)];
        status_spans.push(Span::styled(
            format!("[P2P:{peer_short}..] "),
            Style::default().fg(COLOR_SUCCESS),
        ));
        status_spans.push(Span::styled(
            format!("{}peers ", state.connected_peers),
            Style::default().fg(COLOR_LABEL),
        ));
    } else {
        status_spans.push(Span::styled("[P2P:off] ", Style::default().fg(COLOR_MUTED)));
    }

    // Monitor/result counts
    status_spans.push(Span::styled(
        format!("{}mon {}res ", state.monitors.len(), state.results.len()),
        Style::default().fg(COLOR_LABEL),
    ));

    // Auto-refresh indicator
    if state.auto_refresh {
        let secs = state.last_refresh.elapsed().as_secs();
        status_spans.push(Span::styled(
            format!("[auto:{secs}s] "),
            Style::default().fg(COLOR_MUTED),
        ));
    }

    // Status notification (overrides right side when present)
    if let Some((msg, _, level)) = &state.status_message {
        let color = match level {
            StatusLevel::Success => COLOR_SUCCESS,
            StatusLevel::Error => COLOR_ERROR,
            StatusLevel::Info => COLOR_INFO,
        };
        status_spans.push(Span::styled(
            format!(" -- {msg}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }

    let header = Paragraph::new(vec![
        Line::from(tab_spans),
        Line::from(""),
        Line::from(status_spans),
    ]);

    f.render_widget(Clear, area);
    f.render_widget(header, area);
}
