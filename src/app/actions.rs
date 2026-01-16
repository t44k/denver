use crate::models::EnvironmentType;

use super::{Dialog, View};

/// All possible actions in the application
#[derive(Debug, Clone)]
pub enum Action {
    // Navigation
    NavigateUp,
    NavigateDown,
    NavigateLeft,  // Previous env for current key
    NavigateRight, // Next env for current key
    NextSection,   // Tab to next section
    PrevSection,   // Shift+Tab to previous section
    Select,        // Enter - open editor
    Back,

    // Views
    SwitchView(View),
    ToggleHelp,

    // Project operations
    SelectProject(usize),

    // Key operations
    AddKey,
    AddKeyFromMissing, // Add missing key to .env
    DeleteKey,
    CycleEnv,      // Change to next env value
    CycleEnvBack,  // Change to previous env value

    // Reordering operations
    MoveUp,           // Move item up within same level (Shift+Up)
    MoveDown,         // Move item down within same level (Shift+Down)
    MoveIntoSection,  // Put key into section above (Shift+Right)
    MoveOutOfSection, // Remove key from section (Shift+Left)
    OpenEnvMenu,   // Open environment selection menu
    OpenSectionEnvMenu { section: String }, // Open section env selection menu
    SwitchSection { section: String, env: EnvironmentType }, // Switch all keys in section to env

    // Environment operations
    BulkSwitch,

    // Value editing in KeyEditor
    StartEdit,
    SetValue { key: String, env: EnvironmentType, value: String },
    SetCustomValue { key: String, value: String },
    ConfirmEdit,
    CancelEdit,
    InputChar(char),
    InputBackspace,
    InputDelete,
    InputLeft,
    InputRight,
    InputHome,
    InputEnd,

    // Dialogs
    OpenDialog(Dialog),
    CloseDialog,
    ConfirmDialog,
    DialogUp,
    DialogDown,
    DialogTab,

    // File operations
    SaveAll,
    SaveProject,

    // App
    Quit,
    ForceQuit,
    SaveAndQuit,
    DiscardAndQuit,

    // Tick
    Tick,
}
