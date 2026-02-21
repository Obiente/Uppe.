use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render(f: &mut Frame, size: Rect) {
    let vchunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
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

    let key = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let section = Style::default().fg(Color::Yellow);

    let help_lines = vec![
        Line::from(Span::styled(
            "Uppe. Keybinds",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("Views:", section)),
        Line::from(vec![
            Span::styled("  1", key), Span::raw(" Dashboard    "),
            Span::styled("2", key), Span::raw(" Distributed  "),
            Span::styled("3", key), Span::raw(" Statistics"),
        ]),
        Line::from(vec![
            Span::styled("  4", key), Span::raw(" Network      "),
            Span::styled("5", key), Span::raw(" DHT Debug    "),
            Span::styled("6", key), Span::raw(" Admin Keys"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Tab", key), Span::raw("/"),
            Span::styled("Shift-Tab", key), Span::raw(" cycle views"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Navigation:", section)),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("j/k", key), Span::raw(" or "),
            Span::styled("Up/Down", key), Span::raw("    navigate list"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("h/l", key), Span::raw(" or "),
            Span::styled("Left/Right", key), Span::raw(" switch panes/tabs"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("g", key), Span::raw("/"),
            Span::styled("Home", key), Span::raw(" first  "),
            Span::styled("G", key), Span::raw("/"),
            Span::styled("End", key), Span::raw(" last"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Dashboard Actions:", section)),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("a", key), Span::raw(" add private monitor   "),
            Span::styled("A", key), Span::raw(" add public monitor"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("e", key), Span::raw(" edit   "),
            Span::styled("d", key), Span::raw(" delete   "),
            Span::styled("Space/t", key), Span::raw(" toggle enabled"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Enter", key), Span::raw(" view result detail   "),
            Span::styled("r", key), Span::raw(" refresh"),
        ]),
        Line::from(""),
        Line::from(Span::styled("DHT Debug (view 5):", section)),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("x", key), Span::raw(" quick DHT query   "),
            Span::styled("g", key), Span::raw(" custom key query"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Up/Down", key), Span::raw(" select bucket   "),
            Span::styled("Left/Right", key), Span::raw(" select peer"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Edit Form:", section)),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Tab/Up/Down", key), Span::raw(" navigate fields   "),
            Span::styled("Left/Right", key), Span::raw(" move cursor"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Enter", key), Span::raw(" save   "),
            Span::styled("Esc", key), Span::raw(" cancel   "),
            Span::styled("v", key), Span::raw(" toggle visibility"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("+/-", key), Span::raw(" interval   "),
            Span::styled("[/]", key), Span::raw(" timeout   "),
            Span::styled("Space", key), Span::raw(" toggle enabled"),
        ]),
        Line::from(""),
        Line::from(Span::styled("General:", section)),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("f", key), Span::raw(" toggle auto-refresh   "),
            Span::styled("?", key), Span::raw(" help   "),
            Span::styled("q/Esc", key), Span::raw(" quit"),
        ]),
    ];

    let popup = Paragraph::new(help_lines)
        .block(Block::default().borders(Borders::ALL).title("Help"));

    f.render_widget(Clear, area);
    f.render_widget(popup, area);
}
