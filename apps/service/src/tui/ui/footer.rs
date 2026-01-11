use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Clear};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: Rect, show_help: bool) -> Vec<(String, Rect)> {
    let mut action_buttons: Vec<(String, Rect)> = Vec::new();
    
    if !show_help {
        // Clear the footer area first
        f.render_widget(Clear, area);
        
        let footer_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(16),
                Constraint::Percentage(14),
                Constraint::Percentage(16),
                Constraint::Percentage(16),
                Constraint::Percentage(16),
                Constraint::Percentage(22),
            ])
            .split(area);

        let labels = ["Add", "Edit", "Delete", "Refresh", "Help", "Quit"];
        let keys = ["A", "E", "D", "R", "H/?", "Q/Esc"];
        
        for (i, (label, key)) in labels.iter().zip(keys.iter()).enumerate() {
            let text = format!("{}: {}", key, label);
            let btn = Paragraph::new(Line::from(Span::styled(
                text,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )))
            .alignment(Alignment::Center);
            
            f.render_widget(btn, footer_chunks[i]);
            action_buttons.push((label.to_string(), footer_chunks[i]));
        }
    }
    
    action_buttons
}
