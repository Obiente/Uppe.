use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Clear};
use ratatui::Frame;

use crate::tui::state::AppState;

pub fn render(f: &mut Frame, size: Rect, state: &AppState) {
    let vchunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(70),
            Constraint::Percentage(15),
        ])
        .split(size);
    
    let hchunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(70),
            Constraint::Percentage(15),
        ])
        .split(vchunks[1]);
    
    let area = hchunks[1];
    
    if let Some(r) = state.results.get(state.selected_result) {
        let location = if r.city.is_some() || r.country.is_some() {
            let mut parts = Vec::new();
            if let Some(city) = &r.city {
                parts.push(city.clone());
            }
            if let Some(country) = &r.country {
                parts.push(country.clone());
            }
            if let Some(region) = &r.region {
                parts.push(format!("({})", region));
            }
            parts.join(", ")
        } else if let Some(region) = &r.region {
            region.clone()
        } else {
            "Unknown".to_string()
        };

        let lines = vec![
            Line::from(Span::styled("Result Details", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(format!("Status: {}", r.status)),
            Line::from(format!("Latency: {}", r.latency_ms.map(|v| v.to_string()).unwrap_or_else(|| "-".into()))),
            Line::from(format!("Code: {}", r.status_code.map(|v| v.to_string()).unwrap_or_else(|| "-".into()))),
            Line::from(format!("Location: {}", location)),
            Line::from(format!("Error: {}", r.error_message.clone().unwrap_or_default())),
            Line::from(format!("Peer: {}", r.peer_id)),
            Line::from(""),
            Line::from("Esc/Q: Close"),
        ];
        
        let popup = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Details"));
        
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
    }
}
