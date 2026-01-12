use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Clear, Row, Table};
use std::time::SystemTime;

use crate::tui::state::AppState;
use crate::tui::types::Focus;

/// Format SystemTime as HH:MM:SS in UTC timezone.
/// Returns time in UTC (Coordinated Universal Time) format.
fn format_time(time: SystemTime) -> String {
    use std::time::UNIX_EPOCH;

    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();

    let total_secs = duration.as_secs();
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

/// Format location from city, country, and region.
fn format_location(
    city: &Option<String>,
    country: &Option<String>,
    region: &Option<String>,
) -> String {
    if city.is_some() || country.is_some() {
        let mut parts = Vec::new();
        if let Some(city) = city {
            parts.push(city.clone());
        }
        if let Some(country) = country {
            parts.push(country.clone());
        }
        parts.join(", ")
    } else if let Some(region) = region {
        region.clone()
    } else {
        "-".to_string()
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let rows: Vec<Row> = state
        .results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let location = format_location(&r.city, &r.country, &r.region);

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
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(6),
        Constraint::Length(15),
        Constraint::Min(10),
    ];

    let results_title =
        if state.focus == Focus::Results { "Recent Results (focused)" } else { "Recent Results" };

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec![
                Cell::from("Time"),
                Cell::from("Status"),
                Cell::from("Latency (ms)"),
                Cell::from("Code"),
                Cell::from("Location"),
                Cell::from("Error"),
            ])
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL).title(results_title));

    f.render_widget(Clear, area);
    f.render_widget(table, area);
}
