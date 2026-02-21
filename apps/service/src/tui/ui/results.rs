use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table};
use std::time::SystemTime;

use super::{COLOR_ACTIVE, COLOR_BRAND, COLOR_ERROR, COLOR_MUTED, COLOR_SUCCESS};
use crate::monitoring::types::MonitorStatus;
use crate::tui::state::AppState;
use crate::tui::types::Focus;

/// Format SystemTime as HH:MM:SS using local timezone offset.
fn format_time(time: SystemTime) -> String {
    use std::time::UNIX_EPOCH;

    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let total_secs = duration.as_secs();

    // Use chrono for local time if available, otherwise UTC
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

/// Format location from city, country, and region.
fn format_location(
    city: &Option<String>,
    country: &Option<String>,
    _region: &Option<String>,
) -> String {
    match (city, country) {
        (Some(c), Some(co)) => format!("{}, {}", c, co),
        (Some(c), None) => c.clone(),
        (None, Some(co)) => co.clone(),
        (None, None) => "-".to_string(),
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focus == Focus::Results;
    let border_style = if focused {
        Style::default().fg(COLOR_ACTIVE)
    } else {
        Style::default().fg(COLOR_BRAND)
    };

    // Show selected monitor name in title
    let title = if let Some(m) = state.monitors.get(state.selected) {
        let name: String = m.name.chars().take(20).collect();
        if focused {
            format!(" Results: {} (focused) ", name)
        } else {
            format!(" Results: {} ", name)
        }
    } else if focused {
        " Results (focused) ".to_string()
    } else {
        " Results ".to_string()
    };

    if state.results.is_empty() {
        let msg = if state.monitors.is_empty() {
            "Select a monitor to view results"
        } else {
            "No results yet -- waiting for first check..."
        };
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(msg, Style::default().fg(COLOR_MUTED))),
        ])
        .block(Block::default().borders(Borders::ALL).title(title.as_str()).border_style(border_style));

        f.render_widget(Clear, area);
        f.render_widget(empty, area);
        return;
    }

    let rows: Vec<Row> = state
        .results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let location = format_location(&r.city, &r.country, &r.region);

            let status_str = format!("{}", r.status);
            let status_color = match r.status {
                MonitorStatus::Up => COLOR_SUCCESS,
                MonitorStatus::Down => COLOR_ERROR,
                _ => COLOR_ACTIVE,
            };

            let mut row = Row::new(vec![
                Cell::from(format_time(r.timestamp)),
                Cell::from(Span::styled(status_str, Style::default().fg(status_color))),
                Cell::from(r.latency_ms.map(|v| format!("{}ms", v)).unwrap_or_else(|| "-".into())),
                Cell::from(r.status_code.map(|v| v.to_string()).unwrap_or_else(|| "-".into())),
                Cell::from(location),
                Cell::from(r.error_message.clone().unwrap_or_default()),
            ]);

            if i == state.selected_result {
                row = row.style(Style::default().fg(COLOR_ACTIVE));
            }

            row
        })
        .collect();

    let widths = [
        Constraint::Length(9),
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Length(5),
        Constraint::Length(14),
        Constraint::Min(8),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec![
                Cell::from("Time"),
                Cell::from("Status"),
                Cell::from("Latency"),
                Cell::from("Code"),
                Cell::from("Location"),
                Cell::from("Error"),
            ])
            .style(Style::default().fg(COLOR_BRAND).add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL).title(title.as_str()).border_style(border_style));

    f.render_widget(Clear, area);
    f.render_widget(table, area);
}
