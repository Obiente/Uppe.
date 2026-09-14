use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use super::{COLOR_BRAND, COLOR_LABEL};
use crate::tui::state::AppState;
use crate::tui::types::ViewMode;

/// Render a single key hint: key highlighted, label plain
fn hint<'a>(key: &'a str, label: &'a str) -> Vec<Span<'a>> {
    vec![
        Span::styled(key, Style::default().fg(COLOR_BRAND).add_modifier(Modifier::BOLD)),
        Span::styled(format!(":{label} "), Style::default().fg(COLOR_LABEL)),
    ]
}

pub fn render(f: &mut Frame, area: Rect, state: &AppState) -> Vec<(String, Rect)> {
    if state.show_help {
        return vec![];
    }

    f.render_widget(Clear, area);

    let mut spans: Vec<Span> = Vec::new();

    // Common hints
    spans.extend(hint("Tab", "views"));

    match state.view_mode {
        ViewMode::Dashboard => {
            spans.extend(hint("a/A", "add"));
            spans.extend(hint("e", "edit"));
            spans.extend(hint("d", "del"));
            spans.extend(hint("r", "refresh"));
            spans.extend(hint("Enter", "detail"));
        }
        ViewMode::Distributed | ViewMode::AdminKeys => {
            spans.extend(hint("</>", "tabs"));
            spans.extend(hint("j/k", "nav"));
        }
        ViewMode::DhtDebug => {
            spans.extend(hint("x", "query"));
            spans.extend(hint("g", "custom"));
            spans.extend(hint("j/k", "bucket"));
        }
        ViewMode::Statistics | ViewMode::Network => {
            spans.extend(hint("r", "refresh"));
        }
    }

    spans.extend(hint("?", "help"));
    spans.extend(hint("q", "quit"));

    let footer = Paragraph::new(Line::from(spans));
    f.render_widget(footer, area);

    vec![]
}
