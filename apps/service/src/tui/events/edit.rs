use anyhow::Result;
use crossterm::event::KeyCode;

use crate::database::{Database, DatabaseImpl};
use crate::tui::state::AppState;

/// Handle keyboard events in edit popup
pub async fn handle_edit_popup(
    state: &mut AppState,
    key_code: KeyCode,
    db: &DatabaseImpl,
) -> Result<()> {
    match key_code {
        KeyCode::Esc => {
            state.close_edit();
        }

        // Navigation between fields - vim-style (j/k) and arrows
        KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
            state.edit_field_index = (state.edit_field_index + 1) % 6;
            state.text_cursor = 0;
            state.update_text_cursor();
            state.validation_error = None;
        }

        KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
            state.edit_field_index = state.edit_field_index.checked_sub(1).unwrap_or(5);
            state.text_cursor = 0;
            state.update_text_cursor();
            state.validation_error = None;
        }

        // Text cursor movement - Home/End
        KeyCode::Home => {
            if state.edit_field_index < 2 {
                state.text_cursor = 0;
            }
        }

        KeyCode::End => {
            if state.edit_field_index < 2 {
                if let Some(text) = state.get_current_field_text() {
                    state.text_cursor = text.len();
                }
            }
        }

        // Backspace in text fields
        KeyCode::Backspace => {
            if let Some(m) = state.edit_monitor.as_mut() {
                match state.edit_field_index {
                    0 => {
                        if state.text_cursor > 0 && state.text_cursor <= m.name.len() {
                            m.name.remove(state.text_cursor - 1);
                            state.text_cursor -= 1;
                        }
                    }
                    1 => {
                        if state.text_cursor > 0 && state.text_cursor <= m.target.len() {
                            m.target.remove(state.text_cursor - 1);
                            state.text_cursor -= 1;
                        }
                    }
                    _ => {}
                }
            }
            state.validation_error = None;
        }

        // Delete in text fields
        KeyCode::Delete => {
            if let Some(m) = state.edit_monitor.as_mut() {
                match state.edit_field_index {
                    0 => {
                        if state.text_cursor < m.name.len() {
                            m.name.remove(state.text_cursor);
                        }
                    }
                    1 => {
                        if state.text_cursor < m.target.len() {
                            m.target.remove(state.text_cursor);
                        }
                    }
                    _ => {}
                }
            }
            state.validation_error = None;
        }

        // Save monitor
        // Note: Enter saves from any field (including text fields).
        // Users can use Tab/Arrow keys to navigate between fields before saving.
        KeyCode::Enter | KeyCode::Char('s') => {
            // Only allow 's' as Save if not in text input
            if state.edit_field_index < 2 && matches!(key_code, KeyCode::Char('s')) {
                // 's' in text field, treat as text input
                if let Some(m) = state.edit_monitor.as_mut() {
                    if state.edit_field_index == 0 && state.text_cursor <= m.name.len() {
                        m.name.insert(state.text_cursor, 's');
                        state.text_cursor += 1;
                    } else if state.edit_field_index == 1 && state.text_cursor <= m.target.len() {
                        m.target.insert(state.text_cursor, 's');
                        state.text_cursor += 1;
                    }
                }
            } else {
                // Validate before saving
                if state.validate_current_monitor() {
                    if let Some(m) = state.edit_monitor.take() {
                        db.save_monitor(&m).await?;
                        state.close_edit();
                        state.refresh_monitors_and_results(db).await?;
                    }
                }
            }
        }

        // Arrow keys and vim keys for cursor/value adjustment
        // Handle these before general Char pattern to avoid unreachable pattern warning
        KeyCode::Right => {
            if state.edit_field_index < 2 {
                // Move cursor right in text field
                if let Some(text) = state.get_current_field_text() {
                    if state.text_cursor < text.len() {
                        state.text_cursor += 1;
                    }
                }
            } else {
                // Adjust values in other fields
                if let Some(m) = state.edit_monitor.as_mut() {
                    match state.edit_field_index {
                        2 => {
                            m.check_type = match m.check_type.as_str() {
                                "http" => "https".into(),
                                "https" => "tcp".into(),
                                "tcp" => "icmp".into(),
                                _ => "http".into(),
                            };
                        }
                        3 => {
                            m.interval_seconds = m.interval_seconds.saturating_add(5);
                        }
                        4 => {
                            m.timeout_seconds = m.timeout_seconds.saturating_add(1);
                        }
                        _ => {}
                    }
                }
            }
        }

        KeyCode::Left => {
            if state.edit_field_index < 2 {
                // Move cursor left in text field
                if state.text_cursor > 0 {
                    state.text_cursor -= 1;
                }
            } else {
                // Adjust values in other fields
                if let Some(m) = state.edit_monitor.as_mut() {
                    match state.edit_field_index {
                        2 => {
                            m.check_type = match m.check_type.as_str() {
                                "https" => "http".into(),
                                "tcp" => "https".into(),
                                "icmp" => "tcp".into(),
                                _ => "icmp".into(),
                            };
                        }
                        3 => {
                            m.interval_seconds = m.interval_seconds.saturating_sub(5).max(1);
                        }
                        4 => {
                            m.timeout_seconds = m.timeout_seconds.saturating_sub(1).max(1);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Character input and special commands
        KeyCode::Char(ch) => {
            // Text input in Name (0) and Target (1) fields
            if state.edit_field_index < 2 {
                if let Some(m) = state.edit_monitor.as_mut() {
                    match state.edit_field_index {
                        0 => {
                            if state.text_cursor <= m.name.len() {
                                m.name.insert(state.text_cursor, ch);
                                state.text_cursor += 1;
                            }
                        }
                        1 => {
                            if state.text_cursor <= m.target.len() {
                                m.target.insert(state.text_cursor, ch);
                                state.text_cursor += 1;
                            }
                        }
                        _ => {}
                    }
                }
                state.validation_error = None;
            } else {
                // Commands for non-text fields
                match ch {
                    'c' => {
                        if let Some(m) = state.edit_monitor.as_mut() {
                            m.check_type = match m.check_type.as_str() {
                                "http" => "https".into(),
                                "https" => "tcp".into(),
                                "tcp" => "icmp".into(),
                                _ => "http".into(),
                            };
                        }
                    }
                    'h' => {
                        // Vim left in non-text fields
                        if let Some(m) = state.edit_monitor.as_mut() {
                            match state.edit_field_index {
                                2 => {
                                    m.check_type = match m.check_type.as_str() {
                                        "https" => "http".into(),
                                        "tcp" => "https".into(),
                                        "icmp" => "tcp".into(),
                                        _ => "icmp".into(),
                                    };
                                }
                                3 => {
                                    m.interval_seconds =
                                        m.interval_seconds.saturating_sub(5).max(1);
                                }
                                4 => {
                                    m.timeout_seconds = m.timeout_seconds.saturating_sub(1).max(1);
                                }
                                _ => {}
                            }
                        }
                    }
                    'l' => {
                        // Vim right in non-text fields
                        if let Some(m) = state.edit_monitor.as_mut() {
                            match state.edit_field_index {
                                2 => {
                                    m.check_type = match m.check_type.as_str() {
                                        "http" => "https".into(),
                                        "https" => "tcp".into(),
                                        "tcp" => "icmp".into(),
                                        _ => "http".into(),
                                    };
                                }
                                3 => {
                                    m.interval_seconds = m.interval_seconds.saturating_add(5);
                                }
                                4 => {
                                    m.timeout_seconds = m.timeout_seconds.saturating_add(1);
                                }
                                _ => {}
                            }
                        }
                    }
                    '+' => {
                        if let Some(m) = state.edit_monitor.as_mut() {
                            if state.edit_field_index == 3 {
                                m.interval_seconds = m.interval_seconds.saturating_add(5);
                            }
                        }
                    }
                    '-' => {
                        if let Some(m) = state.edit_monitor.as_mut() {
                            if state.edit_field_index == 3 {
                                m.interval_seconds = m.interval_seconds.saturating_sub(5).max(1);
                            }
                        }
                    }
                    '[' => {
                        if let Some(m) = state.edit_monitor.as_mut() {
                            if state.edit_field_index == 4 {
                                m.timeout_seconds = m.timeout_seconds.saturating_sub(1).max(1);
                            }
                        }
                    }
                    ']' => {
                        if let Some(m) = state.edit_monitor.as_mut() {
                            if state.edit_field_index == 4 {
                                m.timeout_seconds = m.timeout_seconds.saturating_add(1);
                            }
                        }
                    }
                    ' ' => {
                        if let Some(m) = state.edit_monitor.as_mut() {
                            if state.edit_field_index == 5 {
                                m.enabled = !m.enabled;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        _ => {}
    }

    Ok(())
}
