use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::tui::state::AppState;

pub fn render(f: &mut Frame, size: Rect, state: &AppState) {
    let vchunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
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

    let mut lines = vec![
        Line::from(Span::styled(
            "DHT GET Query",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Key: ", Style::default().fg(Color::Yellow)),
            Span::raw(state.dht_query_input.clone()),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Enter to query, Esc to cancel",
            Style::default().fg(Color::Gray),
        )),
    ];

    if state.dht_query_input.is_empty() {
        lines.insert(3, Line::from(Span::styled(
            "(type a DHT key, e.g., /uppe/public-monitor/<domain>)",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let popup = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("DHT GET"));

    f.render_widget(Clear, area);
    f.render_widget(popup, area);
}
