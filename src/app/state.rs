use std::collections::HashMap;
use std::time::Instant;

use crate::models::{EnvironmentType, Project};

/// Main application state
#[derive(Debug)]
pub struct AppState {
    pub projects: Vec<Project>,
    pub selected_project_index: usize,
    pub selected_key_index: usize,
    pub selected_section: Section,
    pub current_view: View,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub cursor_pos: usize, // Cursor position within input_buffer
    pub message: Option<Message>,
    pub dialog: Option<Dialog>,
    pub should_quit: bool,
    pub show_help: bool,
    pub editing_key: Option<String>, // Key currently being edited in KeyEditor
    // Scroll offsets for pagination
    pub project_list_scroll: usize,
    pub config_scroll: usize,
    pub missing_section_scrolls: HashMap<EnvironmentType, usize>,
    pub dialog_scroll: usize,
    /// Output filename for saving (specified via -o flag). Saved to each project's directory.
    pub output_filename: Option<String>,
}

/// Which section of the project detail view is selected
/// (Simplified - now only CurrentConfig since we have unified list)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Section {
    /// Current unified configuration view
    CurrentConfig,
}

impl AppState {
    pub fn new(projects: Vec<Project>, output_filename: Option<String>) -> Self {
        Self {
            projects,
            selected_project_index: 0,
            selected_key_index: 0,
            selected_section: Section::CurrentConfig,
            current_view: View::ProjectList,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            cursor_pos: 0,
            message: None,
            dialog: None,
            should_quit: false,
            show_help: false,
            editing_key: None,
            project_list_scroll: 0,
            config_scroll: 0,
            missing_section_scrolls: HashMap::new(),
            dialog_scroll: 0,
            output_filename,
        }
    }

    /// Ensure the selected item is visible in the scrollable area
    pub fn ensure_visible(&mut self, selected: usize, scroll: &mut usize, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        // If selection is above the visible area, scroll up
        if selected < *scroll {
            *scroll = selected;
        }
        // If selection is below the visible area, scroll down
        else if selected >= *scroll + visible_height {
            *scroll = selected - visible_height + 1;
        }
    }

    /// Get the scroll offset for a missing section
    pub fn get_missing_scroll(&self, env_type: &EnvironmentType) -> usize {
        self.missing_section_scrolls.get(env_type).copied().unwrap_or(0)
    }

    /// Set the scroll offset for a missing section
    pub fn set_missing_scroll(&mut self, env_type: &EnvironmentType, scroll: usize) {
        self.missing_section_scrolls.insert(env_type.clone(), scroll);
    }

    pub fn current_project(&self) -> Option<&Project> {
        self.projects.get(self.selected_project_index)
    }

    pub fn current_project_mut(&mut self) -> Option<&mut Project> {
        self.projects.get_mut(self.selected_project_index)
    }

    /// Get the currently selected display item
    pub fn current_display_item(&self) -> Option<crate::models::DisplayItem> {
        let project = self.current_project()?;
        project.unified_display_items().get(self.selected_key_index).cloned()
    }

    /// Get the currently selected key (returns None if a section header is selected)
    pub fn current_key(&self) -> Option<String> {
        match self.current_display_item()? {
            crate::models::DisplayItem::Key(key) => Some(key),
            crate::models::DisplayItem::Section(_) => None,
            crate::models::DisplayItem::DuplicatedKey(key, _, _) => Some(key),
            crate::models::DisplayItem::InactiveKey(key) => Some(key),
        }
    }

    /// Get the currently selected section name (returns None if a key is selected)
    pub fn current_section_name(&self) -> Option<String> {
        match self.current_display_item()? {
            crate::models::DisplayItem::Section(name) => Some(name),
            crate::models::DisplayItem::Key(_) => None,
            crate::models::DisplayItem::DuplicatedKey(_, _, _) => None,
            crate::models::DisplayItem::InactiveKey(_) => None,
        }
    }

    /// Get the number of items in the unified list (includes section headers and inactive keys)
    pub fn current_section_item_count(&self) -> usize {
        let Some(project) = self.current_project() else {
            return 0;
        };
        project.unified_display_item_count()
    }

    /// Get all sections for the current project (simplified - only one section now)
    pub fn get_sections(&self) -> Vec<Section> {
        vec![Section::CurrentConfig]
    }

    /// Move to next section (no-op with unified list)
    pub fn next_section(&mut self) {
        // No-op: unified list has only one section
    }

    /// Move to previous section (no-op with unified list)
    pub fn prev_section(&mut self) {
        // No-op: unified list has only one section
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.projects.iter().any(|p| p.has_unsaved_changes())
    }

    pub fn show_message(&mut self, text: String, level: MessageLevel) {
        self.message = Some(Message {
            text,
            level,
            timestamp: Instant::now(),
        });
    }

    pub fn clear_old_messages(&mut self) {
        if let Some(ref msg) = self.message {
            if msg.timestamp.elapsed().as_secs() > 3 {
                self.message = None;
            }
        }
    }

    /// Reset selection when entering project detail
    pub fn reset_detail_selection(&mut self) {
        self.selected_section = Section::CurrentConfig;
        self.selected_key_index = 0;
        self.config_scroll = 0;
        self.missing_section_scrolls.clear();
    }

    /// Check if the currently selected item is an inactive key
    pub fn is_current_item_inactive(&self) -> bool {
        matches!(
            self.current_display_item(),
            Some(crate::models::DisplayItem::InactiveKey(_))
        )
    }

    /// Get the section name for the currently selected key (if any)
    pub fn current_key_section(&self) -> Option<String> {
        let project = self.current_project()?;
        let key = self.current_key()?;

        if let Some(env) = project.environments.get(&crate::models::EnvironmentType::Default) {
            if let Some(var) = env.variables.get(&key) {
                return var.section.clone();
            }
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    ProjectList,
    ProjectDetail,
    KeyEditor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub text: String,
    pub level: MessageLevel,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
    Success,
}

#[derive(Debug, Clone)]
pub enum Dialog {
    Confirm {
        title: String,
        message: String,
        on_confirm: DialogAction,
    },
    Input {
        title: String,
        prompt: String,
        value: String,
        cursor_pos: usize,
        on_submit: DialogAction,
    },
    AddKey {
        key: String,
        value: String,
        focus_on_value: bool,
        key_cursor_pos: usize,
        value_cursor_pos: usize,
    },
    BulkSwitch {
        selected_env_index: usize,
    },
    SelectEnv {
        key: String,
        selected_index: usize,
        current_index: Option<usize>, // Index of currently active env (matching .env value)
        options: Vec<EnvOption>,
    },
    SelectSectionEnv {
        section: String,
        selected_index: usize,
        current_indices: Vec<usize>, // Indices of currently matching envs
        options: Vec<EnvOption>,     // Only envs that have this section
    },
    UnsavedChanges,
}

#[derive(Debug, Clone)]
pub struct EnvOption {
    pub env_type: Option<EnvironmentType>, // None means "custom"
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub enum DialogAction {
    DeleteKey { key: String },
    DeleteKeyFromEnv { key: String, env: EnvironmentType },
    SaveChanges,
    DiscardChanges,
    Quit,
}
