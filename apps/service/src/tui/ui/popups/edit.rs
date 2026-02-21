use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::tui::state::AppState;
use crate::tui::ui::{COLOR_ACTIVE, COLOR_BRAND, COLOR_ERROR, COLOR_LABEL, COLOR_MUTED};

pub fn render(f: &mut Frame, size: Rect, state: &AppState) {
    let Some(m) = &state.edit_monitor else { return };

    use crate::database::models::MonitorVisibility;
    let is_public = matches!(m.visibility, MonitorVisibility::Public);

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

    let title = if state.is_add_form { " Add Monitor " } else { " Edit Monitor " };

    // Build field labels and values dynamically
    let (labels, values): (Vec<&str>, Vec<String>) = if is_public {
        (
            vec!["Name", "Target", "Domain", "Display Name", "Type", "Interval (s)", "Timeout (s)", "Visibility", "Enabled"],
            vec![
                m.name.clone(),
                m.target.clone(),
                m.public_domain.as_ref().cloned().unwrap_or_default(),
                m.public_display_name.as_ref().cloned().unwrap_or_default(),
                m.check_type.clone(),
                format!("{}", m.interval_seconds),
                format!("{}", m.timeout_seconds),
                "Public".to_string(),
                if m.enabled { "yes".into() } else { "no".into() },
            ],
        )
    } else {
        (
            vec!["Name", "Target", "Type", "Interval (s)", "Timeout (s)", "Visibility", "Enabled"],
            vec![
                m.name.clone(),
                m.target.clone(),
                m.check_type.clone(),
                format!("{}", m.interval_seconds),
                format!("{}", m.timeout_seconds),
                "Private".to_string(),
                if m.enabled { "yes".into() } else { "no".into() },
            ],
        )
    };

    let text_field_count = if is_public { 4 } else { 2 };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        title.trim(),
        Style::default().fg(COLOR_BRAND).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (i, (label, value)) in labels.iter().zip(values.iter()).enumerate() {
        let is_selected = i == state.edit_field_index;
        let prefix = if is_selected { "> " } else { "  " };
        let field_style = if is_selected {
            Style::default().fg(COLOR_ACTIVE)
        } else {
            Style::default()
        };

        // Show cursor in text fields
        let display_value = if is_selected && i < text_field_count {
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
            Span::styled(format!("{label}: "), Style::default().fg(COLOR_LABEL)),
            Span::styled(display_value, field_style),
        ]));
    }

    lines.push(Line::from(""));

    // Validation error
    if let Some(err) = &state.validation_error {
        lines.push(Line::from(Span::styled(
            format!("! {err}"),
            Style::default().fg(COLOR_ERROR).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
    }

    // Instructions
    lines.push(Line::from(vec![
        Span::styled("Tab/j/k", Style::default().fg(COLOR_BRAND)),
        Span::styled(":fields  ", Style::default().fg(COLOR_LABEL)),
        Span::styled("</>", Style::default().fg(COLOR_BRAND)),
        Span::styled(":adjust  ", Style::default().fg(COLOR_LABEL)),
        Span::styled("v", Style::default().fg(COLOR_BRAND)),
        Span::styled(":visibility", Style::default().fg(COLOR_LABEL)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Enter", Style::default().fg(COLOR_BRAND)),
        Span::styled(":save  ", Style::default().fg(COLOR_LABEL)),
        Span::styled("Esc", Style::default().fg(COLOR_BRAND)),
        Span::styled(":cancel  ", Style::default().fg(COLOR_LABEL)),
        Span::styled("+/-", Style::default().fg(COLOR_BRAND)),
        Span::styled(":interval  ", Style::default().fg(COLOR_LABEL)),
        Span::styled("[/]", Style::default().fg(COLOR_BRAND)),
        Span::styled(":timeout", Style::default().fg(COLOR_LABEL)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        if is_public {
            "Public: community-owned, threshold promotion (5 peers)"
        } else {
            "Private: your monitor, encrypted results via P2P"
        },
        Style::default().fg(COLOR_MUTED),
    )));

    let popup = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title).border_style(Style::default().fg(COLOR_BRAND)));

    f.render_widget(Clear, area);
    f.render_widget(popup, area);
}
