use super::types::{Focus, FrameAreas, ViewMode};
use crate::database::models::{Monitor, MonitorResult, Peer};
use crate::monitoring::types::MonitorStatus;
use crate::validation;
use std::time::{Duration, Instant};

/// Status notification level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Success,
    Error,
}

/// Application state
pub struct AppState {
    pub monitors: Vec<Monitor>,
    pub selected: usize,
    pub results: Vec<MonitorResult>,
    pub show_help: bool,
    pub focus: Focus,
    pub view_mode: ViewMode,
    pub selected_result: usize,

    // Edit & delete
    pub show_edit: bool,
    pub edit_monitor: Option<Monitor>,
    pub show_delete_confirm: bool,
    pub show_result_detail: bool,
    pub areas: Option<FrameAreas>,

    // Editing state
    pub is_add_form: bool,
    pub edit_field_index: usize,

    // Text selection state
    pub text_cursor: usize,
    #[allow(dead_code)] // For future text selection features
    pub text_selection_start: Option<usize>,

    // Auto-refresh
    pub auto_refresh: bool,
    pub last_refresh: Instant,
    pub refresh_interval_secs: u64,

    // P2P status
    pub peer_id: String,
    pub p2p_enabled: bool,
    pub connected_peers: usize,
    pub total_peers_seen: usize,
    pub results_shared: usize,
    pub results_received: usize,
    pub last_peer_event: Option<String>,

    // Validation
    pub validation_error: Option<String>,

    // Status notifications
    pub status_message: Option<(String, Instant, StatusLevel)>,

    // Distributed monitoring
    pub distributed_tab: super::ui::distributed::DistributedTab,
    pub public_monitors: Vec<Monitor>,
    pub selected_public_monitor: usize,
    pub public_monitor_groups: Vec<peerup::distributed::PublicMonitorGroup>,
    pub consensus_states: std::collections::HashMap<String, ConsensusInfo>,
    pub rate_limit_stats: std::collections::HashMap<String, u64>,
    
    // Admin keys
    pub admin_keys_tab: super::ui::admin_keys::AdminKeysTab,
    pub admin_key_stats: Option<crate::orchestrator::admin_trust::TrustChainStats>,
    
    // Retention & Owner Sync status
    pub last_owner_sync: Option<std::time::Instant>,
    pub last_retention_cleanup: Option<std::time::Instant>,
    pub retention_policy_days: (i64, i64, i64), // (private, public, peer)
    
    // DHT Debug state
    pub peers: Vec<Peer>,
    pub dht_snapshot: Option<crate::p2p::messages::DhtSnapshot>,
    pub dht_cursor: usize,          // flat row cursor for DHT table
    pub dht_pending_queries: usize,
    pub dht_successful_queries: usize,
    pub dht_failed_queries: usize,
    pub dht_last_query: Option<(String, bool)>, // (key, success)
    pub show_dht_query_popup: bool,
    pub dht_query_input: String,
}

/// Extended monitor statistics
#[derive(Debug, Clone, Default)]
pub struct MonitorStats {
    pub uptime: f64,
    pub successful: u64,
    pub total: u64,
    pub avg_latency: u64,
    pub min_latency: u64,
    pub max_latency: u64,
    pub p95_latency: u64,
}

/// Simplified consensus info for TUI display
#[derive(Debug, Clone)]
#[allow(dead_code)] // Used by TUI distributed view
pub struct ConsensusInfo {
    pub pending_votes: Vec<String>,
    pub last_consensus_at: Option<i64>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            monitors: Vec::new(),
            selected: 0,
            results: Vec::new(),
            show_help: false,
            focus: Focus::Monitors,
            view_mode: ViewMode::Dashboard,
            selected_result: 0,
            show_edit: false,
            edit_monitor: None,
            show_delete_confirm: false,
            show_result_detail: false,
            areas: None,
            is_add_form: false,
            edit_field_index: 0,
            text_cursor: 0,
            text_selection_start: None,
            auto_refresh: true,
            last_refresh: Instant::now(),
            refresh_interval_secs: 5,
            peer_id: String::new(),
            p2p_enabled: false,
            connected_peers: 0,
            total_peers_seen: 0,
            results_shared: 0,
            results_received: 0,
            last_peer_event: None,
            validation_error: None,
            status_message: None,
            distributed_tab: super::ui::distributed::DistributedTab::default(),
            public_monitors: Vec::new(),
            selected_public_monitor: 0,
            public_monitor_groups: Vec::new(),
            consensus_states: std::collections::HashMap::new(),
            rate_limit_stats: std::collections::HashMap::new(),
            admin_keys_tab: super::ui::admin_keys::AdminKeysTab::Status,
            admin_key_stats: None,
            last_owner_sync: None,
            last_retention_cleanup: None,
            retention_policy_days: (7, 30, 30), // defaults
            peers: Vec::new(),
            dht_snapshot: None,
            dht_cursor: 0,
            dht_pending_queries: 0,
            dht_successful_queries: 0,
            dht_failed_queries: 0,
            dht_last_query: None,
            show_dht_query_popup: false,
            dht_query_input: String::new(),
        }
    }

    /// Set a status notification (auto-clears after 5 seconds)
    pub fn set_status(&mut self, msg: impl Into<String>, level: StatusLevel) {
        self.status_message = Some((msg.into(), Instant::now(), level));
    }

    /// Clear expired status messages
    pub fn clear_expired_status(&mut self) {
        if let Some((_, created, _)) = &self.status_message {
            if created.elapsed() > Duration::from_secs(5) {
                self.status_message = None;
            }
        }
    }

    /// Returns true if any popup overlay is open
    pub fn any_popup_open(&self) -> bool {
        self.show_help
            || self.show_edit
            || self.show_delete_confirm
            || self.show_result_detail
            || self.show_dht_query_popup
    }

    pub fn update_peer_stats(
        &mut self,
        connected: usize,
        total: usize,
        shared: usize,
        received: usize,
    ) {
        self.connected_peers = connected;
        self.total_peers_seen = total;
        self.results_shared = shared;
        self.results_received = received;
    }

    #[allow(dead_code)] // TUI API
    pub fn record_peer_event(&mut self, event: String) {
        self.last_peer_event = Some(event);
    }

    pub fn set_peer_info(&mut self, peer_id: String, enabled: bool) {
        self.peer_id = peer_id;
        self.p2p_enabled = enabled;
    }
    
    #[allow(dead_code)] // TUI wiring API
    pub fn update_retention_sync_stats(
        &mut self,
        last_sync: Option<std::time::Instant>,
        last_cleanup: Option<std::time::Instant>,
        policy: (i64, i64, i64),
    ) {
        self.last_owner_sync = last_sync;
        self.last_retention_cleanup = last_cleanup;
        self.retention_policy_days = policy;
    }
    
    #[allow(dead_code)] // TUI wiring API
    pub fn update_dht_stats(
        &mut self,
        pending: usize,
        successful: usize,
        failed: usize,
        last_query: Option<(String, bool)>,
    ) {
        self.dht_pending_queries = pending;
        self.dht_successful_queries = successful;
        self.dht_failed_queries = failed;
        self.dht_last_query = last_query;
    }

    pub fn get_current_field_text(&self) -> Option<&str> {
        if let Some(m) = &self.edit_monitor {
            match self.edit_field_index {
                0 => Some(&m.name),
                1 => Some(&m.target),
                2 => m.public_domain.as_deref(),
                3 => m.public_display_name.as_deref(),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn update_text_cursor(&mut self) {
        if let Some(text) = self.get_current_field_text() {
            self.text_cursor = self.text_cursor.min(text.len());
        } else {
            self.text_cursor = 0;
        }
    }

    pub fn validate_current_monitor(&mut self) -> bool {
        if let Some(m) = &self.edit_monitor {
            // Validate name
            let name_result = validation::validate_monitor_name(&m.name);
            if !name_result.is_valid {
                self.validation_error = name_result.error;
                return false;
            }

            // Validate target
            let target_result = validation::validate_monitor_target(&m.target, &m.check_type);
            if !target_result.is_valid {
                self.validation_error = target_result.error;
                return false;
            }

            // Validate interval
            let interval_result = validation::validate_interval(m.interval_seconds);
            if !interval_result.is_valid {
                self.validation_error = interval_result.error;
                return false;
            }

            // Validate timeout
            let timeout_result =
                validation::validate_timeout(m.timeout_seconds, m.interval_seconds);
            if !timeout_result.is_valid {
                self.validation_error = timeout_result.error;
                return false;
            }

            // Validate public monitor fields if visibility is Public
            use crate::database::models::MonitorVisibility;
            if matches!(m.visibility, MonitorVisibility::Public) {
                // Domain is required for public monitors
                if let Some(domain) = &m.public_domain {
                    if domain.trim().is_empty() {
                        self.validation_error = Some("Public domain cannot be empty".to_string());
                        return false;
                    }
                    // Basic domain validation
                    if !domain.contains('.') || domain.starts_with('.') || domain.ends_with('.') {
                        self.validation_error = Some("Invalid domain format (e.g., example.com)".to_string());
                        return false;
                    }
                } else {
                    self.validation_error = Some("Public domain is required for public monitors".to_string());
                    return false;
                }
                // Display name is optional, no validation needed
            }

            self.validation_error = None;
            true
        } else {
            false
        }
    }

    /// Reset edit state
    pub fn close_edit(&mut self) {
        self.show_edit = false;
        self.edit_monitor = None;
        self.is_add_form = false;
        self.validation_error = None;
        self.text_cursor = 0;
    }

    /// Navigate to next monitor (with wrapping)
    pub fn next_monitor(&mut self) {
        if !self.monitors.is_empty() {
            self.selected = (self.selected + 1) % self.monitors.len();
        }
    }

    /// Navigate to previous monitor (with wrapping)
    pub fn prev_monitor(&mut self) {
        if !self.monitors.is_empty() {
            self.selected =
                if self.selected == 0 { self.monitors.len() - 1 } else { self.selected - 1 };
        }
    }

    /// Navigate to next result (with wrapping)
    pub fn next_result(&mut self) {
        if !self.results.is_empty() {
            self.selected_result = (self.selected_result + 1) % self.results.len();
        }
    }

    /// Navigate to previous result (with wrapping)
    pub fn prev_result(&mut self) {
        if !self.results.is_empty() {
            self.selected_result = if self.selected_result == 0 {
                self.results.len() - 1
            } else {
                self.selected_result - 1
            };
        }
    }

    /// Jump to first monitor
    pub fn first_monitor(&mut self) {
        if !self.monitors.is_empty() {
            self.selected = 0;
        }
    }

    /// Jump to last monitor
    pub fn last_monitor(&mut self) {
        if !self.monitors.is_empty() {
            self.selected = self.monitors.len() - 1;
        }
    }

    /// Jump to first result
    pub fn first_result(&mut self) {
        if !self.results.is_empty() {
            self.selected_result = 0;
        }
    }

    /// Jump to last result
    pub fn last_result(&mut self) {
        if !self.results.is_empty() {
            self.selected_result = self.results.len() - 1;
        }
    }

    /// Calculate stats for current monitor
    #[allow(dead_code)] // TUI API
    pub fn get_current_monitor_stats(&self) -> (f64, u64, u64, u64) {
        let ext = self.get_extended_stats();
        (ext.uptime, ext.successful, ext.total, ext.avg_latency)
    }

    /// Extended stats including min/max/p95 latency
    pub fn get_extended_stats(&self) -> MonitorStats {
        if self.monitors.is_empty() || self.selected >= self.monitors.len() || self.results.is_empty() {
            return MonitorStats::default();
        }

        let total = self.results.len() as u64;
        let successful =
            self.results.iter().filter(|r| r.status == MonitorStatus::Up).count() as u64;
        let uptime = if total > 0 { (successful as f64 / total as f64) * 100.0 } else { 0.0 };

        let mut latencies: Vec<u64> = self.results.iter().filter_map(|r| r.latency_ms).collect();
        latencies.sort_unstable();

        let (avg_latency, min_latency, max_latency, p95_latency) = if latencies.is_empty() {
            (0, 0, 0, 0)
        } else {
            let sum: u64 = latencies.iter().sum();
            let avg = sum / latencies.len() as u64;
            let min = latencies[0];
            let max = *latencies.last().unwrap();
            let p95_idx = (latencies.len() as f64 * 0.95).ceil() as usize;
            let p95 = latencies[p95_idx.min(latencies.len() - 1)];
            (avg, min, max, p95)
        };

        MonitorStats { uptime, successful, total, avg_latency, min_latency, max_latency, p95_latency }
    }

    /// Get global stats across all monitors
    pub fn get_global_stats(&self) -> (usize, usize, f64) {
        let total_monitors = self.monitors.len();
        let online = self.monitors.iter().filter(|m| m.enabled).count();
        let avg_uptime = if self.monitors.is_empty() {
            0.0
        } else {
            // Simplified: estimate based on enabled count
            (online as f64 / total_monitors as f64) * 100.0
        };
        (total_monitors, online, avg_uptime)
    }

    /// Refresh monitors list and update results for the currently selected monitor.
    /// This helper method eliminates duplicate code across event handlers.
    pub async fn refresh_monitors_and_results(
        &mut self,
        db: &impl crate::database::Database,
    ) -> anyhow::Result<()> {
        self.monitors = db.get_enabled_monitors().await?;
        if let Some(m) = self.monitors.get(self.selected) {
            self.results = db.get_recent_results(m.uuid, 50).await?;
        } else {
            self.results.clear();
        }
        Ok(())
    }

    /// Update distributed monitoring state
    #[allow(dead_code)] // TUI wiring API
    pub fn update_distributed_state(
        &mut self,
        public_monitors: Vec<Monitor>,
        groups: Vec<peerup::distributed::PublicMonitorGroup>,
    ) {
        self.public_monitors = public_monitors;
        self.public_monitor_groups = groups;
    }

    /// Update consensus state for a domain
    #[allow(dead_code)] // TUI wiring API
    pub fn update_consensus_state(&mut self, domain: String, voters: Vec<String>, has_consensus: bool) {
        self.consensus_states.insert(
            domain,
            ConsensusInfo {
                pending_votes: voters,
                last_consensus_at: if has_consensus {
                    Some(chrono::Utc::now().timestamp())
                } else {
                    None
                },
            },
        );
    }

    /// Update rate limit statistics
    #[allow(dead_code)] // TUI wiring API
    pub fn update_rate_limits(&mut self, stats: std::collections::HashMap<String, u64>) {
        self.rate_limit_stats = stats;
    }

    /// Navigate distributed tabs
    pub fn next_distributed_tab(&mut self) {
        use super::ui::distributed::DistributedTab;
        self.distributed_tab = match self.distributed_tab {
            DistributedTab::PublicMonitors => DistributedTab::Consensus,
            DistributedTab::Consensus => DistributedTab::PeerGroups,
            DistributedTab::PeerGroups => DistributedTab::RateLimits,
            DistributedTab::RateLimits => DistributedTab::PublicMonitors,
        };
    }

    pub fn prev_distributed_tab(&mut self) {
        use super::ui::distributed::DistributedTab;
        self.distributed_tab = match self.distributed_tab {
            DistributedTab::PublicMonitors => DistributedTab::RateLimits,
            DistributedTab::Consensus => DistributedTab::PublicMonitors,
            DistributedTab::PeerGroups => DistributedTab::Consensus,
            DistributedTab::RateLimits => DistributedTab::PeerGroups,
        };
    }

    /// Navigate admin keys tabs
    pub fn next_admin_keys_tab(&mut self) {
        use super::ui::admin_keys::AdminKeysTab;
        self.admin_keys_tab = match self.admin_keys_tab {
            AdminKeysTab::Status => AdminKeysTab::KeyList,
            AdminKeysTab::KeyList => AdminKeysTab::RotationHistory,
            AdminKeysTab::RotationHistory => AdminKeysTab::Status,
        };
    }

    pub fn prev_admin_keys_tab(&mut self) {
        use super::ui::admin_keys::AdminKeysTab;
        self.admin_keys_tab = match self.admin_keys_tab {
            AdminKeysTab::Status => AdminKeysTab::RotationHistory,
            AdminKeysTab::KeyList => AdminKeysTab::Status,
            AdminKeysTab::RotationHistory => AdminKeysTab::KeyList,
        };
    }
}
