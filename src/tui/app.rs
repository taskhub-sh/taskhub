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
}

impl App {
    pub fn new(db_pool: SqlitePool) -> Self {
        Self {
            should_quit: false,
            db_pool,
            tasks: Vec::new(),
            mode: AppMode::Terminal,
            command_history: Vec::new(),
            current_input: String::new(),
            cursor_position: 0,
            pending_command: None,
        }
    }

    pub async fn load_tasks(&mut self) -> Result<(), sqlx::Error> {
        self.tasks = operations::list_tasks(&self.db_pool).await?;
        Ok(())
    }

    pub fn on_key(&mut self, key: char) {
        match self.mode {
            AppMode::TaskList => {
                if key == 'q' {
                    self.should_quit = true;
                } else if key == 't' {
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
                    self.mode = AppMode::TaskList;
                }
                KeyCode::Enter => {
                    if !self.current_input.trim().is_empty() {
                        let command = self.current_input.trim().to_string();
                        self.current_input.clear();
                        self.cursor_position = 0;

                        // Mark that we need to execute this command
                        self.pending_command = Some(command);
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
                KeyCode::Home => {
                    self.cursor_position = 0;
                }
                KeyCode::End => {
                    self.cursor_position = self.current_input.chars().count();
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
    }
}
