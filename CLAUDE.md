# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Denver is a Rust TUI application for managing `.env` files across multiple projects. It allows viewing, editing, and synchronizing environment variables between different configuration files (`.env`, `.env.dev`, `.env.staging`, `.env.live`, etc.).

## Build Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo run                # Run the application
cargo run -- --path /some/dir  # Run scanning a specific directory
cargo test               # Run tests (parser unit tests in src/scanner/parser.rs)
cargo check              # Type check without building
cargo clippy             # Run linter
```

## Architecture

### TEA Pattern (The Elm Architecture)
The application follows a functional reactive pattern:
- **State**: `AppState` struct holds all application state
- **Actions**: `Action` enum defines all possible events
- **Update**: `update(state, action) -> Option<Action>` processes actions and returns chained actions
- **View**: `render(frame, state)` renders UI based on state

### Module Structure

```
src/
├── main.rs              # Entry point, CLI setup, event loop, key-to-action mapping
├── lib.rs               # Library exports
├── error.rs             # DenverError enum and validation logic
├── app/                 # Application state machine
│   ├── state.rs         # AppState, View, Section, InputMode enums
│   ├── actions.rs       # Action enum (40+ variants)
│   └── update.rs        # State transition logic
├── models/              # Data structures
│   ├── project.rs       # Project (collection of environments)
│   ├── environment.rs   # Environment with variables and dirty flag
│   └── env_var.rs       # Single environment variable
├── scanner/             # Directory scanning
│   ├── mod.rs           # scan_directory() entry point
│   └── parser.rs        # parse_env_file() with unit tests
├── io/                  # File operations
│   ├── reader.rs        # Read operations
│   └── writer.rs        # Atomic writes with backup support
└── ui/                  # Ratatui rendering
    ├── mod.rs           # Main render() function
    ├── styles.rs        # Colors and styling
    ├── components/      # Header, footer, help overlay, dialogs
    └── views/           # ProjectList, ProjectDetail, KeyEditor views
```

### Key Data Flow

1. **Startup**: CLI parses args → `scan_directory()` finds projects → `AppState::new(projects)`
2. **Event Loop**: Keyboard event → `key_to_action()` → `update(state, action)` chain → `render()`
3. **File Writes**: Atomic pattern (write to `.tmp` then rename), optional `.env.bak` backup

### State Structure

```rust
AppState {
    projects: Vec<Project>,           // All discovered projects
    selected_project_index: usize,
    selected_key_index: usize,
    selected_section: Section,        // CurrentConfig | MissingFrom(env_type)
    current_view: View,               // ProjectList | ProjectDetail | KeyEditor
    input_mode: InputMode,            // Normal | Editing
    dialog: Option<Dialog>,           // Modal dialogs
    // ... other fields
}
```

### Views and Navigation

- **ProjectList**: List of projects, `j/k` navigate, `Enter` select, `s` save all, `q` quit
- **ProjectDetail**: Two sections (CurrentConfig, MissingFrom), `Tab` switches sections, `h/l` cycles env values, `a` add key, `d` delete, `S` bulk switch
- **KeyEditor**: Edit single variable across environments, `Enter` to edit value, `Esc` to go back

## Testing

Test projects are available at `/workspace/test_projects/` with sample multi-environment setups:
- `web-frontend/` (3 env files)
- `api-server/` (3 env files)
- `worker-service/` (2 env files)
- `admin-panel/` (4 env files)

## Key Implementation Details

- **BTreeMap** used for sorted ordering of projects and variables
- **Dirty flag** on `Environment` tracks unsaved changes
- **all_keys_cache** on `Project` aggregates keys across all environments for fast access
- Parser preserves comments above variables and handles quoted values with special characters
