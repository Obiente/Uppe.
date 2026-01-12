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
            format!("Node ID: {}", state.peer_id.chars().take(12).collect::<String>()),
            Style::default().fg(Color::Green),
        )
    } else {
        Span::styled("P2P: Disabled", Style::default().fg(Color::Red))
    };

    lines.push(Line::from(node_status));
    lines.push(Line::from(format!(
        "Status:  {}",
        if state.p2p_enabled { "Connected" } else { "Offline" }
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Peers", Style::default().fg(Color::Yellow))));
    lines.push(Line::from("  Connected: 342"));
    lines.push(Line::from("  Total:     1,250"));
    lines.push(Line::from("  Health:    98%"));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Metrics", Style::default().fg(Color::Yellow))));
    lines.push(Line::from("  Share:     45%"));
    lines.push(Line::from("  Score:     8,750"));
    lines.push(Line::from("  BW:        2.3/10 GB"));
    lines.push(Line::from("  Checks:    2,840 today"));

    let widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title).border_style(focus_style));

    f.render_widget(widget, area);
}
