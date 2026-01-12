use ratatui::layout::Rect;

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
}
