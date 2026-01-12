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
            Constraint::Percentage(35),
            Constraint::Percentage(30),
            Constraint::Percentage(35),
        ])
        .split(size);

    let hchunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(vchunks[1]);

    let area = hchunks[1];

    let name = state.monitors.get(state.selected).map(|m| m.name.clone()).unwrap_or_default();

    let popup = Paragraph::new(vec![
        Line::from(Span::styled(
            "Delete Monitor",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("Are you sure you want to delete '{}' ?", name)),
        Line::from(""),
        Line::from("Y: Yes    N/Esc: No"),
    ])
    .block(Block::default().borders(Borders::ALL).title("Confirm"));

    f.render_widget(Clear, area);
    f.render_widget(popup, area);
}
