use crate::io::{save_project, save_project_to_output};
use crate::models::EnvironmentType;

use super::{Action, AppState, Dialog, DialogAction, InputMode, MessageLevel, Section, View};

/// Process an action and update state (TEA pattern)
pub fn update(state: &mut AppState, action: Action) -> Option<Action> {
    match action {
        Action::NavigateUp => {
            if state.dialog.is_some() {
                return Some(Action::DialogUp);
            }
            match state.current_view {
                View::ProjectList => {
                    if state.selected_project_index > 0 {
                        state.selected_project_index -= 1;
                    }
                }
                View::ProjectDetail => {
                    if state.selected_key_index > 0 {
                        state.selected_key_index -= 1;
                    }
                }
                View::KeyEditor => {
                    // In key editor, up/down navigates between env fields
                    if state.selected_key_index > 0 {
                        state.selected_key_index -= 1;
                    }
                }
            }
            None
        }

        Action::NavigateDown => {
            if state.dialog.is_some() {
                return Some(Action::DialogDown);
            }
            match state.current_view {
                View::ProjectList => {
                    if state.selected_project_index < state.projects.len().saturating_sub(1) {
                        state.selected_project_index += 1;
                    }
                }
                View::ProjectDetail => {
                    let key_count = state.current_section_item_count();
                    if state.selected_key_index < key_count.saturating_sub(1) {
                        state.selected_key_index += 1;
                    }
                }
                View::KeyEditor => {
                    // Navigate between target + env fields
                    if let Some(project) = state.current_project() {
                        let slot_count = 1 + project.named_env_types().len(); // target + named
                        if state.selected_key_index < slot_count.saturating_sub(1) {
                            state.selected_key_index += 1;
                        }
                    }
                }
            }
            None
        }

        Action::NavigateLeft | Action::CycleEnvBack => {
            if state.current_view == View::ProjectDetail {
                cycle_env_value(state, false);
            }
            None
        }

        Action::NavigateRight | Action::CycleEnv => {
            if state.current_view == View::ProjectDetail {
                cycle_env_value(state, true);
            }
            None
        }

        Action::MoveUp => {
            if state.current_view == View::ProjectDetail && state.selected_section == Section::CurrentConfig {
                let current_index = state.selected_key_index;
                if let Some(project) = state.current_project_mut() {
                    if let Some(new_index) = project.move_item_up(current_index) {
                        state.selected_key_index = new_index;
                        state.show_message("Moved up".to_string(), MessageLevel::Info);
                    }
                }
            }
            None
        }

        Action::MoveDown => {
            if state.current_view == View::ProjectDetail && state.selected_section == Section::CurrentConfig {
                let current_index = state.selected_key_index;
                if let Some(project) = state.current_project_mut() {
                    if let Some(new_index) = project.move_item_down(current_index) {
                        state.selected_key_index = new_index;
                        state.show_message("Moved down".to_string(), MessageLevel::Info);
                    }
                }
            }
            None
        }

        Action::MoveIntoSection => {
            if state.current_view == View::ProjectDetail && state.selected_section == Section::CurrentConfig {
                let current_index = state.selected_key_index;
                if let Some(project) = state.current_project_mut() {
                    if let Some(new_index) = project.move_key_into_section_above(current_index) {
                        state.selected_key_index = new_index;
                        state.show_message("Moved into section".to_string(), MessageLevel::Info);
                    }
                }
            }
            None
        }

        Action::MoveOutOfSection => {
            if state.current_view == View::ProjectDetail && state.selected_section == Section::CurrentConfig {
                let current_index = state.selected_key_index;
                if let Some(project) = state.current_project_mut() {
                    if let Some(new_index) = project.move_key_out_of_section(current_index) {
                        state.selected_key_index = new_index;
                        state.show_message("Moved out of section".to_string(), MessageLevel::Info);
                    }
                }
            }
            None
        }

        Action::NextSection => {
            if state.current_view == View::ProjectDetail {
                state.next_section();
            }
            None
        }

        Action::PrevSection => {
            if state.current_view == View::ProjectDetail {
                state.prev_section();
            }
            None
        }

        Action::Select => {
            if state.dialog.is_some() {
                return Some(Action::ConfirmDialog);
            }
            match state.current_view {
                View::ProjectList => {
                    if !state.projects.is_empty() {
                        state.current_view = View::ProjectDetail;
                        state.reset_detail_selection();
                    }
                }
                View::ProjectDetail => {
                    // Open key editor
                    if let Some(key) = state.current_key() {
                        state.editing_key = Some(key);
                        state.current_view = View::KeyEditor;
                        state.selected_key_index = 0; // Reset to first env in editor
                        state.input_mode = InputMode::Normal;
                    }
                }
                View::KeyEditor => {
                    return Some(Action::StartEdit);
                }
            }
            None
        }

        Action::Back => {
            if state.dialog.is_some() {
                return Some(Action::CloseDialog);
            }
            if state.input_mode == InputMode::Editing {
                return Some(Action::CancelEdit);
            }
            match state.current_view {
                View::ProjectDetail => {
                    state.current_view = View::ProjectList;
                }
                View::KeyEditor => {
                    state.current_view = View::ProjectDetail;
                    state.selected_key_index = 0;
                    state.editing_key = None;
                }
                View::ProjectList => {}
            }
            None
        }

        Action::StartEdit => {
            if state.current_view == View::KeyEditor {
                // Get current value for the selected env slot
                if let Some(ref key) = state.editing_key.clone() {
                    let value = get_editor_slot_value(state, key);
                    state.cursor_pos = value.len(); // Start cursor at end
                    state.input_buffer = value;
                    state.input_mode = InputMode::Editing;
                }
            }
            None
        }

        Action::ConfirmEdit => {
            if state.input_mode == InputMode::Editing {
                if let Some(key) = state.editing_key.clone() {
                    let value = state.input_buffer.clone();
                    state.input_mode = InputMode::Normal;
                    state.input_buffer.clear();
                    state.cursor_pos = 0;

                    // Determine which slot was being edited
                    // Slot layout: 0 = target (.env), 1..n = named envs
                    if let Some(project) = state.current_project() {
                        let env_types = project.named_env_types();
                        let slot_index = state.selected_key_index;

                        if slot_index == 0 {
                            // Editing target (.env) value
                            return Some(Action::SetCustomValue { key, value });
                        } else if slot_index <= env_types.len() {
                            // Editing a named env value (indices 1..n)
                            let env_type = env_types[slot_index - 1].clone();
                            return Some(Action::SetValue { key, env: env_type, value });
                        }
                    }
                }
            }
            None
        }

        Action::CancelEdit => {
            state.input_mode = InputMode::Normal;
            state.input_buffer.clear();
            state.cursor_pos = 0;
            None
        }

        Action::SetValue { key, env, value } => {
            if let Some(project) = state.current_project_mut() {
                if let Some(environment) = project.environments.get_mut(&env) {
                    environment.set(key.clone(), value);
                    state.show_message(format!("Updated {} in {}", key, env), MessageLevel::Success);
                }
            }
            None
        }

        Action::SetCustomValue { key, value } => {
            if let Some(project) = state.current_project_mut() {
                project.set_default_value(key.clone(), value);
                state.show_message(format!("Set custom value for {}", key), MessageLevel::Success);
            }
            None
        }

        Action::InputChar(c) => {
            if state.input_mode == InputMode::Editing {
                state.input_buffer.insert(state.cursor_pos, c);
                state.cursor_pos += 1;
            } else if let Some(Dialog::AddKey { ref mut key, ref mut value, focus_on_value, ref mut key_cursor_pos, ref mut value_cursor_pos }) = state.dialog {
                if focus_on_value {
                    value.insert(*value_cursor_pos, c);
                    *value_cursor_pos += 1;
                } else {
                    key.insert(*key_cursor_pos, c);
                    *key_cursor_pos += 1;
                }
            } else if let Some(Dialog::Input { ref mut value, ref mut cursor_pos, .. }) = state.dialog {
                value.insert(*cursor_pos, c);
                *cursor_pos += 1;
            }
            None
        }

        Action::InputBackspace => {
            if state.input_mode == InputMode::Editing {
                if state.cursor_pos > 0 {
                    state.input_buffer.remove(state.cursor_pos - 1);
                    state.cursor_pos -= 1;
                }
            } else if let Some(Dialog::AddKey { ref mut key, ref mut value, focus_on_value, ref mut key_cursor_pos, ref mut value_cursor_pos }) = state.dialog {
                if focus_on_value {
                    if *value_cursor_pos > 0 {
                        value.remove(*value_cursor_pos - 1);
                        *value_cursor_pos -= 1;
                    }
                } else {
                    if *key_cursor_pos > 0 {
                        key.remove(*key_cursor_pos - 1);
                        *key_cursor_pos -= 1;
                    }
                }
            } else if let Some(Dialog::Input { ref mut value, ref mut cursor_pos, .. }) = state.dialog {
                if *cursor_pos > 0 {
                    value.remove(*cursor_pos - 1);
                    *cursor_pos -= 1;
                }
            }
            None
        }

        Action::InputDelete => {
            if state.input_mode == InputMode::Editing {
                if state.cursor_pos < state.input_buffer.len() {
                    state.input_buffer.remove(state.cursor_pos);
                }
            } else if let Some(Dialog::AddKey { ref mut key, ref mut value, focus_on_value, key_cursor_pos, value_cursor_pos }) = state.dialog {
                if focus_on_value {
                    if value_cursor_pos < value.len() {
                        value.remove(value_cursor_pos);
                    }
                } else {
                    if key_cursor_pos < key.len() {
                        key.remove(key_cursor_pos);
                    }
                }
            } else if let Some(Dialog::Input { ref mut value, cursor_pos, .. }) = state.dialog {
                if cursor_pos < value.len() {
                    value.remove(cursor_pos);
                }
            }
            None
        }

        Action::InputLeft => {
            if state.input_mode == InputMode::Editing {
                if state.cursor_pos > 0 {
                    state.cursor_pos -= 1;
                }
            } else if let Some(Dialog::AddKey { focus_on_value, ref mut key_cursor_pos, ref mut value_cursor_pos, .. }) = state.dialog {
                if focus_on_value {
                    if *value_cursor_pos > 0 {
                        *value_cursor_pos -= 1;
                    }
                } else {
                    if *key_cursor_pos > 0 {
                        *key_cursor_pos -= 1;
                    }
                }
            } else if let Some(Dialog::Input { ref mut cursor_pos, .. }) = state.dialog {
                if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                }
            }
            None
        }

        Action::InputRight => {
            if state.input_mode == InputMode::Editing {
                if state.cursor_pos < state.input_buffer.len() {
                    state.cursor_pos += 1;
                }
            } else if let Some(Dialog::AddKey { ref key, ref value, focus_on_value, ref mut key_cursor_pos, ref mut value_cursor_pos }) = state.dialog {
                if focus_on_value {
                    if *value_cursor_pos < value.len() {
                        *value_cursor_pos += 1;
                    }
                } else {
                    if *key_cursor_pos < key.len() {
                        *key_cursor_pos += 1;
                    }
                }
            } else if let Some(Dialog::Input { ref value, ref mut cursor_pos, .. }) = state.dialog {
                if *cursor_pos < value.len() {
                    *cursor_pos += 1;
                }
            }
            None
        }

        Action::InputHome => {
            if state.input_mode == InputMode::Editing {
                state.cursor_pos = 0;
            } else if let Some(Dialog::AddKey { focus_on_value, ref mut key_cursor_pos, ref mut value_cursor_pos, .. }) = state.dialog {
                if focus_on_value {
                    *value_cursor_pos = 0;
                } else {
                    *key_cursor_pos = 0;
                }
            } else if let Some(Dialog::Input { ref mut cursor_pos, .. }) = state.dialog {
                *cursor_pos = 0;
            }
            None
        }

        Action::InputEnd => {
            if state.input_mode == InputMode::Editing {
                state.cursor_pos = state.input_buffer.len();
            } else if let Some(Dialog::AddKey { ref key, ref value, focus_on_value, ref mut key_cursor_pos, ref mut value_cursor_pos }) = state.dialog {
                if focus_on_value {
                    *value_cursor_pos = value.len();
                } else {
                    *key_cursor_pos = key.len();
                }
            } else if let Some(Dialog::Input { ref value, ref mut cursor_pos, .. }) = state.dialog {
                *cursor_pos = value.len();
            }
            None
        }

        Action::AddKey => {
            if state.current_view == View::ProjectDetail {
                state.dialog = Some(Dialog::AddKey {
                    key: String::new(),
                    value: String::new(),
                    focus_on_value: false,
                    key_cursor_pos: 0,
                    value_cursor_pos: 0,
                });
            }
            None
        }

        Action::AddKeyFromMissing => {
            // Enable an inactive key by adding it to .env with value from first available env
            if state.is_current_item_inactive() {
                if let Some(key) = state.current_key() {
                    let key_clone = key.clone();

                    // Find first env that has this key
                    let first_env = state.current_project()
                        .and_then(|p| p.envs_with_key(&key_clone).first().cloned().cloned());

                    if let Some(env_type) = first_env {
                        let env_type_clone = env_type.clone();
                        if let Some(project) = state.current_project_mut() {
                            project.set_value_from_env(&key_clone, &env_type_clone);
                            project.rebuild_keys_cache();
                        }
                        state.show_message(format!("Enabled {} from {}", key_clone, env_type_clone), MessageLevel::Success);
                    }
                }
            }
            None
        }

        Action::DeleteKey => {
            if let Some(key) = state.current_key() {
                state.dialog = Some(Dialog::Confirm {
                    title: "Delete Key".to_string(),
                    message: format!("Delete '{}' from all environments?", key),
                    on_confirm: DialogAction::DeleteKey { key },
                });
            }
            None
        }

        Action::DisableKey { key } => {
            // Remove key from .env (target) but keep in other envs
            if let Some(project) = state.current_project_mut() {
                project.delete_key_from_env(&key, &EnvironmentType::Default);
                state.show_message(format!("Disabled {} (removed from .env)", key), MessageLevel::Info);
            }
            None
        }

        Action::EnableKey { key, from_env } => {
            // Add inactive key to .env from a specific environment
            if let Some(project) = state.current_project_mut() {
                project.set_value_from_env(&key, &from_env);
                project.rebuild_keys_cache();
                state.show_message(format!("Enabled {} from {}", key, from_env), MessageLevel::Success);
            }
            None
        }

        Action::BulkSwitch => {
            state.dialog = Some(Dialog::BulkSwitch { selected_env_index: 0 });
            state.dialog_scroll = 0;
            None
        }

        Action::OpenEnvMenu => {
            // Open environment selection menu for current key
            if state.current_view == View::ProjectDetail && state.selected_section == Section::CurrentConfig {
                if let Some(key) = state.current_key() {
                    if let Some(project) = state.current_project() {
                        let mut options: Vec<super::EnvOption> = Vec::new();

                        // Add all named environments as options
                        for env_type in project.named_env_types() {
                            if let Some(value) = project.get_value(&key, env_type) {
                                options.push(super::EnvOption {
                                    env_type: Some(env_type.clone()),
                                    value: value.to_string(),
                                    label: env_type.display_name().to_string(),
                                });
                            }
                        }

                        // Add "Disable" option if key is currently active (has value in .env)
                        let is_active = project.get_value(&key, &EnvironmentType::Default).is_some();
                        if is_active {
                            options.push(super::EnvOption {
                                env_type: None, // None indicates "Disable" option
                                value: "(remove from .env)".to_string(),
                                label: "Disable".to_string(),
                            });
                        }

                        // Only open dialog if there are options
                        if !options.is_empty() {
                            // Find current match to pre-select and mark as current
                            let current_match = project.find_matching_env(&key);
                            let current_index = current_match
                                .as_ref()
                                .and_then(|m| options.iter().position(|o| o.env_type.as_ref() == Some(m)));
                            let selected_index = current_index.unwrap_or(0);

                            state.dialog = Some(Dialog::SelectEnv {
                                key,
                                selected_index,
                                current_index,
                                options,
                            });
                            state.dialog_scroll = 0;
                        }
                    }
                }
            }
            None
        }

        Action::OpenSectionEnvMenu { section } => {
            // Open section env selection dialog - only show envs that have this section
            if state.current_view == View::ProjectDetail {
                if let Some(project) = state.current_project() {
                    let envs_with_section = project.envs_with_section(&section);

                    let options: Vec<super::EnvOption> = envs_with_section
                        .iter()
                        .map(|env_type| super::EnvOption {
                            env_type: Some((*env_type).clone()),
                            value: String::new(),
                            label: env_type.display_name().to_string(),
                        })
                        .collect();

                    if !options.is_empty() {
                        // Find which envs currently match this section
                        let section_matching = project.find_section_matching_envs(&section);
                        let current_indices: Vec<usize> = options
                            .iter()
                            .enumerate()
                            .filter(|(_, opt)| {
                                opt.env_type
                                    .as_ref()
                                    .map(|e| section_matching.contains(e))
                                    .unwrap_or(false)
                            })
                            .map(|(i, _)| i)
                            .collect();

                        // Pre-select first matching env, or first option
                        let selected_index = current_indices.first().copied().unwrap_or(0);

                        state.dialog = Some(Dialog::SelectSectionEnv {
                            section,
                            selected_index,
                            current_indices,
                            options,
                        });
                        state.dialog_scroll = 0;
                    }
                }
            }
            None
        }

        Action::SwitchSection { section, env } => {
            if let Some(project) = state.current_project_mut() {
                project.switch_section(&section, &env);
                state.show_message(
                    format!("Switched [{}] to {} values", section, env),
                    MessageLevel::Success,
                );
            }
            None
        }

        Action::DialogUp => {
            match &mut state.dialog {
                Some(Dialog::BulkSwitch { selected_env_index }) => {
                    if *selected_env_index > 0 {
                        *selected_env_index -= 1;
                    }
                }
                Some(Dialog::SelectEnv { selected_index, .. }) => {
                    if *selected_index > 0 {
                        *selected_index -= 1;
                    }
                }
                Some(Dialog::SelectSectionEnv { selected_index, .. }) => {
                    if *selected_index > 0 {
                        *selected_index -= 1;
                    }
                }
                _ => {}
            }
            None
        }

        Action::DialogDown => {
            let max_index = match &state.dialog {
                Some(Dialog::BulkSwitch { .. }) => {
                    state.current_project()
                        .map(|p| p.named_env_types().len().saturating_sub(1))
                        .unwrap_or(0)
                }
                Some(Dialog::SelectEnv { options, .. }) => {
                    options.len().saturating_sub(1)
                }
                Some(Dialog::SelectSectionEnv { options, .. }) => {
                    options.len().saturating_sub(1)
                }
                _ => 0,
            };

            match &mut state.dialog {
                Some(Dialog::BulkSwitch { selected_env_index }) => {
                    if *selected_env_index < max_index {
                        *selected_env_index += 1;
                    }
                }
                Some(Dialog::SelectEnv { selected_index, .. }) => {
                    if *selected_index < max_index {
                        *selected_index += 1;
                    }
                }
                Some(Dialog::SelectSectionEnv { selected_index, .. }) => {
                    if *selected_index < max_index {
                        *selected_index += 1;
                    }
                }
                _ => {}
            }
            None
        }

        Action::DialogTab => {
            if let Some(Dialog::AddKey { ref mut focus_on_value, .. }) = state.dialog {
                *focus_on_value = !*focus_on_value;
            }
            None
        }

        Action::ConfirmDialog => {
            if let Some(dialog) = state.dialog.take() {
                match dialog {
                    Dialog::Confirm { on_confirm, .. } => match on_confirm {
                        DialogAction::DeleteKey { key } => {
                            if let Some(project) = state.current_project_mut() {
                                project.delete_key(&key);
                                state.selected_key_index = state.selected_key_index.saturating_sub(1);
                                state.show_message(format!("Deleted {}", key), MessageLevel::Success);
                            }
                        }
                        DialogAction::SaveChanges => {
                            return Some(Action::SaveAll);
                        }
                        DialogAction::DiscardChanges | DialogAction::Quit => {
                            state.should_quit = true;
                        }
                        _ => {}
                    },
                    Dialog::AddKey { key, value, .. } => {
                        if !key.is_empty() {
                            if let Some(project) = state.current_project_mut() {
                                project.add_key(key.clone(), value);
                                state.show_message(format!("Added {}", key), MessageLevel::Success);
                            }
                        }
                    }
                    Dialog::BulkSwitch { selected_env_index } => {
                        if let Some(project) = state.current_project_mut() {
                            let env_types: Vec<_> = project.named_env_types().into_iter().cloned().collect();
                            if let Some(env_type) = env_types.get(selected_env_index) {
                                project.bulk_switch(env_type);
                                state.show_message(
                                    format!("Replaced .env with {}", env_type),
                                    MessageLevel::Success,
                                );
                            }
                        }
                    }
                    Dialog::SelectEnv { key, selected_index, options, .. } => {
                        if let Some(option) = options.get(selected_index) {
                            if let Some(ref env_type) = option.env_type {
                                // Set value from selected environment
                                if let Some(project) = state.current_project_mut() {
                                    project.set_value_from_env(&key, env_type);
                                    state.show_message(
                                        format!("Set {} to {} value", key, env_type),
                                        MessageLevel::Success,
                                    );
                                }
                            } else {
                                // Disable option selected - remove from .env
                                return Some(Action::DisableKey { key });
                            }
                        }
                    }
                    Dialog::SelectSectionEnv { section, selected_index, options, .. } => {
                        if let Some(option) = options.get(selected_index) {
                            if let Some(ref env_type) = option.env_type {
                                return Some(Action::SwitchSection {
                                    section,
                                    env: env_type.clone(),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
            None
        }

        Action::CloseDialog => {
            state.dialog = None;
            None
        }

        Action::SaveAll => {
            let mut total_saved = 0;

            // If output filename is specified, save to that file in each project's directory
            if let Some(ref output_filename) = state.output_filename {
                for project in &state.projects {
                    if let Err(e) = save_project_to_output(project, output_filename) {
                        state.show_message(format!("Error saving to output: {}", e), MessageLevel::Error);
                        return None;
                    }
                    total_saved += 1;
                }
                state.show_message(
                    format!("Saved {} project(s) to {}", total_saved, output_filename),
                    MessageLevel::Success,
                );
                return None;
            }

            // Otherwise, save to original files
            for project in &mut state.projects {
                match save_project(project, true) {
                    Ok(count) => total_saved += count,
                    Err(e) => {
                        state.show_message(format!("Error saving: {}", e), MessageLevel::Error);
                        return None;
                    }
                }
            }
            if total_saved > 0 {
                state.show_message(format!("Saved {} file(s)", total_saved), MessageLevel::Success);
            } else {
                state.show_message("No changes to save".to_string(), MessageLevel::Info);
            }
            None
        }

        Action::SaveProject => {
            if let Some(project) = state.current_project_mut() {
                match save_project(project, true) {
                    Ok(count) => {
                        if count > 0 {
                            state.show_message(format!("Saved {} file(s)", count), MessageLevel::Success);
                        } else {
                            state.show_message("No changes to save".to_string(), MessageLevel::Info);
                        }
                    }
                    Err(e) => {
                        state.show_message(format!("Error saving: {}", e), MessageLevel::Error);
                    }
                }
            }
            None
        }

        Action::ToggleHelp => {
            state.show_help = !state.show_help;
            None
        }

        Action::Quit => {
            if state.has_unsaved_changes() {
                state.dialog = Some(Dialog::UnsavedChanges);
            } else {
                state.should_quit = true;
            }
            None
        }

        Action::ForceQuit => {
            state.should_quit = true;
            None
        }

        Action::SaveAndQuit => {
            state.dialog = None;

            // If output filename is specified, save to that file in each project's directory
            if let Some(ref output_filename) = state.output_filename {
                for project in &state.projects {
                    if let Err(e) = save_project_to_output(project, output_filename) {
                        state.show_message(format!("Error saving to output: {}", e), MessageLevel::Error);
                        return None;
                    }
                }
            } else {
                // Otherwise, save to original files and quit
                for project in &mut state.projects {
                    if let Err(e) = save_project(project, true) {
                        state.show_message(format!("Error saving: {}", e), MessageLevel::Error);
                        return None;
                    }
                }
            }
            state.should_quit = true;
            None
        }

        Action::DiscardAndQuit => {
            state.dialog = None;
            state.should_quit = true;
            None
        }

        Action::Tick => {
            state.clear_old_messages();
            None
        }

        _ => None,
    }
}

/// Cycle through environment values for the current key
/// Options cycle: env1 -> env2 -> ... -> envN -> disabled -> env1 -> ...
/// For inactive keys: disabled -> env1 -> ...
fn cycle_env_value(state: &mut AppState, forward: bool) {
    let Some(key) = state.current_key() else { return };

    // Check if key is currently inactive (disabled)
    let is_inactive = state.is_current_item_inactive();

    // Gather info from immutable borrow
    let action = {
        let Some(project) = state.current_project() else { return };

        // Get envs that have this key (for inactive keys, get all envs with the key)
        let env_types: Vec<_> = if is_inactive {
            project.envs_with_key(&key).into_iter().cloned().collect()
        } else {
            project.named_env_types().into_iter().cloned().collect()
        };

        if env_types.is_empty() {
            return;
        }

        if is_inactive {
            // Key is disabled - cycling enables it from an env
            // Forward: enable from first env, Backward: enable from last env
            let env_idx = if forward { 0 } else { env_types.len() - 1 };
            let env_type = env_types[env_idx].clone();
            CycleAction::Enable { key: key.clone(), from_env: env_type }
        } else {
            // Key is active - find current match and cycle
            let current_match = project.find_matching_env(&key);
            let current_idx = current_match
                .as_ref()
                .and_then(|m| env_types.iter().position(|e| e == m));

            // Extended cycle includes "disabled" as the last option
            // Options: 0..len-1 = envs, len = disabled
            let total_options = env_types.len() + 1; // +1 for disabled

            let current_pos = match current_idx {
                Some(idx) => idx,
                None => total_options, // custom value - treat as after disabled
            };

            let next_pos = if forward {
                (current_pos + 1) % total_options
            } else {
                if current_pos == 0 {
                    total_options - 1 // wrap to disabled
                } else {
                    current_pos - 1
                }
            };

            if next_pos < env_types.len() {
                // Set value from env
                let new_env = &env_types[next_pos];
                let value = project.get_value(&key, new_env).map(String::from);
                CycleAction::SetValue {
                    key: key.clone(),
                    value,
                    env_name: new_env.to_string(),
                }
            } else {
                // Disable (remove from .env)
                CycleAction::Disable { key: key.clone() }
            }
        }
    };

    // Apply the action with mutable borrow
    match action {
        CycleAction::SetValue { key, value, env_name } => {
            if let Some(value) = value {
                if let Some(project) = state.current_project_mut() {
                    project.set_default_value(key.clone(), value);
                    state.show_message(format!("Set {} to {} value", key, env_name), MessageLevel::Info);
                }
            }
        }
        CycleAction::Disable { key } => {
            if let Some(project) = state.current_project_mut() {
                project.delete_key_from_env(&key, &EnvironmentType::Default);
                state.show_message(format!("Disabled {}", key), MessageLevel::Info);
            }
        }
        CycleAction::Enable { key, from_env } => {
            if let Some(project) = state.current_project_mut() {
                project.set_value_from_env(&key, &from_env);
                project.rebuild_keys_cache();
                state.show_message(format!("Enabled {} from {}", key, from_env), MessageLevel::Success);
            }
        }
    }
}

/// Helper enum for cycle_env_value
enum CycleAction {
    SetValue { key: String, value: Option<String>, env_name: String },
    Disable { key: String },
    Enable { key: String, from_env: EnvironmentType },
}

/// Get the value for the selected slot in key editor
/// Slot layout: 0 = target (.env), 1..n = named envs
fn get_editor_slot_value(state: &AppState, key: &str) -> String {
    let Some(project) = state.current_project() else {
        return String::new();
    };

    let env_types = project.named_env_types();
    let slot_index = state.selected_key_index;

    if slot_index == 0 {
        // Target slot - get .env value
        project.get_value(key, &EnvironmentType::Default).unwrap_or("").to_string()
    } else if slot_index <= env_types.len() {
        // Named env slot (indices 1..n)
        project.get_value(key, env_types[slot_index - 1]).unwrap_or("").to_string()
    } else {
        String::new()
    }
}
