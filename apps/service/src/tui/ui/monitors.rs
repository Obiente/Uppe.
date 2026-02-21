use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

use super::{COLOR_ACTIVE, COLOR_BRAND, COLOR_ERROR, COLOR_LABEL, COLOR_MUTED, COLOR_SUCCESS};
use crate::database::models::MonitorVisibility;
use crate::tui::state::AppState;
use crate::tui::types::Focus;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focus == Focus::Monitors;
    let border_style = if focused {
        Style::default().fg(COLOR_ACTIVE)
    } else {
        Style::default().fg(COLOR_BRAND)
    };

    let title = if focused { " Monitors (focused) " } else { " Monitors " };

    if state.monitors.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("No monitors configured", Style::default().fg(COLOR_MUTED))),
            Line::from(""),
            Line::from(Span::styled("Press 'a' to add a private monitor", Style::default().fg(COLOR_LABEL))),
            Line::from(Span::styled("Press 'A' to add a public monitor", Style::default().fg(COLOR_LABEL))),
        ])
        .block(Block::default().borders(Borders::ALL).title(title).border_style(border_style));

        f.render_widget(Clear, area);
        f.render_widget(empty, area);
        return;
    }

    // Available width for content (minus borders)
    let inner_width = area.width.saturating_sub(2) as usize;

    let items: Vec<ListItem> = state
        .monitors
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let selected = i == state.selected;
            let name_style = if selected {
                Style::default().fg(COLOR_ACTIVE).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let status_char = if m.enabled { "+" } else { "-" };
            let status_color = if m.enabled { COLOR_SUCCESS } else { COLOR_ERROR };

            let vis = match m.visibility {
                MonitorVisibility::Public => "Pub",
                MonitorVisibility::Private => "Prv",
                MonitorVisibility::Internal => "Int",
            };

            // Build: [+] Name [Pub] type -> target
            let prefix = format!("[{}] ", status_char);
            let vis_badge = format!(" [{}] ", vis);
            let type_target = format!("{} -> ", m.check_type);

            // Calculate remaining space for target
            let fixed_len = prefix.len() + m.name.len() + vis_badge.len() + type_target.len();
            let target = if fixed_len + m.target.len() > inner_width && inner_width > fixed_len + 3 {
                let max = inner_width - fixed_len - 2;
                format!("{}..", &m.target[..max.min(m.target.len())])
            } else {
                m.target.clone()
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(status_color)),
                Span::styled(m.name.clone(), name_style),
                Span::styled(vis_badge, Style::default().fg(COLOR_MUTED)),
                Span::styled(type_target, Style::default().fg(COLOR_LABEL)),
                Span::raw(target),
            ]))
        })
        .collect();

    let monitors_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title).border_style(border_style));

    f.render_widget(Clear, area);
    f.render_widget(monitors_list, area);
}
