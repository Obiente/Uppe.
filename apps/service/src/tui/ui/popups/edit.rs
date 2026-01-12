use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::tui::state::AppState;

pub fn render(f: &mut Frame, size: Rect, state: &AppState) {
    if let Some(m) = &state.edit_monitor {
        let vchunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(size);

        let hchunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(vchunks[1]);

        let area = hchunks[1];

        let title = if state.is_add_form { "Add Monitor" } else { "Edit Monitor" };

        let labels = ["Name", "Target", "Type", "Interval (s)", "Timeout (s)", "Enabled"];
        let values = [
            m.name.clone(),
            m.target.clone(),
            m.check_type.clone(),
            format!("{}", m.interval_seconds),
            format!("{}", m.timeout_seconds),
            if m.enabled { "yes".into() } else { "no".into() },
        ];

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(Span::styled(
            title,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for (i, (label, value)) in labels.iter().zip(values.iter()).enumerate() {
            let prefix = if i == state.edit_field_index { "> " } else { "  " };
            let field_style = if i == state.edit_field_index {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            // Show cursor position for text fields
            let display_value = if i == state.edit_field_index && i < 2 {
                let mut v = value.clone();
                if state.text_cursor <= v.len() {
                    v.insert(state.text_cursor, '|');
                }
                v
            } else {
                value.clone()
            };

            lines.push(Line::from(vec![
                Span::raw(prefix),
                Span::styled(format!("{label}: "), Style::default().fg(Color::Gray)),
                Span::styled(display_value, field_style),
            ]));
        }

        lines.push(Line::from(""));

        // Show validation error if present
        if let Some(err) = &state.validation_error {
            lines.push(Line::from(Span::styled(
                format!("⚠ {err}"),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
        }

        lines.push(Line::from(Span::styled("Navigation:", Style::default().fg(Color::Gray))));
        lines.push(Line::from(
            "  Tab/Shift-Tab/j/k: Next/Prev field  Left/Right/h/l: Move cursor/adjust",
        ));
        lines.push(Line::from("  Home/End: Jump to start/end of text"));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Edit:", Style::default().fg(Color::Gray))));
        lines.push(Line::from(
            "  Type: edit text  +/- [ ]: adjust numbers  C: cycle type  Space: toggle",
        ));
        lines.push(Line::from("  Enter/S: Save  Esc: Cancel"));

        let popup =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Edit"));

        f.render_widget(Clear, area);
        f.render_widget(popup, area);
    }
}
