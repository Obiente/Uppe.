use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{COLOR_ACTIVE, COLOR_BRAND, COLOR_ERROR, COLOR_LABEL, COLOR_MUTED, COLOR_SUCCESS};
use crate::tui::state::AppState;
use crate::tui::types::Focus;

fn uptime_color(pct: f64) -> ratatui::style::Color {
    if pct >= 99.0 {
        COLOR_SUCCESS
    } else if pct >= 95.0 {
        COLOR_ACTIVE
    } else {
        COLOR_ERROR
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focus == Focus::Stats;
    let border_style = if focused {
        Style::default().fg(COLOR_ACTIVE)
    } else {
        Style::default().fg(COLOR_BRAND)
    };

    let title = if focused { " Statistics (focused) " } else { " Statistics " };

    let mut lines = vec![];

    if !state.monitors.is_empty() && state.selected < state.monitors.len() {
        let monitor = &state.monitors[state.selected];
        let stats = state.get_extended_stats();
        let name: String = monitor.name.chars().take(25).collect();

        lines.push(Line::from(Span::styled(
            name,
            Style::default().fg(COLOR_ACTIVE).add_modifier(Modifier::BOLD),
        )));

        // Uptime with color
        let color = uptime_color(stats.uptime);
        lines.push(Line::from(vec![
            Span::styled("  Uptime:  ", Style::default().fg(COLOR_LABEL)),
            Span::styled(format!("{:.1}%", stats.uptime), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            Span::styled(format!("  ({}/{})", stats.successful, stats.total), Style::default().fg(COLOR_MUTED)),
        ]));

        // Latency stats
        if stats.avg_latency > 0 {
            lines.push(Line::from(vec![
                Span::styled("  Latency: ", Style::default().fg(COLOR_LABEL)),
                Span::raw(format!("avg {}ms", stats.avg_latency)),
                Span::styled(
                    format!("  min {}ms  max {}ms  p95 {}ms", stats.min_latency, stats.max_latency, stats.p95_latency),
                    Style::default().fg(COLOR_MUTED),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("  Latency: ", Style::default().fg(COLOR_LABEL)),
                Span::styled("--", Style::default().fg(COLOR_MUTED)),
            ]));
        }

        // Check interval
        lines.push(Line::from(vec![
            Span::styled("  Check:   ", Style::default().fg(COLOR_LABEL)),
            Span::raw(format!("every {}s", monitor.interval_seconds)),
            Span::styled(format!("  timeout {}s", monitor.timeout_seconds), Style::default().fg(COLOR_MUTED)),
        ]));
    } else {
        lines.push(Line::from(Span::styled("No monitor selected", Style::default().fg(COLOR_MUTED))));
    }

    // Global stats section
    lines.push(Line::from(""));
    let (total_monitors, online_monitors, global_uptime) = state.get_global_stats();
    lines.push(Line::from(Span::styled(
        "Global",
        Style::default().fg(COLOR_ACTIVE).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(vec![
        Span::styled("  Monitors: ", Style::default().fg(COLOR_LABEL)),
        Span::raw(format!("{total_monitors} total, {online_monitors} enabled")),
    ]));

    let color = uptime_color(global_uptime);
    lines.push(Line::from(vec![
        Span::styled("  Health:   ", Style::default().fg(COLOR_LABEL)),
        Span::styled(format!("{global_uptime:.0}%"), Style::default().fg(color)),
    ]));

    let widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title).border_style(border_style));

    f.render_widget(widget, area);
}
