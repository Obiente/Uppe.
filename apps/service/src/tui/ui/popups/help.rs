use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render(f: &mut Frame, size: Rect) {
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

    let help_lines = vec![
        Line::from(Span::styled(
            "Keybinds",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("Navigation:", Style::default().fg(Color::Yellow))),
        Line::from("  Up/Down, k/j      - Navigate in focused pane"),
        Line::from("  Tab / Shift-Tab   - Cycle focus (Monitors → Results → Stats → Network)"),
        Line::from("  Left/Right, h/l   - Jump focus (Monitors ↔ Results)"),
        Line::from("  g/Home            - Jump to first"),
        Line::from("  G/End             - Jump to last"),
        Line::from(""),
        Line::from(Span::styled("Actions:", Style::default().fg(Color::Yellow))),
        Line::from("  A                 - Add monitor"),
        Line::from("  E                 - Edit selected monitor"),
        Line::from("  D                 - Delete selected monitor"),
        Line::from("  Space/T           - Toggle enabled (Monitors list)"),
        Line::from("  Enter             - View result details (Results list)"),
        Line::from("  R                 - Refresh data"),
        Line::from("  F                 - Toggle auto-refresh"),
        Line::from(""),
        Line::from(Span::styled("Panes:", Style::default().fg(Color::Yellow))),
        Line::from("  Top-Left    - Monitors list"),
        Line::from("  Top-Right   - Recent results with latency & location"),
        Line::from("  Bottom-Left - Statistics (uptime, success, latency)"),
        Line::from("  Bottom-Right- Network & P2P (peers, bandwidth, score)"),
        Line::from(""),
        Line::from(Span::styled("General:", Style::default().fg(Color::Yellow))),
        Line::from("  ?                 - Toggle help"),
        Line::from("  Q / Esc           - Quit"),
        Line::from(""),
        Line::from(Span::styled("Edit Form:", Style::default().fg(Color::Gray))),
        Line::from("  Tab/↑/↓           - Navigate fields"),
        Line::from("  Enter             - Save monitor"),
        Line::from("  Esc/C             - Cancel"),
        Line::from(""),
        Line::from(Span::styled("Tips:", Style::default().fg(Color::Gray))),
        Line::from("  - All 4 panes visible at once"),
        Line::from("  - Mouse clicks supported"),
        Line::from("  - Use Tab to cycle through panes"),
    ];

    let popup = Paragraph::new(help_lines)
        .block(Block::default().borders(Borders::ALL).title("Help - Keybinds"));

    f.render_widget(Clear, area);
    f.render_widget(popup, area);
}
