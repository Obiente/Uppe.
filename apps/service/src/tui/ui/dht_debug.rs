/// DHT Debug view - for debugging and nerds who want to see what's in the DHT
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame,
};

use super::{COLOR_ACTIVE, COLOR_BRAND, COLOR_ERROR, COLOR_INFO, COLOR_LABEL, COLOR_MUTED, COLOR_SUCCESS};
use crate::tui::state::AppState;

/// Semantic type for each row in the DHT table so the details panel knows what's selected
#[derive(Debug, Clone)]
enum DhtRow {
    BucketHeader { bucket_idx: usize, peer_count: usize },
    DhtPeer { bucket_idx: usize, peer_idx: usize },
    SectionHeader { label: String },
    ConsensusRecord { domain: String, votes: usize, agreed: bool },
    KnownPeer { peer_idx: usize },
    Empty,
}

/// Build the flat list of rows from state, returning (rows, row_metadata)
fn build_rows(state: &AppState) -> Vec<(Row<'static>, DhtRow)> {
    let mut rows: Vec<(Row<'static>, DhtRow)> = Vec::new();

    // Section 1: DHT Kademlia routing table
    if let Some(snapshot) = &state.dht_snapshot {
        for (b_idx, bucket) in snapshot.buckets.iter().enumerate() {
            if bucket.peers.is_empty() {
                continue;
            }

            // Bucket header
            rows.push((
                Row::new(vec![
                    format!("K-Bucket {}", bucket.index),
                    "Bucket".to_string(),
                    format!("{} peer(s)", bucket.peers.len()),
                    String::new(),
                    String::new(),
                ]),
                DhtRow::BucketHeader { bucket_idx: b_idx, peer_count: bucket.peers.len() },
            ));

            // Peers in bucket
            for (p_idx, p) in bucket.peers.iter().enumerate() {
                let addr = if p.addrs.is_empty() {
                    "(no addrs)".to_string()
                } else {
                    p.addrs[0].clone()
                };
                let extra = if p.addrs.len() > 1 {
                    format!("+{} more", p.addrs.len() - 1)
                } else {
                    String::new()
                };
                let truncated_id = truncate_peer_id(&p.peer_id);
                rows.push((
                    Row::new(vec![
                        "  Peer".to_string(),
                        "DHT".to_string(),
                        truncated_id,
                        addr,
                        extra,
                    ]),
                    DhtRow::DhtPeer { bucket_idx: b_idx, peer_idx: p_idx },
                ));
            }
        }
    }

    // Section 2: Consensus records
    if !state.consensus_states.is_empty() {
        rows.push((
            Row::new(vec![
                "Consensus".to_string(),
                "---".to_string(),
                format!("{} domain(s)", state.consensus_states.len()),
                String::new(),
                String::new(),
            ]),
            DhtRow::SectionHeader { label: "Consensus".to_string() },
        ));
        for (domain, info) in &state.consensus_states {
            let votes = info.pending_votes.len();
            let agreed = info.last_consensus_at.is_some();
            let status = if agreed { "Agreed" } else { "Pending" };
            rows.push((
                Row::new(vec![
                    "  Record".to_string(),
                    "Consensus".to_string(),
                    domain.clone(),
                    format!("{} votes", votes),
                    status.to_string(),
                ]),
                DhtRow::ConsensusRecord { domain: domain.clone(), votes, agreed },
            ));
        }
    }

    // Section 3: Known peers from DB
    if !state.peers.is_empty() {
        rows.push((
            Row::new(vec![
                "Known Peers".to_string(),
                "---".to_string(),
                format!("{} peer(s)", state.peers.len()),
                String::new(),
                String::new(),
            ]),
            DhtRow::SectionHeader { label: "Known Peers".to_string() },
        ));
        let now = std::time::SystemTime::now();
        for (i, peer) in state.peers.iter().enumerate() {
            let truncated_id = truncate_peer_id(&peer.peer_id);
            let ttl = match now.duration_since(peer.last_seen) {
                Ok(elapsed) => format!("{}s ago", elapsed.as_secs()),
                Err(_) => "-".to_string(),
            };
            rows.push((
                Row::new(vec![
                    "  Peer".to_string(),
                    "DB".to_string(),
                    truncated_id,
                    peer.status.clone(),
                    ttl,
                ]),
                DhtRow::KnownPeer { peer_idx: i },
            ));
        }
    }

    // Empty state
    if rows.is_empty() {
        rows.push((
            Row::new(vec![
                "No DHT data".to_string(),
                String::new(),
                "Waiting for P2P connection...".to_string(),
                String::new(),
                String::new(),
            ]),
            DhtRow::Empty,
        ));
    }

    rows
}

/// Render DHT debug view
pub fn render(f: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),   // DHT records table
            Constraint::Length(12), // Selected record details
            Constraint::Length(5),  // Footer with stats (3 content + 2 border)
        ])
        .split(area);

    render_header(f, chunks[0]);

    let row_data = build_rows(state);

    // Clamp cursor
    let max_row = row_data.len().saturating_sub(1);
    state.dht_cursor = state.dht_cursor.min(max_row);

    render_dht_table(f, chunks[1], state, &row_data);
    render_details(f, chunks[2], state, &row_data);
    render_footer(f, chunks[3], state);
}

fn render_header(f: &mut Frame, area: Rect) {
    let header = Paragraph::new(vec![Line::from(vec![
        Span::styled(
            "DHT Debug View ",
            Style::default()
                .fg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("(for nerds)", Style::default().fg(COLOR_LABEL)),
        Span::raw("  "),
        Span::styled("Kademlia DHT", Style::default().fg(COLOR_SUCCESS)),
        Span::raw("  "),
        Span::styled("X", Style::default().fg(COLOR_ACTIVE).add_modifier(Modifier::BOLD)),
        Span::styled(":Query  ", Style::default().fg(COLOR_LABEL)),
        Span::styled("G", Style::default().fg(COLOR_ACTIVE).add_modifier(Modifier::BOLD)),
        Span::styled(":Custom GET  ", Style::default().fg(COLOR_LABEL)),
        Span::styled("Up/Down", Style::default().fg(COLOR_ACTIVE).add_modifier(Modifier::BOLD)),
        Span::styled(":Navigate", Style::default().fg(COLOR_LABEL)),
    ])])
    .block(Block::default().borders(Borders::ALL));

    f.render_widget(header, area);
}

fn render_dht_table(f: &mut Frame, area: Rect, state: &AppState, row_data: &[(Row<'static>, DhtRow)]) {
    let header = Row::new(vec!["Source", "Type", "Identifier", "Address", "Info"])
        .style(Style::default().fg(COLOR_ACTIVE))
        .bottom_margin(1);

    let cursor = state.dht_cursor;

    let styled_rows: Vec<Row> = row_data.iter().enumerate().map(|(i, (row, meta))| {
        let is_selected = i == cursor;

        // Base style for this row type
        let base_style = match meta {
            DhtRow::BucketHeader { .. } => Style::default().fg(COLOR_INFO).add_modifier(Modifier::BOLD),
            DhtRow::DhtPeer { .. } => Style::default().fg(COLOR_BRAND),
            DhtRow::SectionHeader { .. } => Style::default().fg(COLOR_ACTIVE).add_modifier(Modifier::BOLD),
            DhtRow::ConsensusRecord { .. } => Style::default().fg(COLOR_ACTIVE),
            DhtRow::KnownPeer { .. } => Style::default().fg(COLOR_INFO),
            DhtRow::Empty => Style::default().fg(COLOR_MUTED),
        };

        // Override with selection highlight
        let style = if is_selected {
            Style::default().fg(COLOR_SUCCESS).add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            base_style
        };

        row.clone().style(style)
    }).collect();

    let table = Table::new(
        styled_rows,
        [
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
            Constraint::Percentage(15),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("DHT Records (Kademlia Distributed Hash Table)"),
    );

    f.render_widget(table, area);
}

fn render_details(f: &mut Frame, area: Rect, state: &AppState, row_data: &[(Row<'static>, DhtRow)]) {
    let mut lines: Vec<Line> = Vec::new();

    // Show info about the selected row
    if let Some((_, meta)) = row_data.get(state.dht_cursor) {
        match meta {
            DhtRow::BucketHeader { bucket_idx, peer_count } => {
                if let Some(snapshot) = &state.dht_snapshot {
                    if let Some(bucket) = snapshot.buckets.get(*bucket_idx) {
                        lines.push(Line::from(vec![
                            Span::styled("K-Bucket #", Style::default().fg(COLOR_ACTIVE)),
                            Span::styled(format!("{}", bucket.index), Style::default().fg(COLOR_ACTIVE).add_modifier(Modifier::BOLD)),
                            Span::styled(format!("  ({} peers)", peer_count), Style::default().fg(COLOR_LABEL)),
                        ]));
                        lines.push(Line::from(""));
                        // List all peers in this bucket
                        for (i, peer) in bucket.peers.iter().enumerate() {
                            let prefix = if i == 0 { "  Peers: " } else { "         " };
                            lines.push(Line::from(vec![
                                Span::styled(prefix, Style::default().fg(COLOR_LABEL)),
                                Span::styled(&peer.peer_id, Style::default().fg(COLOR_BRAND)),
                            ]));
                            if !peer.addrs.is_empty() {
                                for addr in &peer.addrs {
                                    lines.push(Line::from(vec![
                                        Span::raw("           "),
                                        Span::styled(addr, Style::default().fg(COLOR_MUTED)),
                                    ]));
                                }
                            }
                        }
                    }
                }
            }
            DhtRow::DhtPeer { bucket_idx, peer_idx } => {
                if let Some(snapshot) = &state.dht_snapshot {
                    if let Some(bucket) = snapshot.buckets.get(*bucket_idx) {
                        if let Some(peer) = bucket.peers.get(*peer_idx) {
                            lines.push(Line::from(vec![
                                Span::styled("Peer ID: ", Style::default().fg(COLOR_BRAND)),
                                Span::raw(&peer.peer_id),
                            ]));
                            lines.push(Line::from(vec![
                                Span::styled("Bucket:  ", Style::default().fg(COLOR_BRAND)),
                                Span::raw(format!("#{}", bucket.index)),
                            ]));
                            if peer.addrs.is_empty() {
                                lines.push(Line::from(vec![
                                    Span::styled("Addrs:   ", Style::default().fg(COLOR_LABEL)),
                                    Span::styled("(none known)", Style::default().fg(COLOR_MUTED)),
                                ]));
                            } else {
                                for (i, addr) in peer.addrs.iter().enumerate() {
                                    let label = if i == 0 { "Addrs:   " } else { "         " };
                                    lines.push(Line::from(vec![
                                        Span::styled(label, Style::default().fg(COLOR_LABEL)),
                                        Span::raw(addr),
                                    ]));
                                }
                            }
                            if let Some(ref st) = peer.state {
                                lines.push(Line::from(vec![
                                    Span::styled("State:   ", Style::default().fg(COLOR_LABEL)),
                                    Span::raw(st),
                                ]));
                            }
                        }
                    }
                }
            }
            DhtRow::ConsensusRecord { domain, votes, agreed } => {
                lines.push(Line::from(vec![
                    Span::styled("Domain:  ", Style::default().fg(COLOR_BRAND)),
                    Span::raw(domain.as_str()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("Votes:   ", Style::default().fg(COLOR_BRAND)),
                    Span::raw(format!("{}", votes)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("Status:  ", Style::default().fg(COLOR_BRAND)),
                    if *agreed {
                        Span::styled("Consensus reached", Style::default().fg(COLOR_SUCCESS))
                    } else {
                        Span::styled("Pending consensus", Style::default().fg(COLOR_ACTIVE))
                    },
                ]));
            }
            DhtRow::KnownPeer { peer_idx } => {
                if let Some(peer) = state.peers.get(*peer_idx) {
                    lines.push(Line::from(vec![
                        Span::styled("Peer ID: ", Style::default().fg(COLOR_BRAND)),
                        Span::raw(&peer.peer_id),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Status:  ", Style::default().fg(COLOR_BRAND)),
                        Span::raw(&peer.status),
                    ]));
                    let now = std::time::SystemTime::now();
                    let seen = match now.duration_since(peer.last_seen) {
                        Ok(elapsed) => format!("{}s ago", elapsed.as_secs()),
                        Err(_) => "just now".to_string(),
                    };
                    lines.push(Line::from(vec![
                        Span::styled("Seen:    ", Style::default().fg(COLOR_BRAND)),
                        Span::raw(seen),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Source:  ", Style::default().fg(COLOR_LABEL)),
                        Span::styled("Database (peer table)", Style::default().fg(COLOR_MUTED)),
                    ]));
                }
            }
            DhtRow::SectionHeader { label } => {
                lines.push(Line::from(vec![
                    Span::styled("Section: ", Style::default().fg(COLOR_ACTIVE)),
                    Span::styled(label.as_str(), Style::default().fg(COLOR_ACTIVE).add_modifier(Modifier::BOLD)),
                ]));
            }
            DhtRow::Empty => {
                lines.push(Line::from(Span::styled(
                    "No data to display",
                    Style::default().fg(COLOR_MUTED),
                )));
            }
        }
    }

    // Append snapshot summary if available
    if let Some(snapshot) = &state.dht_snapshot {
        let total_peers: usize = snapshot.buckets.iter().map(|b| b.peers.len()).sum();
        if lines.len() < 8 {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Snapshot: ", Style::default().fg(COLOR_LABEL)),
                Span::raw(format!("{} buckets, {} peers  ", snapshot.buckets.len(), total_peers)),
                Span::styled("Local: ", Style::default().fg(COLOR_LABEL)),
                Span::raw(truncate_peer_id(&snapshot.local_peer_id)),
                Span::raw(format!("  ({})", format_timestamp(snapshot.captured_at))),
            ]));
        }
    }

    // Show last query result
    if let Some((key, success)) = &state.dht_last_query {
        if lines.len() < 9 {
            let status_color = if *success { COLOR_SUCCESS } else { COLOR_ERROR };
            let status_text = if *success { "[OK]" } else { "[NOT FOUND]" };
            lines.push(Line::from(vec![
                Span::styled("Last GET: ", Style::default().fg(COLOR_LABEL)),
                Span::styled(status_text, Style::default().fg(status_color)),
                Span::raw(" "),
                Span::raw(key.as_str()),
            ]));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No DHT snapshot available. Snapshots emit every ~30s when P2P is connected.",
            Style::default().fg(COLOR_MUTED),
        )));
    }

    let widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Details"));
    f.render_widget(widget, area);
}

fn render_footer(f: &mut Frame, area: Rect, state: &AppState) {
    let peer_count = state.connected_peers;

    let mut footer_lines = vec![];

    // Line 1: DHT protocol stats
    let dht_peers: usize = state.dht_snapshot.as_ref()
        .map(|s| s.buckets.iter().map(|b| b.peers.len()).sum())
        .unwrap_or(0);
    footer_lines.push(Line::from(vec![
        Span::styled("DHT Peers: ", Style::default().fg(COLOR_BRAND)),
        Span::raw(format!("{}  ", dht_peers)),
        Span::styled("Connected: ", Style::default().fg(COLOR_BRAND)),
        Span::raw(format!("{}  ", peer_count)),
        Span::styled("Protocol: ", Style::default().fg(COLOR_BRAND)),
        Span::raw("Kademlia  "),
        Span::styled("Replication: ", Style::default().fg(COLOR_BRAND)),
        Span::raw("20"),
    ]));

    // Line 2: Query stats
    let total_queries = state.dht_successful_queries + state.dht_failed_queries;
    let success_rate = if total_queries > 0 {
        (state.dht_successful_queries * 100) / total_queries
    } else {
        0
    };
    let success_color = if total_queries == 0 { COLOR_MUTED } else if success_rate > 80 { COLOR_SUCCESS } else if success_rate > 50 { COLOR_ACTIVE } else { COLOR_ERROR };
    footer_lines.push(Line::from(vec![
        Span::styled("Queries: ", Style::default().fg(COLOR_BRAND)),
        Span::styled(format!("{} ok  ", state.dht_successful_queries), Style::default().fg(COLOR_SUCCESS)),
        Span::styled(format!("{} fail  ", state.dht_failed_queries), Style::default().fg(COLOR_ERROR)),
        Span::styled(format!("{} pending  ", state.dht_pending_queries), Style::default().fg(COLOR_ACTIVE)),
        Span::styled("Success: ", Style::default().fg(COLOR_BRAND)),
        Span::styled(format!("{}%", success_rate), Style::default().fg(success_color)),
    ]));

    // Line 3: Last query or help
    if let Some((key, success)) = &state.dht_last_query {
        let status_text = if *success { "[OK]" } else { "[MISS]" };
        let status_color = if *success { COLOR_SUCCESS } else { COLOR_ERROR };
        footer_lines.push(Line::from(vec![
            Span::styled("Last: ", Style::default().fg(COLOR_LABEL)),
            Span::styled(status_text, Style::default().fg(status_color)),
            Span::raw(" "),
            Span::raw(truncate_str(key, 60)),
        ]));
    } else {
        footer_lines.push(Line::from(vec![
            Span::styled("No queries yet. Press ", Style::default().fg(COLOR_LABEL)),
            Span::styled("X", Style::default().fg(COLOR_ACTIVE).add_modifier(Modifier::BOLD)),
            Span::styled(" to query a key.", Style::default().fg(COLOR_LABEL)),
        ]));
    }

    let footer_widget = Paragraph::new(footer_lines)
        .block(Block::default().borders(Borders::ALL).title("Query Stats"));

    f.render_widget(footer_widget, area);
}

fn truncate_peer_id(id: &str) -> String {
    if id.len() > 20 {
        format!("{}..{}", &id[..8], &id[id.len()-8..])
    } else {
        id.to_string()
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

fn format_timestamp(ts: i64) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    let time = if ts >= 0 {
        UNIX_EPOCH + Duration::from_secs(ts as u64)
    } else {
        return format!("{}", ts);
    };
    let elapsed = SystemTime::now().duration_since(time).unwrap_or(Duration::from_secs(0));
    let secs = elapsed.as_secs();
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h {}m ago", secs / 3600, (secs % 3600) / 60)
    }
}
