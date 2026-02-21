use anyhow::Result;
use crossterm::event::KeyCode;

use crate::database::models::MonitorVisibility;
use crate::database::{Database, DatabaseImpl};
use crate::tui::state::AppState;

/// Field layout indices that differ between Public and Private monitors
struct FieldLayout {
    text_fields: usize,   // Number of text-editable fields (name, target, [domain, display_name])
    check_type: usize,    // Index of check_type field
    interval: usize,      // Index of interval field
    timeout: usize,       // Index of timeout field
    enabled: usize,       // Index of enabled field
    total: usize,         // Total number of fields
}

impl FieldLayout {
    fn for_monitor(m: &crate::database::models::Monitor) -> Self {
        if matches!(m.visibility, MonitorVisibility::Public) {
            Self { text_fields: 4, check_type: 4, interval: 5, timeout: 6, enabled: 8, total: 9 }
        } else {
            Self { text_fields: 2, check_type: 2, interval: 3, timeout: 4, enabled: 6, total: 7 }
        }
    }
}

fn cycle_check_type_forward(current: &str) -> String {
    match current {
        "http" => "https".into(),
        "https" => "tcp".into(),
        "tcp" => "icmp".into(),
        _ => "http".into(),
    }
}

fn cycle_check_type_backward(current: &str) -> String {
    match current {
        "https" => "http".into(),
        "tcp" => "https".into(),
        "icmp" => "tcp".into(),
        _ => "icmp".into(),
    }
}

/// Apply a value adjustment (increment/decrement) to the current non-text field
fn adjust_field(state: &mut AppState, layout: &FieldLayout, forward: bool) {
    if let Some(m) = state.edit_monitor.as_mut() {
        let idx = state.edit_field_index;
        if idx == layout.check_type {
            m.check_type = if forward {
                cycle_check_type_forward(&m.check_type)
            } else {
                cycle_check_type_backward(&m.check_type)
            };
        } else if idx == layout.interval {
            if forward {
                m.interval_seconds = m.interval_seconds.saturating_add(5);
            } else {
                m.interval_seconds = m.interval_seconds.saturating_sub(5).max(1);
            }
        } else if idx == layout.timeout {
            if forward {
                m.timeout_seconds = m.timeout_seconds.saturating_add(1);
            } else {
                m.timeout_seconds = m.timeout_seconds.saturating_sub(1).max(1);
            }
        }
    }
}

/// Insert a character into the current text field at the cursor position
fn insert_char_at_cursor(state: &mut AppState, ch: char) {
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
            2 => {
                if let Some(domain) = m.public_domain.as_mut() {
                    if state.text_cursor <= domain.len() {
                        domain.insert(state.text_cursor, ch);
                        state.text_cursor += 1;
                    }
                } else if matches!(m.visibility, MonitorVisibility::Public) {
                    m.public_domain = Some(ch.to_string());
                    state.text_cursor = 1;
                }
            }
            3 => {
                if let Some(display_name) = m.public_display_name.as_mut() {
                    if state.text_cursor <= display_name.len() {
                        display_name.insert(state.text_cursor, ch);
                        state.text_cursor += 1;
                    }
                } else if matches!(m.visibility, MonitorVisibility::Public) {
                    m.public_display_name = Some(ch.to_string());
                    state.text_cursor = 1;
                }
            }
            _ => {}
        }
    }
    state.validation_error = None;
}

/// Remove a character before the cursor (backspace) in the current text field
fn backspace_at_cursor(state: &mut AppState) {
    if state.text_cursor == 0 { return; }
    if let Some(m) = state.edit_monitor.as_mut() {
        match state.edit_field_index {
            0 => {
                if state.text_cursor <= m.name.len() {
                    m.name.remove(state.text_cursor - 1);
                    state.text_cursor -= 1;
                }
            }
            1 => {
                if state.text_cursor <= m.target.len() {
                    m.target.remove(state.text_cursor - 1);
                    state.text_cursor -= 1;
                }
            }
            2 => {
                if let Some(domain) = m.public_domain.as_mut() {
                    if state.text_cursor <= domain.len() {
                        domain.remove(state.text_cursor - 1);
                        state.text_cursor -= 1;
                    }
                }
            }
            3 => {
                if let Some(dn) = m.public_display_name.as_mut() {
                    if state.text_cursor <= dn.len() {
                        dn.remove(state.text_cursor - 1);
                        state.text_cursor -= 1;
                    }
                }
            }
            _ => {}
        }
    }
    state.validation_error = None;
}

/// Remove a character at the cursor (delete) in the current text field
fn delete_at_cursor(state: &mut AppState) {
    if let Some(m) = state.edit_monitor.as_mut() {
        match state.edit_field_index {
            0 => { if state.text_cursor < m.name.len() { m.name.remove(state.text_cursor); } }
            1 => { if state.text_cursor < m.target.len() { m.target.remove(state.text_cursor); } }
            2 => {
                if let Some(domain) = m.public_domain.as_mut() {
                    if state.text_cursor < domain.len() { domain.remove(state.text_cursor); }
                }
            }
            3 => {
                if let Some(dn) = m.public_display_name.as_mut() {
                    if state.text_cursor < dn.len() { dn.remove(state.text_cursor); }
                }
            }
            _ => {}
        }
    }
    state.validation_error = None;
}

/// Handle keyboard events in edit popup
pub async fn handle_edit_popup(
    state: &mut AppState,
    key_code: KeyCode,
    db: &DatabaseImpl,
) -> Result<()> {
    // Compute layout once; bail if no monitor being edited
    let layout = match &state.edit_monitor {
        Some(m) => FieldLayout::for_monitor(m),
        None => return Ok(()),
    };

    match key_code {
        KeyCode::Esc => {
            state.close_edit();
        }

        // Navigation between fields
        KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
            state.edit_field_index = (state.edit_field_index + 1) % layout.total;
            state.text_cursor = 0;
            state.update_text_cursor();
            state.validation_error = None;
        }

        KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
            state.edit_field_index = state.edit_field_index.checked_sub(1).unwrap_or(layout.total - 1);
            state.text_cursor = 0;
            state.update_text_cursor();
            state.validation_error = None;
        }

        KeyCode::Home => {
            if state.edit_field_index < layout.text_fields {
                state.text_cursor = 0;
            }
        }

        KeyCode::End => {
            if state.edit_field_index < layout.text_fields {
                if let Some(text) = state.get_current_field_text() {
                    state.text_cursor = text.len();
                }
            }
        }

        KeyCode::Backspace => backspace_at_cursor(state),
        KeyCode::Delete => delete_at_cursor(state),

        // Save monitor
        KeyCode::Enter | KeyCode::Char('s') => {
            // 's' in text field should type 's', not save
            if state.edit_field_index < layout.text_fields && matches!(key_code, KeyCode::Char('s')) {
                insert_char_at_cursor(state, 's');
            } else {
                // Validate and save
                if state.validate_current_monitor() {
                    if let Some(m) = state.edit_monitor.take() {
                        match db.save_monitor(&m).await {
                            Ok(_) => {
                                state.close_edit();
                                if let Err(e) = state.refresh_monitors_and_results(db).await {
                                    tracing::warn!("Failed to refresh after save: {}", e);
                                }
                                state.set_status("Monitor saved", crate::tui::state::StatusLevel::Success);
                            }
                            Err(e) => {
                                state.validation_error = Some(format!("Save failed: {}", e));
                                state.edit_monitor = Some(m);
                            }
                        }
                    }
                }
            }
        }

        KeyCode::Right => {
            if state.edit_field_index < layout.text_fields {
                if let Some(text) = state.get_current_field_text() {
                    if state.text_cursor < text.len() {
                        state.text_cursor += 1;
                    }
                }
            } else {
                adjust_field(state, &layout, true);
            }
        }

        KeyCode::Left => {
            if state.edit_field_index < layout.text_fields {
                if state.text_cursor > 0 {
                    state.text_cursor -= 1;
                }
            } else {
                adjust_field(state, &layout, false);
            }
        }

        KeyCode::Char(ch) => {
            if state.edit_field_index < layout.text_fields {
                insert_char_at_cursor(state, ch);
            } else {
                // Commands for non-text fields
                match ch {
                    'c' => {
                        if state.edit_field_index == layout.check_type {
                            if let Some(m) = state.edit_monitor.as_mut() {
                                m.check_type = cycle_check_type_forward(&m.check_type);
                            }
                        }
                    }
                    'h' => adjust_field(state, &layout, false),
                    'l' => adjust_field(state, &layout, true),
                    '+' => {
                        if state.edit_field_index == layout.interval {
                            if let Some(m) = state.edit_monitor.as_mut() {
                                m.interval_seconds = m.interval_seconds.saturating_add(5);
                            }
                        }
                    }
                    '-' => {
                        if state.edit_field_index == layout.interval {
                            if let Some(m) = state.edit_monitor.as_mut() {
                                m.interval_seconds = m.interval_seconds.saturating_sub(5).max(1);
                            }
                        }
                    }
                    '[' => {
                        if state.edit_field_index == layout.timeout {
                            if let Some(m) = state.edit_monitor.as_mut() {
                                m.timeout_seconds = m.timeout_seconds.saturating_sub(1).max(1);
                            }
                        }
                    }
                    ']' => {
                        if state.edit_field_index == layout.timeout {
                            if let Some(m) = state.edit_monitor.as_mut() {
                                m.timeout_seconds = m.timeout_seconds.saturating_add(1);
                            }
                        }
                    }
                    ' ' => {
                        if state.edit_field_index == layout.enabled {
                            if let Some(m) = state.edit_monitor.as_mut() {
                                m.enabled = !m.enabled;
                            }
                        }
                    }
                    'v' | 'V' => {
                        if let Some(m) = state.edit_monitor.as_mut() {
                            m.visibility = match m.visibility {
                                MonitorVisibility::Public => {
                                    m.public_domain = None;
                                    m.public_display_name = None;
                                    if state.edit_field_index >= 7 {
                                        state.edit_field_index = 6;
                                    }
                                    MonitorVisibility::Private
                                }
                                MonitorVisibility::Private => {
                                    m.public_domain = Some(String::new());
                                    m.public_display_name = Some(String::new());
                                    MonitorVisibility::Public
                                }
                                MonitorVisibility::Internal => MonitorVisibility::Public,
                            };
                            state.validation_error = None;
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
