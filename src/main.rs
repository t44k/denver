use std::io;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use denver::app::{Action, AppState, InputMode, Section, View};
use denver::scanner::scan_directory;
use denver::ui;

#[derive(Parser, Debug)]
#[command(name = "denver")]
#[command(about = "A TUI application for managing .env files across multiple projects")]
#[command(version)]
struct Args {
    /// Directory to scan for projects (defaults to current directory)
    #[arg(short, long)]
    path: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Determine root directory
    let root = args.path.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // Scan for projects
    let projects = scan_directory(&root)?;

    // Initialize state
    let mut state = AppState::new(projects);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let result = run_app(&mut terminal, &mut state);

    // Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
) -> anyhow::Result<()> {
    loop {
        // Render
        terminal.draw(|frame| ui::render(frame, state))?;
        // Note: render may have updated scroll offsets for pagination

        // Handle events with timeout for tick
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Only handle Press events
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Handle help overlay - any key closes it
                if state.show_help {
                    state.show_help = false;
                    continue;
                }

                // Convert key event to action
                let action = key_to_action(key.code, key.modifiers, state);

                if let Some(action) = action {
                    // Process action chain
                    let mut next_action = Some(action);
                    while let Some(action) = next_action {
                        next_action = denver::app::update(state, action);
                    }
                }
            }
        } else {
            // Tick for periodic updates
            denver::app::update(state, Action::Tick);
        }

        // Check quit flag
        if state.should_quit {
            break;
        }
    }

    Ok(())
}

fn key_to_action(code: KeyCode, modifiers: KeyModifiers, state: &AppState) -> Option<Action> {
    // Handle editing mode
    if state.input_mode == InputMode::Editing {
        return match code {
            KeyCode::Enter => Some(Action::ConfirmEdit),
            KeyCode::Esc => Some(Action::CancelEdit),
            KeyCode::Backspace => Some(Action::InputBackspace),
            KeyCode::Delete => Some(Action::InputDelete),
            KeyCode::Left => Some(Action::InputLeft),
            KeyCode::Right => Some(Action::InputRight),
            KeyCode::Home => Some(Action::InputHome),
            KeyCode::End => Some(Action::InputEnd),
            KeyCode::Char(c) => Some(Action::InputChar(c)),
            _ => None,
        };
    }

    // Handle dialog input
    if let Some(ref dialog) = state.dialog {
        // Special handling for UnsavedChanges dialog - only responds to y/n/ESC
        if matches!(dialog, denver::app::Dialog::UnsavedChanges) {
            return match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => Some(Action::SaveAndQuit),
                KeyCode::Char('n') | KeyCode::Char('N') => Some(Action::DiscardAndQuit),
                KeyCode::Esc => Some(Action::CloseDialog),
                _ => None,
            };
        }
        // Standard dialog handling
        return match code {
            KeyCode::Enter => Some(Action::ConfirmDialog),
            KeyCode::Esc => Some(Action::CloseDialog),
            KeyCode::Tab => Some(Action::DialogTab),
            KeyCode::Up | KeyCode::Char('k') => Some(Action::DialogUp),
            KeyCode::Down | KeyCode::Char('j') => Some(Action::DialogDown),
            KeyCode::Backspace => Some(Action::InputBackspace),
            KeyCode::Char(c) => Some(Action::InputChar(c)),
            _ => None,
        };
    }

    // View-specific handling
    match state.current_view {
        View::ProjectList => match code {
            KeyCode::Up | KeyCode::Char('k') => Some(Action::NavigateUp),
            KeyCode::Down | KeyCode::Char('j') => Some(Action::NavigateDown),
            KeyCode::Enter => Some(Action::Select),
            KeyCode::Char('s') => Some(Action::SaveAll),
            KeyCode::Char('?') => Some(Action::ToggleHelp),
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => Some(Action::ForceQuit),
            _ => None,
        },

        View::ProjectDetail => {
            // Different behavior based on section
            match &state.selected_section {
                Section::CurrentConfig => match code {
                    // Shift+Arrow for reordering
                    KeyCode::Up if modifiers.contains(KeyModifiers::SHIFT) => Some(Action::MoveUp),
                    KeyCode::Down if modifiers.contains(KeyModifiers::SHIFT) => Some(Action::MoveDown),
                    KeyCode::Right if modifiers.contains(KeyModifiers::SHIFT) => Some(Action::MoveIntoSection),
                    KeyCode::Left if modifiers.contains(KeyModifiers::SHIFT) => Some(Action::MoveOutOfSection),
                    // Normal navigation
                    KeyCode::Up | KeyCode::Char('k') => Some(Action::NavigateUp),
                    KeyCode::Down | KeyCode::Char('j') => Some(Action::NavigateDown),
                    KeyCode::Left | KeyCode::Char('h') => Some(Action::CycleEnvBack),
                    KeyCode::Right | KeyCode::Char('l') => Some(Action::CycleEnv),
                    KeyCode::Tab => {
                        // If on a section header, open section env selector
                        // If on a key, open key env selector
                        if let Some(section) = state.current_section_name() {
                            Some(Action::OpenSectionEnvMenu { section })
                        } else {
                            Some(Action::OpenEnvMenu)
                        }
                    }
                    KeyCode::Enter => Some(Action::Select), // Open key editor
                    KeyCode::Char('a') => Some(Action::AddKey),
                    KeyCode::Char('d') => Some(Action::DeleteKey),
                    KeyCode::Char('S') => Some(Action::BulkSwitch),
                    KeyCode::Char('s') => Some(Action::SaveAll),
                    KeyCode::Esc => Some(Action::Back),
                    KeyCode::Char('?') => Some(Action::ToggleHelp),
                    KeyCode::Char('q') => Some(Action::Quit),
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => Some(Action::ForceQuit),
                    _ => None,
                },
                Section::MissingFrom(_) => match code {
                    KeyCode::Up | KeyCode::Char('k') => Some(Action::NavigateUp),
                    KeyCode::Down | KeyCode::Char('j') => Some(Action::NavigateDown),
                    KeyCode::Tab => Some(Action::NextSection),
                    KeyCode::BackTab => Some(Action::PrevSection),
                    KeyCode::Enter => Some(Action::AddKeyFromMissing), // Add to .env
                    KeyCode::Esc => Some(Action::Back),
                    KeyCode::Char('s') => Some(Action::SaveAll),
                    KeyCode::Char('?') => Some(Action::ToggleHelp),
                    KeyCode::Char('q') => Some(Action::Quit),
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => Some(Action::ForceQuit),
                    _ => None,
                },
            }
        }

        View::KeyEditor => match code {
            KeyCode::Up | KeyCode::Char('k') => Some(Action::NavigateUp),
            KeyCode::Down | KeyCode::Char('j') => Some(Action::NavigateDown),
            KeyCode::Enter => Some(Action::StartEdit),
            KeyCode::Esc => Some(Action::Back),
            KeyCode::Char('s') => Some(Action::SaveAll),
            KeyCode::Char('?') => Some(Action::ToggleHelp),
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => Some(Action::ForceQuit),
            _ => None,
        },
    }
}
