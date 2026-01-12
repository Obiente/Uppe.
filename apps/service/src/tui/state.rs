use std::time::Instant;
use crate::database::models::{Monitor, MonitorResult};
use crate::monitoring::types::MonitorStatus;
use crate::validation;
use super::types::{Focus, FrameAreas};

/// Application state
pub struct AppState {
    pub monitors: Vec<Monitor>,
    pub selected: usize,
    pub results: Vec<MonitorResult>,
    pub show_help: bool,
    pub focus: Focus,
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
    #[allow(dead_code)]
    pub text_selection_start: Option<usize>,
    
    // Auto-refresh
    pub auto_refresh: bool,
    pub last_refresh: Instant,
    pub refresh_interval_secs: u64,
    
    // P2P status
    pub peer_id: String,
    pub p2p_enabled: bool,
    
    // Validation
    pub validation_error: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            monitors: Vec::new(),
            selected: 0,
            results: Vec::new(),
            show_help: false,
            focus: Focus::Monitors,
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
            validation_error: None,
        }
    }

    pub fn set_peer_info(&mut self, peer_id: String, enabled: bool) {
        self.peer_id = peer_id;
        self.p2p_enabled = enabled;
    }

    pub fn get_current_field_text(&self) -> Option<&str> {
        if let Some(m) = &self.edit_monitor {
            match self.edit_field_index {
                0 => Some(&m.name),
                1 => Some(&m.target),
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
            let timeout_result = validation::validate_timeout(m.timeout_seconds, m.interval_seconds);
            if !timeout_result.is_valid {
                self.validation_error = timeout_result.error;
                return false;
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
            self.selected = if self.selected == 0 {
                self.monitors.len() - 1
            } else {
                self.selected - 1
            };
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
    pub fn get_current_monitor_stats(&self) -> (f64, u64, u64, u64) {
        if self.monitors.is_empty() || self.selected >= self.monitors.len() {
            return (0.0, 0, 0, 0);
        }

        if self.results.is_empty() {
            return (0.0, 0, 0, 0);
        }

        let total = self.results.len() as u64;
        let successful = self.results.iter().filter(|r| r.status == MonitorStatus::Up).count() as u64;
        let uptime = if total > 0 { (successful as f64 / total as f64) * 100.0 } else { 0.0 };
        let (latency_sum, latency_count) = self.results
            .iter()
            .filter_map(|r| r.latency_ms)
            .fold((0u64, 0u64), |(sum, count), latency| (sum + latency, count + 1));
        let avg_latency = if latency_count > 0 {
            latency_sum / latency_count
        } else {
            0
        };

        (uptime, successful, total, avg_latency)
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
}
