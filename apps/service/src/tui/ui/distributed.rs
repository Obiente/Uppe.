/// Distributed monitoring TUI components
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, Tabs},
    Frame,
};

use crate::tui::state::AppState;

/// Render distributed monitoring overview
pub fn render_distributed_overview(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header tabs
            Constraint::Min(10),    // Main content
            Constraint::Length(3),  // Summary footer
        ])
        .split(area);

    // Tabs for different views
    render_distributed_tabs(f, chunks[0], state);

    // Main content based on selected tab
    match state.distributed_tab {
        DistributedTab::PublicMonitors => render_public_monitors(f, chunks[1], state),
        DistributedTab::Consensus => render_pending_promotion(f, chunks[1], state),
        DistributedTab::PeerGroups => render_peer_groups(f, chunks[1], state),
        DistributedTab::RateLimits => render_rate_limits(f, chunks[1], state),
    }

    // Summary footer
    render_distributed_summary(f, chunks[2], state);
}

/// Render tab selector
fn render_distributed_tabs(f: &mut Frame, area: Rect, state: &AppState) {
    let titles = vec!["Public Monitors", "Pending Promotion", "Peer Groups", "Rate Limits"];
    let selected = state.distributed_tab as usize;

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Distributed Monitoring"))
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

/// Render public monitors list
fn render_public_monitors(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: List of public monitors
    let items: Vec<ListItem> = state
        .public_monitors
        .iter()
        .enumerate()
        .map(|(i, monitor)| {
            let domain = monitor.public_domain.as_deref().unwrap_or("unknown");
            let display_name = monitor
                .public_display_name
                .as_deref()
                .unwrap_or(domain);
            let check_type = &monitor.check_type;
            let interval = monitor.interval_seconds;

            let style = if i == state.selected_public_monitor {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let status_indicator = if monitor.enabled { "●" } else { "○" };
            let status_color = if monitor.enabled {
                Color::Green
            } else {
                Color::Gray
            };

            let content = Line::from(vec![
                Span::styled(status_indicator, Style::default().fg(status_color)),
                Span::raw(" "),
                Span::styled(display_name, style),
                Span::raw(" "),
                Span::styled(
                    format!("({})", check_type),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{}s", interval),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            ListItem::new(content)
        })
        .collect();

    if items.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("No public monitors", Style::default().fg(Color::DarkGray))),
            Line::from(""),
            Line::from(Span::styled("Press Shift+A in Dashboard to add one", Style::default().fg(Color::Gray))),
        ])
        .block(Block::default().borders(Borders::ALL).title("Public Monitors (Community)"));
        f.render_widget(empty, chunks[0]);
    } else {
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Public Monitors (Community)"));
        f.render_widget(list, chunks[0]);
    }

    // Right: Monitor details
    if let Some(monitor) = state.public_monitors.get(state.selected_public_monitor) {
        render_public_monitor_detail(f, chunks[1], monitor, state);
    }
}

/// Render detailed view of selected public monitor
fn render_public_monitor_detail(
    f: &mut Frame,
    area: Rect,
    monitor: &crate::database::models::Monitor,
    state: &AppState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(5)])
        .split(area);

    // Monitor info
    let domain = monitor.public_domain.as_deref().unwrap_or("unknown");
    let display_name = monitor
        .public_display_name
        .as_deref()
        .unwrap_or(domain);

    let info_text = vec![
        Line::from(vec![
            Span::styled("Display Name: ", Style::default().fg(Color::Cyan)),
            Span::raw(display_name),
        ]),
        Line::from(vec![
            Span::styled("Domain: ", Style::default().fg(Color::Cyan)),
            Span::raw(domain),
        ]),
        Line::from(vec![
            Span::styled("Target: ", Style::default().fg(Color::Cyan)),
            Span::raw(&monitor.target),
        ]),
        Line::from(vec![
            Span::styled("Check Type: ", Style::default().fg(Color::Cyan)),
            Span::raw(&monitor.check_type),
        ]),
        Line::from(vec![
            Span::styled("Interval: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}s", monitor.interval_seconds)),
        ]),
        Line::from(vec![
            Span::styled("Timeout: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}s", monitor.timeout_seconds)),
        ]),
    ];

    let info = Paragraph::new(info_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Monitor Details"),
    );

    f.render_widget(info, chunks[0]);

    // Peer coordination info
    if let Some(group) = state
        .public_monitor_groups
        .iter()
        .find(|g| g.domain == domain)
    {
        let peer_info = vec![
            Line::from(vec![
                Span::styled("Participating Peers: ", Style::default().fg(Color::Green)),
                Span::raw(group.participating_peers.len().to_string()),
            ]),
            Line::from(vec![
                Span::styled("Check Schedule: ", Style::default().fg(Color::Green)),
                Span::raw(format!(
                    "Every {}s, staggered across peers",
                    group.schedule.interval_seconds
                )),
            ]),
            Line::from(vec![
                Span::styled("Total Checks: ", Style::default().fg(Color::Green)),
                Span::raw(group.total_checks.to_string()),
            ]),
        ];

        let coordination = Paragraph::new(peer_info).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Coordination Status"),
        );

        f.render_widget(coordination, chunks[1]);
    }
}

/// Render monitors pending promotion (below threshold)
fn render_pending_promotion(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(5)])
        .split(area);

    // Pending promotion table
    let header = Row::new(vec!["Domain", "Interest Count", "Status", "Last Update"])
        .style(Style::default().fg(Color::Yellow))
        .bottom_margin(1);

    let rows: Vec<Row> = state
        .consensus_states
        .iter()
        .map(|(domain, consensus)| {
            let interest_count = consensus.pending_votes.len(); // Reinterpret as interest signals
            let threshold = 5;
            let is_promoted = interest_count >= threshold;
            let status_text = if is_promoted { 
                "✓ Promoted" 
            } else { 
                &format!("{}/{}", interest_count, threshold)
            };
            let status_color = if is_promoted {
                Color::Green
            } else {
                Color::Red
            };

            Row::new(vec![
                domain.clone(),
                interest_count.to_string(),
                status_text.to_string(),
                "Recently".to_string(), // TODO: Format timestamp
            ])
            .style(Style::default().fg(status_color))
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Pending Promotion (Threshold: 5 peers)"))
    .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(table, chunks[0]);

    // Promotion info
    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Quorum Threshold: ", Style::default().fg(Color::Cyan)),
            Span::raw("67% of peers must agree"),
        ]),
        Line::from(vec![
            Span::styled("Purpose: ", Style::default().fg(Color::Cyan)),
            Span::raw("Coordinate check scheduling across peers"),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Consensus Info"),
    );

    f.render_widget(info, chunks[1]);
}

/// Render peer groups
fn render_peer_groups(f: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .public_monitor_groups
        .iter()
        .map(|group| {
            let peer_count = group.participating_peers.len();
            let interval = group.schedule.interval_seconds;
            let checks = group.total_checks;

            let content = Line::from(vec![
                Span::styled(&group.display_name, Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::styled(
                    format!("({} peers)", peer_count),
                    Style::default().fg(Color::Green),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{}s interval", interval),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{} checks", checks),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            ListItem::new(content)
        })
        .collect();

    if items.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("No peer groups formed yet", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled("Groups form when peers coordinate on public monitors", Style::default().fg(Color::Gray))),
        ])
        .block(Block::default().borders(Borders::ALL).title("Public Monitor Groups"));
        f.render_widget(empty, area);
    } else {
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Public Monitor Groups"));
        f.render_widget(list, area);
    }
}

/// Render rate limit status
fn render_rate_limits(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(5)])
        .split(area);

    // Rate limit configuration
    let config = vec![
        Line::from(vec![
            Span::styled(
                "Minimum Check Interval: ",
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("10 seconds"),
        ]),
        Line::from(vec![
            Span::styled(
                "Maximum Checks/Hour/Peer: ",
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("360 (1 every 10s)"),
        ]),
        Line::from(vec![
            Span::styled("Purpose: ", Style::default().fg(Color::Cyan)),
            Span::raw("Prevent DDoS and resource exhaustion"),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Green)),
            Span::styled("✓ ENFORCED", Style::default().fg(Color::Green)),
        ]),
    ];

    let config_widget = Paragraph::new(config).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Rate Limit Configuration"),
    );

    f.render_widget(config_widget, chunks[0]);

    // Per-peer stats (if available)
    if !state.rate_limit_stats.is_empty() {
        let header = Row::new(vec!["Peer ID", "Checks This Hour", "Status"])
            .style(Style::default().fg(Color::Yellow))
            .bottom_margin(1);

        let rows: Vec<Row> = state
            .rate_limit_stats
            .iter()
            .map(|(peer_id, count)| {
                let status = if *count < 360 { "OK" } else { "LIMITED" };
                let status_color = if *count < 360 {
                    Color::Green
                } else {
                    Color::Red
                };

                Row::new(vec![peer_id.clone(), count.to_string(), status.to_string()])
                    .style(Style::default().fg(status_color))
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(50),
                Constraint::Percentage(30),
                Constraint::Percentage(20),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Per-Peer Rate Limits"),
        );

        f.render_widget(table, chunks[1]);
    }
}

/// Render summary footer
fn render_distributed_summary(f: &mut Frame, area: Rect, state: &AppState) {
    let public_count = state.public_monitors.len();
    let private_count = state.monitors.len() - public_count;
    let group_count = state.public_monitor_groups.len();
    let consensus_count = state.consensus_states.len();

    let summary = Line::from(vec![
        Span::styled("Public: ", Style::default().fg(Color::Green)),
        Span::raw(format!("{} ", public_count)),
        Span::styled("│ Private: ", Style::default().fg(Color::Cyan)),
        Span::raw(format!("{} ", private_count)),
        Span::styled("│ Groups: ", Style::default().fg(Color::Yellow)),
        Span::raw(format!("{} ", group_count)),
        Span::styled("│ Consensus: ", Style::default().fg(Color::Magenta)),
        Span::raw(format!("{}", consensus_count)),
    ]);

    let summary_widget = Paragraph::new(summary).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Summary"),
    );

    f.render_widget(summary_widget, area);
}

/// Distributed monitoring tab enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistributedTab {
    PublicMonitors,
    Consensus,
    PeerGroups,
    RateLimits,
}

impl Default for DistributedTab {
    fn default() -> Self {
        Self::PublicMonitors
    }
}
