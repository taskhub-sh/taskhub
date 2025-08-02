# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Build & Run
- `cargo check` - For a quick build check. Works most of the time
- `cargo build` - Build the project - use only if `cargo check` is not enough
- Never use `cargo run` - it requires a TTY which you do not have
- `cargo run --bin ansi-parser` - Standalone ANSI parser utility for testing escape sequences

### Quality Assurance
- `cargo test` - Run all tests
- `cargo clippy` - Run linter (required before commits). Fix errors and warnings.
- `cargo fmt` - Format code
- `cargo clean` - Clean build artifacts

## Project Architecture

TaskHub is a terminal-based (TUI) task management system built in Rust with a modular, layered architecture:

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
- Advanced ANSI parsing (`ansi_parser.rs`) with full VT100/VT102 terminal emulation support
- Built-in command system with `/` prefix for special commands
- Command autocompletion and filtering when typing `/` (`completion.rs`)
- Command history with persistent storage via SQLite (`history.rs`)
- Keyboard navigation with arrow keys for command selection
- Mouse support for text selection and coordinate mapping
- Copy/paste functionality with clipboard integration (`arboard` crate)
- Reverse search (Ctrl-R) through command history
- Output search capabilities within terminal history
- Progress bar and color support through ANSI escape sequence processing

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
- Unit tests for core functionality (with `#[cfg(test)]` modules in source files)
- Integration tests for database operations (`tests/database_tests.rs`, `tests/db_operations.rs`)
- Comprehensive UI interaction tests (`tests/app_tests.rs`, `tests/keyboard_input_tests.rs`)
- ANSI parsing and color handling tests (`tests/ansi_color_test.rs`, `tests/color_regression_test.rs`)
- Terminal emulation tests (`tests/cursor_movement_tests.rs`, `tests/scroll_mouse_test.rs`)
- Clipboard and copy/paste functionality tests (`tests/copy_paste_tests.rs`)
- Command completion and filtering tests (`tests/command_filtering_tests.rs`, `tests/tab_completion_tests.rs`)
- Test database containers for isolated testing using SQLite in-memory databases

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
- SQLite for fast local operations with async `sqlx` driver
- Command history with configurable limits (default 1000 entries)
- Persistent history stored in SQLite database
- Parallel sync operations for multiple sources
- Clipboard integration via `arboard` with Wayland support
- ANSI parsing with `vtparse` for efficient terminal emulation

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

### Running Tests

Run tests with proper database setup:
```bash
cargo test
```

Tests use SQLite in-memory databases for isolation. Integration tests are in the `tests/` directory.

### Test-Driven Development Guidelines

**For New Features:**
1. **Start with failing tests**: Before implementing any new feature, write comprehensive tests that exercise the expected behavior. Run the tests to confirm they fail.
2. **Implement incrementally**: Write the minimal code needed to make the first test pass, then add more tests and continue implementing.
3. **Verify test coverage**: Ensure your tests cover edge cases, error conditions, and integration points.

**For Bug Fixes:**
1. **Reproduce the bug**: Before fixing any bug, write a test that reproduces the issue. This test should fail with the current implementation.
2. **Confirm the bug**: Run the failing test to verify it demonstrates the problem accurately.
3. **Fix and verify**: Implement the bug fix and confirm the test now passes, and that no existing tests are broken.

### Test Categories and Examples

**Unit Tests** (in source files with `#[cfg(test)]`):
- Test individual functions and methods in isolation
- Mock external dependencies where appropriate
- Example: Testing ANSI parsing logic in `src/tui/ansi_parser.rs`

**Integration Tests** (in `tests/` directory):
- Test interactions between components
- Database operations with real SQLite connections
- UI interactions and event handling
- Terminal emulation and rendering
- Examples: `tests/app_tests.rs`, `tests/database_tests.rs`

**Specific Test Areas:**
- **ANSI Processing**: `tests/ansi_color_test.rs`, `tests/color_regression_test.rs`
- **UI Interactions**: `tests/keyboard_input_tests.rs`, `tests/mouse_selection_test.rs`
- **Command System**: `tests/command_filtering_tests.rs`, `tests/tab_completion_tests.rs`
- **Copy/Paste**: `tests/copy_paste_tests.rs`, `tests/clipboard_integration_test.rs`
- **Terminal Features**: `tests/cursor_movement_tests.rs`, `tests/scroll_mouse_test.rs`

### Writing Effective Tests

**Test Structure:**
```rust
#[tokio::test]
async fn test_feature_name() {
    // Arrange: Set up test data and dependencies
    let db_pool = create_test_db().await;

    // Act: Execute the functionality being tested
    let result = function_under_test(&db_pool, input).await;

    // Assert: Verify the expected behavior
    assert_eq!(result.unwrap(), expected_value);
}
```

**Test Database Setup:**
```rust
async fn create_test_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    run_migration(&pool).await.unwrap();
    pool
}
```

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
- Follow the existing code organization patterns and module structure
- Use comprehensive error handling with `Result` types and `thiserror` for structured errors
- Write async code using `tokio` runtime for I/O operations
- Maintain separation of concerns between UI, business logic, and data layers

## Design Principles

- Prefer to use the applications database for persistent storage when possible instead of additional files
