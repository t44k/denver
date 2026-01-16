# Denver

A TUI application for managing `.env` files across multiple projects.

Denver allows viewing, editing, and synchronizing environment variables between different configuration files (`.env`, `.env.dev`, `.env.staging`, `.env.live`, etc.).

## Features

- Scan directories to discover projects with environment files
- View all environment variables across multiple `.env` files side-by-side
- Edit values directly in the terminal
- Identify missing keys between different environments
- Atomic file writes with optional backup support
- Preserves comments in `.env` files

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
```

## Usage

```bash
# Scan current directory
denver

# Scan a specific directory
denver --path /path/to/projects
```

## Keyboard Shortcuts

### Project List View
| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up/down |
| `Enter` | Select project |
| `s` | Save all changes |
| `q` | Quit |

### Project Detail View
| Key | Action |
|-----|--------|
| `j` / `k` | Navigate variables |
| `h` / `l` | Cycle environment values |
| `Tab` | Switch sections (Current Config / Missing Keys) |
| `Enter` | Edit key |
| `a` | Add new key |
| `d` | Delete key |
| `S` | Bulk switch environment |
| `Esc` | Go back |

### Key Editor View
| Key | Action |
|-----|--------|
| `Enter` | Edit value |
| `Esc` | Go back |

## Architecture

Denver follows the TEA (The Elm Architecture) pattern:

- **State**: `AppState` struct holds all application state
- **Actions**: `Action` enum defines all possible events
- **Update**: `update(state, action)` processes actions and returns chained actions
- **View**: `render(frame, state)` renders UI based on state

## Project Structure

```
src/
├── main.rs              # Entry point, CLI setup, event loop
├── lib.rs               # Library exports
├── error.rs             # Error types and validation
├── app/                 # Application state machine
│   ├── state.rs         # AppState and enums
│   ├── actions.rs       # Action definitions
│   └── update.rs        # State transitions
├── models/              # Data structures
│   ├── project.rs       # Project model
│   ├── environment.rs   # Environment model
│   └── env_var.rs       # Environment variable model
├── scanner/             # Directory scanning
│   ├── mod.rs           # Scanner entry point
│   └── parser.rs        # .env file parser
├── io/                  # File operations
│   ├── reader.rs        # Read operations
│   └── writer.rs        # Atomic writes
└── ui/                  # Ratatui rendering
    ├── mod.rs           # Main render function
    ├── styles.rs        # Colors and styling
    ├── components/      # UI components
    └── views/           # View implementations
```

## License

MIT
