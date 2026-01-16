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
    pub message: Option<Message>,
    pub dialog: Option<Dialog>,
    pub should_quit: bool,
    pub show_help: bool,
    pub editing_key: Option<String>, // Key currently being edited in KeyEditor
}

/// Which section of the project detail view is selected
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Section {
    /// Current .env configuration
    CurrentConfig,
    /// Missing keys from a specific environment
    MissingFrom(EnvironmentType),
}

impl AppState {
    pub fn new(projects: Vec<Project>) -> Self {
        Self {
            projects,
            selected_project_index: 0,
            selected_key_index: 0,
            selected_section: Section::CurrentConfig,
            current_view: View::ProjectList,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            message: None,
            dialog: None,
            should_quit: false,
            show_help: false,
            editing_key: None,
        }
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
        match &self.selected_section {
            Section::CurrentConfig => {
                project.display_items().get(self.selected_key_index).cloned()
            }
            Section::MissingFrom(env_type) => {
                let keys = project.missing_keys_for_env(env_type);
                keys.get(self.selected_key_index)
                    .map(|k| crate::models::DisplayItem::Key((*k).clone()))
            }
        }
    }

    /// Get the currently selected key (returns None if a section header is selected)
    pub fn current_key(&self) -> Option<String> {
        match self.current_display_item()? {
            crate::models::DisplayItem::Key(key) => Some(key),
            crate::models::DisplayItem::Section(_) => None,
            crate::models::DisplayItem::DuplicatedKey(key, _, _) => Some(key),
        }
    }

    /// Get the currently selected section name (returns None if a key is selected)
    pub fn current_section_name(&self) -> Option<String> {
        match self.current_display_item()? {
            crate::models::DisplayItem::Section(name) => Some(name),
            crate::models::DisplayItem::Key(_) => None,
            crate::models::DisplayItem::DuplicatedKey(_, _, _) => None,
        }
    }

    /// Get the number of items in the current section (includes section headers for CurrentConfig)
    pub fn current_section_item_count(&self) -> usize {
        let Some(project) = self.current_project() else {
            return 0;
        };
        match &self.selected_section {
            Section::CurrentConfig => project.display_item_count(),
            Section::MissingFrom(env_type) => project.missing_keys_for_env(env_type).len(),
        }
    }

    /// Get all sections for the current project
    pub fn get_sections(&self) -> Vec<Section> {
        let mut sections = vec![Section::CurrentConfig];
        if let Some(project) = self.current_project() {
            for env_type in project.named_env_types() {
                if !project.missing_keys_for_env(env_type).is_empty() {
                    sections.push(Section::MissingFrom(env_type.clone()));
                }
            }
        }
        sections
    }

    /// Move to next section
    pub fn next_section(&mut self) {
        let sections = self.get_sections();
        if let Some(pos) = sections.iter().position(|s| *s == self.selected_section) {
            if pos + 1 < sections.len() {
                self.selected_section = sections[pos + 1].clone();
                self.selected_key_index = 0;
            }
        }
    }

    /// Move to previous section
    pub fn prev_section(&mut self) {
        let sections = self.get_sections();
        if let Some(pos) = sections.iter().position(|s| *s == self.selected_section) {
            if pos > 0 {
                self.selected_section = sections[pos - 1].clone();
                self.selected_key_index = 0;
            }
        }
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
        on_submit: DialogAction,
    },
    AddKey {
        key: String,
        value: String,
        focus_on_value: bool,
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
