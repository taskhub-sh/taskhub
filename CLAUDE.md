# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Build & Run
- `cargo check` - For a quick build check. Works most of the time
- `cargo build` - Build the project - use only if `cargo check` is not enough
- Never use `cargo run` - it requires a TTY which you do not have

### Quality Assurance
- `cargo test` - Run all tests
- `cargo clippy` - Run linter (required before commits). Fix errors and warnings.
- `cargo fmt` - Format code
- `cargo clean` - Clean build artifacts

## Project Architecture

TaskHub is a terminal-based task management system built in Rust with a modular, layered architecture:

### Core Components

**Application Layer** (`src/main.rs`, `src/tui/app.rs`):
- Main application state management
- Event handling and UI rendering
- Two primary modes: TaskList view and Terminal view
- Uses `ratatui` for terminal UI with `crossterm` for cross-platform terminal handling

**Database Layer** (`src/db/`):
- SQLite database with `sqlx` for async operations
- Core `Task` model with support for multiple sources (GitHub, Jira, GitLab, Markdown)
- CRUD operations for task management
- Database migrations are implemented in `run_migration` in `src/db/mod.rs`. **Never** change the schema of existing tables, always use ALTER, DROP, etc modifiers for existing tables.

**Terminal User Interface** (`src/tui/`):
- Terminal-first interface with task list as secondary view
- Terminal mode is the main entry point with command execution and scrolling history
- Built-in command system with `/` prefix for special commands
- Command autocompletion and filtering when typing `/`
- Command history with 1000-entry limit
- Keyboard navigation with arrow keys for command selection

**Integration Layer** (`src/integrations/`):
- Modular design for external service connections
- Planned support for GitHub, Jira, GitLab, and Markdown files
- OAuth 2.0 and API token authentication support

**Synchronization Engine** (`src/sync/`):
- Bidirectional sync between local database and external sources
- Conflict resolution capabilities
- Incremental updates and offline support

**Configuration Management** (`src/config/`):
- Settings loaded from TOML files
- Secure credential storage via OS keychain
- XDG-compliant configuration directories

### Key Data Structures

**Task Model** (`src/db/models.rs`):
- Unified task structure supporting all external sources
- UUID-based local IDs with optional external IDs
- Status tracking (Open, InProgress, Done)
- Priority levels (High, Medium, Low)
- Custom fields for source-specific data

**App State** (`src/tui/app.rs`):
- Two modes: TaskList and Terminal (Terminal is default)
- Command history management with scrolling
- Built-in command filtering and selection system
- Asynchronous command execution
- Event-driven architecture with command autocomplete

### Development Patterns

**Error Handling**:
- Comprehensive `Result` types throughout
- `thiserror` for structured error types
- Graceful error handling in async operations

**Async Architecture**:
- `tokio` runtime for all I/O operations
- Database operations are fully async
- Command execution uses async process spawning

**Testing Strategy**:
- We are practicing *TDD* - Test Driven Development. It is recommended that before implementing a feature failing tests should be added and their failure should be observed, and only then implement the change and see the test pass.
- Unit tests for core functionality
- Integration tests for database operations
- Test database containers for isolated testing

### Security Considerations

**Credential Management**:
- API tokens stored in OS keychain via `keyring` crate
- No hardcoded credentials in source code
- Secure token refresh mechanisms

**Input Validation**:
- SQL injection prevention via parameterized queries
- Command injection protection through proper input sanitization
- Path traversal prevention

### Performance Characteristics

**Resource Usage**:
- Memory efficient with <50MB baseline usage
- SQLite for fast local operations
- Command history limited to 1000 entries
- Parallel sync operations for multiple sources

**Response Times**:
- Local operations target <50ms
- Database queries optimized for quick retrieval
- Async operations prevent UI blocking

## Integration Points

The application is designed to integrate with multiple external services:

- **GitHub Issues**: Via REST API with personal access tokens
- **Jira**: OAuth 2.0 authentication with full issue lifecycle
- **GitLab**: Personal/project token authentication
- **Markdown Files**: File watching for real-time updates

## Testing

Run tests with proper database setup:
```bash
cargo test
```

Tests use SQLite in-memory databases for isolation. Integration tests are in the `tests/` directory.

## Configuration

Application settings are managed through TOML configuration files in XDG-compliant directories. The config system supports:

- Database path configuration
- API credentials (stored securely in keychain)
- Sync intervals and preferences
- UI customization options

## Built-in Commands

The terminal interface includes several built-in commands:

- `/quit` - Exit the application
- `/task` - Switch to task list view
- `/help` - Show help information

### Command Usage

1. Start typing `/` to see available commands
2. Use ↑↓ arrows to navigate the command list
3. Press Enter to select a command
4. Press Esc to cancel command selection
5. In TaskList mode, press 'q' to return to Terminal mode

## Development Best Practices

- Start each new task in a separate git feature branch named feature/&lt;description&gt;

## Design Principles

- Prefer to use the applications database for persistent storage when possible instead of additional files

## MCP Usage

### terminal-driver-mcp

The terminal-driver-mcp is used to run applications with TTY and debug them by controlling them like a puppeteer. Here is how to use it
in the context of taskhub.

* Start taskhub by using the tool `terminal_launch with command="cargo run". **DO NOT RUN cargo run with termianl_input** its not designed for that.
* Enter terminal commands by using:
  * `terminal_input` with input_text to enter the command itself - for example `ls -la`
  * `terminal_input` with key to enter special keyboard keys - for example `Return`
* Then capture a screen shot with `terminal_capture`
* Close the session when done with `terminal_close`

You can repeat this process for any program using a TTY which you cannot debug directly.
