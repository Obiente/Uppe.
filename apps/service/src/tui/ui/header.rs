use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Clear};
use ratatui::Frame;

use crate::tui::state::AppState;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let p2p_status = if state.p2p_enabled {
        format!("P2P: Connected ({})", &state.peer_id[..state.peer_id.len().min(8)])
    } else {
        "P2P: Disabled".to_string()
    };
    
    let status = format!(
        "Auto-refresh: {}  Monitors: {}  Results: {}  Last: {}s  {}",
        if state.auto_refresh { "On" } else { "Off" },
        state.monitors.len(),
        state.results.len(),
        state.last_refresh.elapsed().as_secs(),
        p2p_status
    );
    
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Uppe. Dashboard ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("— 4-Pane View (Monitors | Results | Stats | Network)"),
        ]),
        Line::from(Span::styled(status, Style::default().fg(Color::Gray))),
    ]);
    
    f.render_widget(Clear, area);
    f.render_widget(header, area);
}
