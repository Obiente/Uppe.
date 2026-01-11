use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::tui::state::AppState;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let focus_style = if state.focus == crate::tui::types::Focus::Stats {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };

    let title = if state.focus == crate::tui::types::Focus::Stats {
        "Statistics (focused)"
    } else {
        "Statistics"
    };

    let (uptime, success_count, total_checks, avg_latency) = state.get_current_monitor_stats();
    let (total_monitors, online_monitors, global_uptime) = state.get_global_stats();

    let mut lines = vec![];

    if !state.monitors.is_empty() && state.selected < state.monitors.len() {
        let monitor = &state.monitors[state.selected];
        lines.push(Line::from(Span::styled(
            format!("Monitor: {}", &monitor.name[..monitor.name.len().min(20)]),
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(format!("  Uptime:  {:.1}%", uptime)));
        lines.push(Line::from(format!("  Success: {} / {}", success_count, total_checks)));
        lines.push(Line::from(format!("  Latency: {} ms", avg_latency)));
    } else {
        lines.push(Line::from("No monitor selected"));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Global Stats",
        Style::default().fg(Color::Yellow),
    )));
    lines.push(Line::from(format!("  Total:  {}", total_monitors)));
    lines.push(Line::from(format!("  Online: {}", online_monitors)));
    lines.push(Line::from(format!("  Avg:    {:.1}%", global_uptime)));

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(focus_style),
    );

    f.render_widget(widget, area);
}
