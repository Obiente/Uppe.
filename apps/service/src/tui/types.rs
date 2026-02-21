use ratatui::layout::Rect;

/// View mode - different layouts for different purposes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Dashboard: 4-pane overview (Monitors, Results, Stats, Network)
    Dashboard,
    /// Distributed monitoring fullscreen
    Distributed,
    /// Monitoring statistics fullscreen
    Statistics,
    /// Network/P2P view fullscreen
    Network,
    /// DHT debug view (for nerds)
    DhtDebug,
    /// Admin keys management
    AdminKeys,
}

impl Default for ViewMode {
    fn default() -> Self {
        ViewMode::Dashboard
    }
}

impl ViewMode {
    pub fn next(&self) -> Self {
        match self {
            ViewMode::Dashboard => ViewMode::Distributed,
            ViewMode::Distributed => ViewMode::Statistics,
            ViewMode::Statistics => ViewMode::Network,
            ViewMode::Network => ViewMode::DhtDebug,
            ViewMode::DhtDebug => ViewMode::AdminKeys,
            ViewMode::AdminKeys => ViewMode::Dashboard,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            ViewMode::Dashboard => ViewMode::AdminKeys,
            ViewMode::AdminKeys => ViewMode::DhtDebug,
            ViewMode::DhtDebug => ViewMode::Network,
            ViewMode::Network => ViewMode::Statistics,
            ViewMode::Statistics => ViewMode::Distributed,
            ViewMode::Distributed => ViewMode::Dashboard,
        }
    }

}

/// Frame areas for mouse hit-testing
pub struct FrameAreas {
    #[allow(dead_code)] // May be used for future header interactions
    pub header: Rect,
    pub monitors: Rect,
    pub results: Rect,
    #[allow(dead_code)] // May be used for future footer interactions
    pub footer: Rect,
    pub action_buttons: Vec<(String, Rect)>,
}

/// Which panel has focus
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Monitors,
    Results,
    Stats,
    Network,
    Distributed,
}
