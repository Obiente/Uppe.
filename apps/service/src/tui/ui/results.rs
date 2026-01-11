use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Table, Row, Cell, Clear};
use ratatui::Frame;
use std::time::SystemTime;

use crate::tui::state::AppState;
use crate::tui::types::Focus;

/// Format SystemTime as HH:MM:SS
fn format_time(time: SystemTime) -> String {
    use std::time::UNIX_EPOCH;
    
    let duration = time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    
    let total_secs = duration.as_secs();
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let rows: Vec<Row> = state.results.iter().enumerate().map(|(i, r)| {
        let location = if r.city.is_some() || r.country.is_some() {
            let mut parts = Vec::new();
            if let Some(city) = &r.city {
                parts.push(city.clone());
            }
            if let Some(country) = &r.country {
                parts.push(country.clone());
            }
            parts.join(", ")
        } else if let Some(region) = &r.region {
            region.clone()
        } else {
            "-".to_string()
        };

        let mut row = Row::new(vec![
            Cell::from(format_time(r.timestamp)),
            Cell::from(format!("{}", r.status)),
            Cell::from(r.latency_ms.map(|v| v.to_string()).unwrap_or_else(|| "-".into())),
            Cell::from(r.status_code.map(|v| v.to_string()).unwrap_or_else(|| "-".into())),
            Cell::from(location),
            Cell::from(r.error_message.clone().unwrap_or_default()),
        ]);
        
        if i == state.selected_result {
            row = row.style(Style::default().fg(Color::Yellow));
        }
        
        row
    }).collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(6),
        Constraint::Length(15),
        Constraint::Min(10),
    ];

    let results_title = if state.focus == Focus::Results {
        "Recent Results (focused)"
    } else {
        "Recent Results"
    };

    let table = Table::new(rows, widths)
        .header(Row::new(vec![
            Cell::from("Time"),
            Cell::from("Status"),
            Cell::from("Latency (ms)"),
            Cell::from("Code"),
            Cell::from("Location"),
            Cell::from("Error"),
        ]).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
        .block(Block::default().borders(Borders::ALL).title(results_title));

    f.render_widget(Clear, area);
    f.render_widget(table, area);
}
