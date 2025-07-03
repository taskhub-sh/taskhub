use crate::db::models::Task;
use crate::db::operations;
use crate::tui::views::terminal::CommandEntry;
use chrono::Utc;
use sqlx::SqlitePool;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, PartialEq)]
pub enum AppMode {
    TaskList,
    Terminal,
}

pub struct App {
    pub should_quit: bool,
    pub db_pool: SqlitePool,
    pub tasks: Vec<Task>,
    pub mode: AppMode,
    pub command_history: Vec<CommandEntry>,
    pub current_input: String,
    pub cursor_position: usize,
    pub pending_command: Option<String>,
    pub scroll_offset: usize,
    pub show_command_list: bool,
    pub command_filter: String,
    pub selected_command_index: usize,
    pub available_commands: Vec<String>,
}

impl App {
    pub fn new(db_pool: SqlitePool) -> Self {
        let available_commands = vec![
            "/quit".to_string(),
            "/task".to_string(),
            "/help".to_string(),
        ];

        Self {
            should_quit: false,
            db_pool,
            tasks: Vec::new(),
            mode: AppMode::Terminal,
            command_history: Vec::new(),
            current_input: String::new(),
            cursor_position: 0,
            pending_command: None,
            scroll_offset: 0,
            show_command_list: false,
            command_filter: String::new(),
            selected_command_index: 0,
            available_commands,
        }
    }

    pub async fn load_tasks(&mut self) -> Result<(), sqlx::Error> {
        self.tasks = operations::list_tasks(&self.db_pool).await?;
        Ok(())
    }

    pub fn on_key(&mut self, key: char) {
        match self.mode {
            AppMode::TaskList => {
                if key == 'q' || key == 't' {
                    self.mode = AppMode::Terminal;
                }
            }
            AppMode::Terminal => {
                self.handle_terminal_input(key);
            }
        }
    }

    pub fn on_key_code(&mut self, key_code: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;

        if self.mode == AppMode::Terminal {
            match key_code {
                KeyCode::Esc => {
                    if self.show_command_list {
                        self.show_command_list = false;
                        self.command_filter.clear();
                        self.selected_command_index = 0;
                    }
                }
                KeyCode::Enter => {
                    if self.show_command_list {
                        // Select the currently highlighted command
                        let filtered_commands = self.get_filtered_commands();
                        if !filtered_commands.is_empty()
                            && self.selected_command_index < filtered_commands.len()
                        {
                            let selected_command =
                                filtered_commands[self.selected_command_index].clone();
                            self.current_input = selected_command;
                            self.cursor_position = self.current_input.chars().count();
                            self.show_command_list = false;
                            self.command_filter.clear();
                            self.selected_command_index = 0;
                        }
                    } else if !self.current_input.trim().is_empty() {
                        let command = self.current_input.trim().to_string();
                        self.current_input.clear();
                        self.cursor_position = 0;

                        // Handle built-in commands
                        if self.handle_builtin_command(&command) {
                            return;
                        }

                        // Mark that we need to execute this command
                        self.pending_command = Some(command);

                        // Reset scroll to bottom when new command is entered
                        self.scroll_offset = 0;
                    }
                }
                KeyCode::Backspace => {
                    if self.cursor_position > 0 {
                        let mut chars: Vec<char> = self.current_input.chars().collect();
                        let cursor_pos = self.cursor_position.min(chars.len());
                        if cursor_pos > 0 {
                            chars.remove(cursor_pos - 1);
                            self.current_input = chars.into_iter().collect();
                            self.cursor_position = cursor_pos - 1;

                            // Update command filtering
                            self.update_command_filtering();
                        }
                    }
                }
                KeyCode::Delete => {
                    let chars: Vec<char> = self.current_input.chars().collect();
                    let cursor_pos = self.cursor_position.min(chars.len());
                    if cursor_pos < chars.len() {
                        let mut chars = chars;
                        chars.remove(cursor_pos);
                        self.current_input = chars.into_iter().collect();

                        // Update command filtering
                        self.update_command_filtering();
                    }
                }
                KeyCode::Left => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                    }
                }
                KeyCode::Right => {
                    let char_count = self.current_input.chars().count();
                    if self.cursor_position < char_count {
                        self.cursor_position += 1;
                    }
                }
                KeyCode::Up => {
                    if self.show_command_list {
                        // Navigate up in command list
                        if self.selected_command_index > 0 {
                            self.selected_command_index -= 1;
                        }
                    } else if self.current_input.is_empty() {
                        // Scroll up through history (show older content)
                        let max_scroll = self.get_total_history_lines().saturating_sub(1);
                        if self.scroll_offset < max_scroll {
                            self.scroll_offset += 1;
                        }
                    }
                }
                KeyCode::Down => {
                    if self.show_command_list {
                        // Navigate down in command list
                        let filtered_commands = self.get_filtered_commands();
                        if self.selected_command_index < filtered_commands.len().saturating_sub(1) {
                            self.selected_command_index += 1;
                        }
                    } else if self.current_input.is_empty() && self.scroll_offset > 0 {
                        // Scroll down through history (show newer content)
                        self.scroll_offset -= 1;
                    }
                }
                KeyCode::PageUp => {
                    // Scroll up by 10 lines
                    let max_scroll = self.get_total_history_lines().saturating_sub(1);
                    self.scroll_offset = (self.scroll_offset + 10).min(max_scroll);
                }
                KeyCode::PageDown => {
                    // Scroll down by 10 lines
                    self.scroll_offset = self.scroll_offset.saturating_sub(10);
                }
                KeyCode::Home => {
                    if self.current_input.is_empty() {
                        // Go to top of history
                        self.scroll_offset = self.get_total_history_lines().saturating_sub(1);
                    } else {
                        self.cursor_position = 0;
                    }
                }
                KeyCode::End => {
                    if self.current_input.is_empty() {
                        // Go to bottom of history
                        self.scroll_offset = 0;
                    } else {
                        self.cursor_position = self.current_input.chars().count();
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_terminal_input(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }

        let mut chars: Vec<char> = self.current_input.chars().collect();
        let cursor_pos = self.cursor_position.min(chars.len());
        chars.insert(cursor_pos, ch);
        self.current_input = chars.into_iter().collect();
        self.cursor_position = cursor_pos + 1;

        // Check if we're starting to type a command
        if self.current_input.starts_with('/') {
            self.command_filter = self.current_input[1..].to_string();
            self.show_command_list = true;
            self.selected_command_index = 0;
        } else {
            self.show_command_list = false;
            self.command_filter.clear();
        }
    }

    pub async fn handle_pending_commands(&mut self) {
        if let Some(command) = self.pending_command.take() {
            self.execute_command(command).await;
        }
    }

    pub async fn execute_command(&mut self, command: String) {
        let timestamp = Utc::now().format("%H:%M:%S").to_string();

        let result = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", &command])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
        } else {
            Command::new("sh")
                .args(["-c", &command])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
        };

        let (output, success) = match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined_output = if stderr.is_empty() {
                    stdout.to_string()
                } else if stdout.is_empty() {
                    stderr.to_string()
                } else {
                    format!("{stdout}\n{stderr}")
                };
                (combined_output, output.status.success())
            }
            Err(e) => (format!("Error executing command: {e}"), false),
        };

        let entry = CommandEntry {
            command,
            output,
            timestamp,
            success,
        };

        self.command_history.push(entry);

        // Keep only the last 1000 commands
        if self.command_history.len() > 1000 {
            self.command_history.drain(0..100);
        }

        // Reset scroll to bottom when new command is executed
        self.scroll_offset = 0;
    }

    /// Calculate total number of lines in command history
    pub fn get_total_history_lines(&self) -> usize {
        let mut total_lines = 0;
        for entry in &self.command_history {
            // Command line
            total_lines += 1;
            // Output lines
            if !entry.output.is_empty() {
                total_lines += entry.output.lines().count();
            }
            // Empty spacing line
            total_lines += 1;
        }
        total_lines
    }

    /// Update command filtering based on current input
    pub fn update_command_filtering(&mut self) {
        if self.current_input.starts_with('/') {
            self.command_filter = self.current_input[1..].to_string();
            self.show_command_list = true;
            self.selected_command_index = 0;
        } else {
            self.show_command_list = false;
            self.command_filter.clear();
        }
    }

    /// Get filtered commands based on current filter
    pub fn get_filtered_commands(&self) -> Vec<String> {
        if self.command_filter.is_empty() {
            self.available_commands.clone()
        } else {
            self.available_commands
                .iter()
                .filter(|cmd| cmd[1..].starts_with(&self.command_filter))
                .cloned()
                .collect()
        }
    }

    /// Handle built-in commands that don't need shell execution
    pub fn handle_builtin_command(&mut self, command: &str) -> bool {
        match command {
            "/quit" => {
                self.should_quit = true;
                true
            }
            "/task" => {
                self.mode = AppMode::TaskList;
                true
            }
            "/help" => {
                let help_text = "Available commands:\n/quit - Exit the application\n/task - Switch to task list view\n/help - Show this help message";
                let entry = CommandEntry {
                    command: command.to_string(),
                    output: help_text.to_string(),
                    timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
                    success: true,
                };
                self.command_history.push(entry);
                true
            }
            _ => false,
        }
    }
}
