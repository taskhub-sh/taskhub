# TaskHub

TaskHub is a terminal-based multi-source task management system built in Rust. It aggregates tasks from various sources (Jira, GitHub Issues, GitLab Issues, markdown files) into a unified terminal interface, combining the power of Taskwarrior with modern CLI paradigms.

## Features

- **Unified Terminal Experience**: Full terminal functionality with command execution alongside specialized `/commands` for task management.
- **Multi-Source Aggregation**: Seamless integration with popular development platforms like GitHub Issues (Jira, GitLab, and Markdown coming soon).
- **Developer-First Design**: Built for developers, prioritizing speed and efficiency.
- **Rust-Powered Performance**: Blazing fast execution with memory safety guarantees.
- **Comprehensive Testing**: Industry-leading test coverage and quality assurance.

## Installation

### Prerequisites

- Rust (2021 edition) and Cargo
- `pkg-config` and `libssl-dev` (for OpenSSL dependency)
- `mold` (for faster linking on Linux - optional but recommended)

### Build from Source

1. Clone the repository:

   ```bash
   git clone https://github.com/taskhub-sh/taskhub.git
   cd taskhub
   ```

2. Install dependencies (Ubuntu/Debian):

   ```bash
   sudo apt update
   sudo apt install pkg-config libssl-dev mold
   ```

3. Build the project:

   ```bash
   cargo build --release
   ```

4. The executable will be located at `target/release/taskhub`.

## Usage

To run the TaskHub TUI:

```bash
./target/release/taskhub
```

### Commands (Planned)

- `/tasks list --status open --source github`
- `/tasks create "Fix login bug" --priority high`
- `/tasks update GH-123 --status done`
- `/sync github --repo owner/repo`
- `/config set github.token`

## Project Structure

```
src/
├── main.rs              # Entry point
├── cli/                 # Command-line interface
│   ├── mod.rs
│   ├── parser.rs        # Command parsing logic
│   └── commands/        # Individual command implementations
├── tui/                 # Terminal UI components
│   ├── mod.rs
│   ├── app.rs          # Main application state
│   ├── components/     # Reusable UI components
│   └── views/          # Different screens/views
├── db/                  # Database layer
│   ├── mod.rs
│   ├── models.rs       # Data models
│   └── operations.rs   # CRUD operations
├── integrations/        # External service integrations
│   ├── mod.rs
│   ├── github/         # GitHub API client
│   ├── jira/           # Jira API client
│   └── gitlab/         # GitLab API client
├── sync/               # Synchronization engine
│   ├── mod.rs
│   ├── engine.rs       # Main sync logic
│   └── conflict.rs     # Conflict resolution
├── config/             # Configuration management
│   ├── mod.rs
│   └── settings.rs
└── lib.rs              # Library exports
```

## Contributing

Contributions are welcome! Please see the `CONTRIBUTING.md` (coming soon) for guidelines.

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.
