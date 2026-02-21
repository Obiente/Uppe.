use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect, Direction},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Tabs},
};

use crate::tui::state::AppState;

/// Admin keys sub-tab selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminKeysTab {
    Status,
    KeyList,
    RotationHistory,
}

#[allow(dead_code)] // May be used for direct tab cycling
impl AdminKeysTab {
    pub fn next(&self) -> Self {
        match self {
            Self::Status => Self::KeyList,
            Self::KeyList => Self::RotationHistory,
            Self::RotationHistory => Self::Status,
        }
    }
    
    pub fn prev(&self) -> Self {
        match self {
            Self::Status => Self::RotationHistory,
            Self::RotationHistory => Self::KeyList,
            Self::KeyList => Self::Status,
        }
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Tabs
            Constraint::Min(0),     // Content
        ])
        .split(area);
    
    // Render tabs
    let tab_index = match state.admin_keys_tab {
        AdminKeysTab::Status => 0,
        AdminKeysTab::KeyList => 1,
        AdminKeysTab::RotationHistory => 2,
    };
    
    let titles = vec!["Status", "Keys", "Rotation History"];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Admin Key Management"))
        .select(tab_index)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    
    f.render_widget(tabs, chunks[0]);
    
    // Render selected tab content
    match state.admin_keys_tab {
        AdminKeysTab::Status => render_status(f, chunks[1], state),
        AdminKeysTab::KeyList => render_key_list(f, chunks[1], state),
        AdminKeysTab::RotationHistory => render_rotation_history(f, chunks[1], state),
    }
}

fn render_status(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),   // Bootstrap status
            Constraint::Length(10),  // Statistics
            Constraint::Min(0),      // Info
        ])
        .split(area);
    
    // Bootstrap status
    let bootstrap_lines = if let Some(ref stats) = state.admin_key_stats {
        vec![
            Line::from(vec![
                Span::styled("Bootstrap Status: ", Style::default().fg(Color::Gray)),
                Span::styled("✓ Success", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::raw("  Source: "),
                Span::styled("HTTPS (keys.uppe.dev)", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::raw("  Version: "),
                Span::styled(format!("{}", stats.version), Style::default().fg(Color::Yellow)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Last Update Check: ", Style::default().fg(Color::Gray)),
                Span::raw(format_timestamp(stats.last_updated)),
            ]),
            Line::from(vec![
                Span::raw("  Next check in: ~"),
                Span::styled("45 minutes", Style::default().fg(Color::Green)),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::styled("Bootstrap Status: ", Style::default().fg(Color::Gray)),
                Span::styled("-- Waiting", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Admin keys not yet loaded. The backend orchestrator",
                Style::default().fg(Color::Gray)
            )),
            Line::from(Span::styled(
                "must be running to bootstrap the trust chain.",
                Style::default().fg(Color::Gray)
            )),
        ]
    };
    
    let bootstrap_block = Paragraph::new(bootstrap_lines)
        .block(Block::default().borders(Borders::ALL).title("Bootstrap Status"));
    f.render_widget(bootstrap_block, chunks[0]);
    
    // Statistics
    if let Some(ref stats) = state.admin_key_stats {
        let stats_lines = vec![
            Line::from(vec![
                Span::styled("Total Keys: ", Style::default().fg(Color::Gray)),
                Span::styled(format!("{}", stats.total_keys), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Valid Keys: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}", stats.valid_keys),
                    Style::default().fg(if stats.valid_keys > 0 { Color::Green } else { Color::Red })
                ),
            ]),
            Line::from(vec![
                Span::styled("Expired Keys: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}", stats.expired_keys),
                    Style::default().fg(if stats.expired_keys > 0 { Color::Yellow } else { Color::Gray })
                ),
            ]),
            Line::from(vec![
                Span::styled("Revoked Keys: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}", stats.revoked_keys),
                    Style::default().fg(if stats.revoked_keys > 0 { Color::Red } else { Color::Gray })
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Key Rotations: ", Style::default().fg(Color::Gray)),
                Span::styled(format!("{}", stats.rotations_count), Style::default().fg(Color::Cyan)),
            ]),
        ];
        
        let stats_block = Paragraph::new(stats_lines)
            .block(Block::default().borders(Borders::ALL).title("Statistics"));
        f.render_widget(stats_block, chunks[1]);
    }
    
    // Information
    let info_lines = vec![
        Line::from(Span::styled("About Admin Keys", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("Admin keys are used to sign and verify public monitors. The Uppe team"),
        Line::from("maintains these keys and rotates them periodically for security."),
        Line::from(""),
        Line::from(vec![
            Span::raw("Keys are fetched from: "),
            Span::styled("keys.uppe.dev", Style::default().fg(Color::Cyan)),
        ]),
        Line::from("Fallback: GitHub Pages, Raw GitHub"),
        Line::from(""),
        Line::from(Span::styled("All keys are verified cryptographically before acceptance.", Style::default().fg(Color::Green))),
    ];
    
    let info_block = Paragraph::new(info_lines)
        .block(Block::default().borders(Borders::ALL).title("Information"));
    f.render_widget(info_block, chunks[2]);
}

fn render_key_list(f: &mut Frame, area: Rect, state: &AppState) {
    if state.admin_key_stats.is_none() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("No admin keys loaded", Style::default().fg(Color::DarkGray))),
            Line::from(""),
            Line::from(Span::styled("Admin keys are fetched from the bootstrap server", Style::default().fg(Color::Gray))),
            Line::from(Span::styled("when the backend orchestrator starts.", Style::default().fg(Color::Gray))),
        ])
        .block(Block::default().borders(Borders::ALL).title("Current Admin Keys"));
        f.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec!["Key ID", "Description", "Valid From", "Valid Until", "Status"])
        .style(Style::default().fg(Color::Yellow))
        .bottom_margin(1);

    // Empty table — real key data would come from trust chain
    let rows: Vec<Row> = vec![];

    let table = Table::new(
        rows,
        [
            Constraint::Length(16),
            Constraint::Min(30),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Current Admin Keys"))
    .column_spacing(2);

    f.render_widget(table, area);
}

fn render_rotation_history(f: &mut Frame, area: Rect, _state: &AppState) {
    let empty = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled("No rotation history yet", Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from(Span::styled("Key rotations will appear here when admin keys are rotated.", Style::default().fg(Color::Gray))),
    ])
    .block(Block::default().borders(Borders::ALL).title("Key Rotation History"));
    f.render_widget(empty, area);
}

fn format_timestamp(timestamp: u64) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    
    let time = UNIX_EPOCH + Duration::from_secs(timestamp);
    let elapsed = SystemTime::now().duration_since(time).unwrap_or(Duration::from_secs(0));
    
    let hours = elapsed.as_secs() / 3600;
    let minutes = (elapsed.as_secs() % 3600) / 60;
    
    if hours > 24 {
        format!("{} days ago", hours / 24)
    } else if hours > 0 {
        format!("{} hours ago", hours)
    } else {
        format!("{} minutes ago", minutes)
    }
}
