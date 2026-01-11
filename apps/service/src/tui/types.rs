use ratatui::layout::Rect;

/// Frame areas for mouse hit-testing
#[allow(dead_code)]
pub struct FrameAreas {
    pub header: Rect,
    pub monitors: Rect,
    pub results: Rect,
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
