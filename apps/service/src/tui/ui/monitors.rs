use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};

use crate::tui::state::AppState;
use crate::tui::types::Focus;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .monitors
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let selected = i == state.selected;
            let style = if selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{}", m.name), style),
                Span::styled(
                    if m.enabled { "  ✓ " } else { "  ✗ " },
                    Style::default().fg(if m.enabled { Color::Green } else { Color::Red }),
                ),
                Span::raw(format!(" [{}]", m.check_type)),
                Span::raw(format!("  -> {}", m.target)),
            ]))
        })
        .collect();

    let monitors_title =
        if state.focus == Focus::Monitors { "Monitors (focused)" } else { "Monitors" };

    let monitors_list =
        List::new(items).block(Block::default().borders(Borders::ALL).title(monitors_title));

    f.render_widget(Clear, area);
    f.render_widget(monitors_list, area);
}
