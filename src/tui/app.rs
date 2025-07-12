use crate::db::models::{Priority, Task, TaskSource, TaskStatus};
use crate::db::operations;
use crate::history::HistoryManager;
use crate::tui::completion::{CompletionEngine, CompletionState};
use crate::tui::views::terminal::CommandEntry;
use regex::Regex;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::{Child, Command};
use uuid::Uuid;

#[derive(Debug, PartialEq)]
pub enum AppMode {
    TaskList,
    Terminal,
}

#[derive(Debug, PartialEq, Clone)]
pub enum SearchMode {
    CaseInsensitive,
    CaseSensitive,
    Regex,
}

pub struct App {
    pub should_quit: bool,
    pub db_pool: SqlitePool,
    pub tasks: Vec<Task>,
    pub mode: AppMode,
    pub command_history: Vec<CommandEntry>,
    pub persistent_command_history: Vec<String>,
    pub current_input: String,
    pub cursor_position: usize,
    pub pending_command: Option<String>,
    pub scroll_offset: usize,
    pub show_command_list: bool,
    pub command_filter: String,
    pub selected_command_index: usize,
    pub available_commands: Vec<String>,
    pub user_navigated_command_list: bool,
    pub pending_task_add: Option<Task>,
    pub completion_engine: CompletionEngine,
    pub completion_state: CompletionState,
    pub running_command: Option<RunningCommand>,
    pub spinner_frame: usize,
    pub history_index: Option<usize>,
    pub saved_input: String,
    pub history_manager: Option<HistoryManager>,
    pub selection_start: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
    pub is_selecting: bool,
    pub input_selection_start: Option<usize>,
    pub input_selection_end: Option<usize>,
    pub is_selecting_input: bool,
    pub terminal_area_height: u16,
    pub clipboard: Option<arboard::Clipboard>,
    pub history_area_start: u16,
    pub history_area_height: u16,
    pub input_area_start: u16,
    pub auto_suggestion: Option<String>,
    pub reverse_search_active: bool,
    pub reverse_search_query: String,
    pub reverse_search_results: Vec<String>,
    pub reverse_search_index: usize,
    pub output_search_active: bool,
    pub output_search_query: String,
    pub output_search_matches: Vec<(usize, usize, usize)>, // (line_index, start_col, end_col)
    pub output_search_current_match: usize,
    pub output_search_mode: SearchMode,
}

pub struct RunningCommand {
    pub command: String,
    pub child: Child,
}

impl App {
    pub fn new(db_pool: SqlitePool) -> Self {
        let available_commands = vec![
            "/quit".to_string(),
            "/task".to_string(),
            "/task add".to_string(),
            "/task list".to_string(),
            "/help".to_string(),
            "/help keys".to_string(),
            "/clear".to_string(),
        ];

        let completion_engine = CompletionEngine::new(available_commands.clone());

        Self {
            should_quit: false,
            db_pool,
            tasks: Vec::new(),
            mode: AppMode::Terminal,
            command_history: Vec::new(),
            persistent_command_history: Vec::new(),
            current_input: String::new(),
            cursor_position: 0,
            pending_command: None,
            scroll_offset: 0,
            show_command_list: false,
            command_filter: String::new(),
            selected_command_index: 0,
            available_commands,
            user_navigated_command_list: false,
            pending_task_add: None,
            completion_engine,
            completion_state: CompletionState::new(),
            running_command: None,
            spinner_frame: 0,
            history_index: None,
            saved_input: String::new(),
            history_manager: None,
            selection_start: None,
            selection_end: None,
            is_selecting: false,
            input_selection_start: None,
            input_selection_end: None,
            is_selecting_input: false,
            terminal_area_height: 24,
            clipboard: None,
            history_area_start: 0,
            history_area_height: 21,
            input_area_start: 21,
            auto_suggestion: None,
            reverse_search_active: false,
            reverse_search_query: String::new(),
            reverse_search_results: Vec::new(),
            reverse_search_index: 0,
            output_search_active: false,
            output_search_query: String::new(),
            output_search_matches: Vec::new(),
            output_search_current_match: 0,
            output_search_mode: SearchMode::CaseInsensitive,
        }
    }

    pub fn with_history_manager(mut self, max_entries: Option<usize>) -> Self {
        self.history_manager = Some(HistoryManager::new(self.db_pool.clone(), max_entries));
        self
    }

    pub async fn load_persistent_history(&mut self) {
        if let Some(ref history_manager) = self.history_manager {
            self.persistent_command_history = history_manager.load_history().await;
        }
    }

    pub async fn save_persistent_history(&self) {
        if let Some(ref history_manager) = self.history_manager {
            if let Err(e) = history_manager
                .save_history(&self.persistent_command_history)
                .await
            {
                eprintln!("Warning: Failed to save command history: {e}");
            }
        }
    }

    pub async fn append_to_persistent_history(&mut self, command: &str) {
        if let Some(ref history_manager) = self.history_manager {
            if let Err(e) = history_manager.append_command(command).await {
                eprintln!("Warning: Failed to append to command history: {e}");
            } else {
                // Also add to local persistent history
                self.persistent_command_history.push(command.to_string());
                if self.persistent_command_history.len() > 1000 {
                    self.persistent_command_history.drain(0..100);
                }
            }
        }
    }

    /// Add an entry to command history and persist it if enabled
    pub async fn add_command_entry(&mut self, entry: CommandEntry) {
        // Add to current session history with full entry
        self.command_history.push(entry.clone());

        // Keep only the last 1000 commands in memory
        if self.command_history.len() > 1000 {
            self.command_history.drain(0..100);
        }

        // Persist only the command if history manager is enabled
        self.append_to_persistent_history(&entry.command).await;

        // Reset scroll to show newest entry
        self.scroll_offset = 0;
    }

    pub async fn load_tasks(&mut self) -> Result<(), sqlx::Error> {
        self.tasks = operations::list_tasks(&self.db_pool).await?;
        Ok(())
    }

    /// Get combined history for navigation (persistent + current session)
    fn get_combined_command_history(&self) -> Vec<String> {
        let mut combined = self.persistent_command_history.clone();
        for entry in &self.command_history {
            combined.push(entry.command.clone());
        }
        combined
    }

    pub fn get_prompt(&self) -> &'static str {
        if self.running_command.is_some() {
            // Spinner characters: ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏
            const SPINNER_CHARS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            SPINNER_CHARS[self.spinner_frame % SPINNER_CHARS.len()]
        } else {
            ">"
        }
    }

    pub fn update_spinner(&mut self) {
        if self.running_command.is_some() {
            self.spinner_frame = (self.spinner_frame + 1) % 10;
        }
    }

    pub fn on_key(&mut self, key: char) {
        match self.mode {
            AppMode::TaskList => {
                if key == 'q' || key == 't' {
                    self.mode = AppMode::Terminal;
                } else {
                    self.handle_terminal_input(key);
                }
            }
            AppMode::Terminal => {
                self.handle_terminal_input(key);
            }
        }
    }

    pub fn on_key_code(
        &mut self,
        key_code: crossterm::event::KeyCode,
        modifiers: crossterm::event::KeyModifiers,
    ) {
        use crossterm::event::{KeyCode, KeyModifiers};

        // Handle Ctrl-C for killing running commands
        if key_code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
            // This will be handled by checking for KeyEventKind::Press and KeyModifiers::CONTROL in main.rs
            return;
        }

        // Handle Ctrl-R for reverse search
        if key_code == KeyCode::Char('r') && modifiers.contains(KeyModifiers::CONTROL) {
            self.start_reverse_search();
            return;
        }

        // Handle Ctrl-L for clear screen
        if key_code == KeyCode::Char('l') && modifiers.contains(KeyModifiers::CONTROL) {
            self.clear_screen();
            return;
        }

        // Handle advanced cursor movement shortcuts
        if modifiers.contains(KeyModifiers::CONTROL) {
            match key_code {
                KeyCode::Char('a') => {
                    // Ctrl+A: Move cursor to beginning of line
                    self.cursor_position = 0;
                    self.update_auto_suggestion();
                    return;
                }
                KeyCode::Char('e') => {
                    // Ctrl+E: Move cursor to end of line
                    self.cursor_position = self.current_input.chars().count();
                    self.update_auto_suggestion();
                    return;
                }
                KeyCode::Char('f') => {
                    // Ctrl+F: Start output search
                    self.start_output_search();
                    return;
                }
                KeyCode::Char('b') => {
                    // Ctrl+B: Move cursor backward one character
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                        self.update_auto_suggestion();
                    }
                    return;
                }
                KeyCode::Char('k') => {
                    // Ctrl+K: Kill (delete) from cursor to end of line
                    let chars: Vec<char> = self.current_input.chars().collect();
                    let cursor_pos = self.cursor_position.min(chars.len());
                    self.current_input = chars[..cursor_pos].iter().collect();
                    // Cursor position stays the same (at end of remaining text)
                    self.completion_state.reset();
                    self.reset_history_navigation();
                    self.update_command_filtering();
                    self.update_auto_suggestion();
                    return;
                }
                KeyCode::Left => {
                    // Ctrl+Left: Move cursor backward by word
                    self.move_cursor_word_backward();
                    return;
                }
                KeyCode::Right => {
                    // Ctrl+Right: Move cursor forward by word
                    self.move_cursor_word_forward();
                    return;
                }
                _ => {}
            }
        }

        // Handle key codes for both modes
        match self.mode {
            AppMode::Terminal | AppMode::TaskList => {
                match key_code {
                    KeyCode::Esc => {
                        if self.output_search_active {
                            self.cancel_output_search();
                        } else if self.reverse_search_active {
                            self.cancel_reverse_search();
                        } else if self.show_command_list {
                            self.show_command_list = false;
                            self.command_filter.clear();
                            self.selected_command_index = 0;
                            self.user_navigated_command_list = false;
                        }
                    }
                    KeyCode::Enter => {
                        if self.output_search_active {
                            self.cancel_output_search();
                        } else if self.reverse_search_active {
                            self.accept_reverse_search();
                        } else if !self.current_input.trim().is_empty() {
                            let command = self.current_input.trim().to_string();

                            // If showing command list, decide between executing typed command or selecting from list
                            if self.show_command_list {
                                if self.user_navigated_command_list {
                                    // User actively navigated - select from list
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
                                        self.user_navigated_command_list = false;
                                    }
                                } else {
                                    // User never navigated - execute typed command if it's complete
                                    let is_complete_command =
                                        self.available_commands.contains(&command)
                                            || command.starts_with("/task add ")
                                            || command.starts_with("/help")
                                            || command.starts_with("/quit");

                                    if is_complete_command {
                                        // Execute the command directly
                                        if self.running_command.is_none() {
                                            self.current_input.clear();
                                            self.cursor_position = 0;
                                            self.show_command_list = false;
                                            self.command_filter.clear();
                                            self.selected_command_index = 0;
                                            self.user_navigated_command_list = false;
                                            self.pending_command = Some(command);
                                            self.scroll_offset = 0;
                                            self.reset_history_navigation();
                                        }
                                    } else {
                                        // Incomplete command - select from list
                                        let filtered_commands = self.get_filtered_commands();
                                        if !filtered_commands.is_empty()
                                            && self.selected_command_index < filtered_commands.len()
                                        {
                                            let selected_command = filtered_commands
                                                [self.selected_command_index]
                                                .clone();
                                            self.current_input = selected_command;
                                            self.cursor_position =
                                                self.current_input.chars().count();
                                            self.show_command_list = false;
                                            self.command_filter.clear();
                                            self.selected_command_index = 0;
                                            self.user_navigated_command_list = false;
                                        }
                                    }
                                }
                            } else {
                                // No command list showing, execute the command
                                // But only if no command is currently running
                                if self.running_command.is_none() {
                                    self.current_input.clear();
                                    self.cursor_position = 0;
                                    self.pending_command = Some(command);
                                    self.scroll_offset = 0;
                                    self.reset_history_navigation();
                                }
                            }
                        }
                    }
                    KeyCode::Tab => {
                        if self.output_search_active {
                            // Toggle search mode in output search mode
                            self.toggle_output_search_mode();
                        } else {
                            self.handle_tab_completion();
                        }
                    }
                    KeyCode::Backspace => {
                        if self.output_search_active {
                            if !self.output_search_query.is_empty() {
                                self.output_search_query.pop();
                                self.update_output_search();
                            }
                        } else if self.reverse_search_active {
                            if !self.reverse_search_query.is_empty() {
                                self.reverse_search_query.pop();
                                self.update_reverse_search();
                            }
                        } else {
                            // Reset completion state when user modifies input
                            self.completion_state.reset();
                            // Reset command history navigation when user modifies input
                            self.reset_history_navigation();
                            if self.cursor_position > 0 {
                                let mut chars: Vec<char> = self.current_input.chars().collect();
                                let cursor_pos = self.cursor_position.min(chars.len());
                                if cursor_pos > 0 {
                                    chars.remove(cursor_pos - 1);
                                    self.current_input = chars.into_iter().collect();
                                    self.cursor_position = cursor_pos - 1;

                                    // Update command filtering
                                    self.update_command_filtering();
                                    // Update auto-suggestion
                                    self.update_auto_suggestion();
                                }
                            }
                        }
                    }
                    KeyCode::Delete => {
                        // Reset completion state when user modifies input
                        self.completion_state.reset();
                        // Reset command history navigation when user modifies input
                        self.reset_history_navigation();
                        let chars: Vec<char> = self.current_input.chars().collect();
                        let cursor_pos = self.cursor_position.min(chars.len());
                        if cursor_pos < chars.len() {
                            let mut chars = chars;
                            chars.remove(cursor_pos);
                            self.current_input = chars.into_iter().collect();

                            // Update command filtering
                            self.update_command_filtering();
                            // Update auto-suggestion
                            self.update_auto_suggestion();
                        }
                    }
                    KeyCode::Left => {
                        if self.cursor_position > 0 {
                            self.cursor_position -= 1;
                            // Update auto-suggestion based on new cursor position
                            self.update_auto_suggestion();
                        }
                    }
                    KeyCode::Right => {
                        let char_count = self.current_input.chars().count();
                        if self.cursor_position < char_count {
                            self.cursor_position += 1;
                            // Update auto-suggestion based on new cursor position
                            self.update_auto_suggestion();
                        } else if let Some(_suggestion) = &self.auto_suggestion {
                            // Accept next character from auto-suggestion
                            self.accept_next_suggestion_char();
                        }
                    }
                    KeyCode::Up => {
                        if self.output_search_active {
                            self.output_search_previous_match();
                        } else if self.reverse_search_active {
                            self.reverse_search_previous();
                        } else if modifiers.contains(KeyModifiers::SHIFT) {
                            // Shift+Up: Scroll up through history (show older content)
                            let max_scroll = self.get_total_history_lines().saturating_sub(1);
                            if self.scroll_offset < max_scroll {
                                self.scroll_offset += 1;
                            }
                        } else if self.show_command_list {
                            // Navigate up in command list
                            if self.selected_command_index > 0 {
                                self.selected_command_index -= 1;
                                self.user_navigated_command_list = true;
                            }
                        } else {
                            // Command history navigation
                            let combined_history = self.get_combined_command_history();
                            if combined_history.is_empty() {
                                return; // No history available
                            }

                            // Save current input if we haven't started navigating history yet
                            if self.history_index.is_none() {
                                self.saved_input = self.current_input.clone();
                            }

                            // Navigate backward through history (older commands)
                            let new_index = match self.history_index {
                                None => combined_history.len() - 1,
                                Some(idx) => {
                                    if idx > 0 {
                                        idx - 1
                                    } else {
                                        return; // Already at oldest command
                                    }
                                }
                            };

                            self.history_index = Some(new_index);
                            self.current_input = combined_history[new_index].clone();
                            self.cursor_position = self.current_input.chars().count();
                        }
                    }
                    KeyCode::Down => {
                        if self.output_search_active {
                            self.output_search_next_match();
                        } else if self.reverse_search_active {
                            self.reverse_search_next();
                        } else if modifiers.contains(KeyModifiers::SHIFT) {
                            // Shift+Down: Scroll down through history (show newer content)
                            if self.scroll_offset > 0 {
                                self.scroll_offset -= 1;
                            }
                        } else if self.show_command_list {
                            // Navigate down in command list
                            let filtered_commands = self.get_filtered_commands();
                            if self.selected_command_index
                                < filtered_commands.len().saturating_sub(1)
                            {
                                self.selected_command_index += 1;
                                self.user_navigated_command_list = true;
                            }
                        } else {
                            // Command history navigation
                            if let Some(idx) = self.history_index {
                                let combined_history = self.get_combined_command_history();
                                // Navigate forward through history (newer commands)
                                if idx < combined_history.len() - 1 {
                                    let new_index = idx + 1;
                                    self.history_index = Some(new_index);
                                    self.current_input = combined_history[new_index].clone();
                                    self.cursor_position = self.current_input.chars().count();
                                } else {
                                    // Reached newest command, restore saved input
                                    self.history_index = None;
                                    self.current_input = self.saved_input.clone();
                                    self.cursor_position = self.current_input.chars().count();
                                }
                            }
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
    }

    pub fn handle_terminal_input(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }

        if self.output_search_active {
            // Handle output search input
            self.output_search_query.push(ch);
            self.update_output_search();
            return;
        }

        if self.reverse_search_active {
            // Handle search input
            self.reverse_search_query.push(ch);
            self.update_reverse_search();
            return;
        }

        // Reset completion state when user types new characters
        self.completion_state.reset();
        // Reset command history navigation when user starts typing
        self.reset_history_navigation();

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
            self.user_navigated_command_list = false;
        } else {
            self.show_command_list = false;
            self.command_filter.clear();
            self.user_navigated_command_list = false;
        }

        // Update auto-suggestion
        self.update_auto_suggestion();
    }

    pub fn handle_tab_completion(&mut self) {
        // If there's an auto-suggestion and cursor is at the end, accept it completely
        if let Some(suggestion) = &self.auto_suggestion {
            if self.cursor_position == self.current_input.chars().count() {
                self.current_input = suggestion.clone();
                self.cursor_position = self.current_input.chars().count();
                self.auto_suggestion = None;
                return;
            }
        }

        // If completion is already active, cycle to next completion
        if self.completion_state.is_active {
            if let Some(completed_text) = self.completion_state.cycle_next() {
                self.current_input = completed_text;
                self.cursor_position = self.current_input.chars().count();
            }
            return;
        }

        // Start new completion session
        let completions = self.completion_engine.get_completions(
            &self.current_input,
            self.cursor_position,
            &self.tasks,
        );

        if !completions.is_empty() {
            let word_start = self
                .completion_engine
                .find_word_start(&self.current_input, self.cursor_position);
            self.completion_state
                .start(&self.current_input, completions, word_start);

            // Apply first completion
            if let Some(completed_text) = self.completion_state.cycle_next() {
                self.current_input = completed_text;
                self.cursor_position = self.current_input.chars().count();
            }
        }
    }

    pub async fn handle_pending_commands(&mut self) {
        if let Some(command) = self.pending_command.take() {
            // Handle built-in commands first, then shell commands
            if self.handle_builtin_command(&command).await {
                // Handle pending task add after built-in command processing
                self.handle_pending_task_add().await;
                return;
            }
            self.execute_command(command).await;
        }

        // Also handle pending task add if no command was processed
        self.handle_pending_task_add().await;
    }

    pub async fn execute_command(&mut self, command: String) {
        // Don't start a new command if one is already running
        if self.running_command.is_some() {
            return;
        }

        let child = if cfg!(target_os = "windows") {
            match Command::new("cmd")
                .args(["/C", &command])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(e) => {
                    let entry = CommandEntry {
                        command,
                        output: format!("Error executing command: {e}"),
                        success: false,
                    };
                    self.add_command_entry(entry).await;
                    return;
                }
            }
        } else {
            match Command::new("sh")
                .args(["-c", &command])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(e) => {
                    let entry = CommandEntry {
                        command,
                        output: format!("Error executing command: {e}"),
                        success: false,
                    };
                    self.add_command_entry(entry).await;
                    return;
                }
            }
        };

        // Store the running command
        self.running_command = Some(RunningCommand {
            command: command.clone(),
            child,
        });

        // Add entry to show command started
        let entry = CommandEntry {
            command: command.clone(),
            output: "Running...".to_string(),
            success: true,
        };
        self.add_command_entry(entry).await;
    }

    pub async fn check_running_command(&mut self) {
        if let Some(mut running) = self.running_command.take() {
            match running.child.try_wait() {
                Ok(Some(status)) => {
                    // Command finished, collect output
                    let output = running.child.wait_with_output().await;
                    let (output_text, success) = match output {
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
                            (combined_output, status.success())
                        }
                        Err(e) => (format!("Error reading command output: {e}"), false),
                    };

                    // Update the last entry in history (the "Running..." entry)
                    if let Some(last_entry) = self.command_history.last_mut() {
                        if last_entry.command == running.command
                            && last_entry.output == "Running..."
                        {
                            last_entry.output = if output_text.trim().is_empty() {
                                "(no output)".to_string()
                            } else {
                                output_text
                            };
                            last_entry.success = success;
                        }
                    }
                }
                Ok(None) => {
                    // Command still running, put it back
                    self.running_command = Some(running);
                }
                Err(e) => {
                    // Error checking status
                    if let Some(last_entry) = self.command_history.last_mut() {
                        if last_entry.command == running.command
                            && last_entry.output == "Running..."
                        {
                            last_entry.output = format!("Error checking command status: {e}");
                            last_entry.success = false;
                        }
                    }
                }
            }
        }
    }

    pub async fn kill_running_command(&mut self) {
        if let Some(mut running) = self.running_command.take() {
            let _ = running.child.kill().await;

            // Update the last entry in history
            if let Some(last_entry) = self.command_history.last_mut() {
                if last_entry.command == running.command && last_entry.output == "Running..." {
                    last_entry.output = "Killed by user (Ctrl-C)".to_string();
                    last_entry.success = false;
                }
            }
        }
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
            self.user_navigated_command_list = false;
        } else {
            self.show_command_list = false;
            self.command_filter.clear();
            self.user_navigated_command_list = false;
        }
    }

    /// Get filtered commands based on current filter
    pub fn get_filtered_commands(&self) -> Vec<String> {
        if self.command_filter.is_empty() {
            // Show only top-level commands when no filter
            self.available_commands
                .iter()
                .filter(|cmd| !cmd[1..].contains(' '))
                .cloned()
                .collect()
        } else {
            // Show matching commands including subcommands
            let filter = &self.command_filter;
            self.available_commands
                .iter()
                .filter(|cmd| {
                    let cmd_without_slash = &cmd[1..];
                    cmd_without_slash.starts_with(filter)
                })
                .cloned()
                .collect()
        }
    }

    /// Handle built-in commands that don't need shell execution
    pub async fn handle_builtin_command(&mut self, command: &str) -> bool {
        match command {
            "/quit" => {
                self.should_quit = true;
                true
            }
            "/task" | "/task list" => {
                self.mode = AppMode::TaskList;
                true
            }
            "/help" => {
                let help_text = "Available commands:\n/quit - Exit the application\n/task - Switch to task list view\n/task add - Add a new task\n/task list - Show task list\n/clear - Clear terminal screen (Ctrl+L)\n/help - Show this help message\n/help keys - Show keyboard shortcuts";
                let entry = CommandEntry {
                    command: command.to_string(),
                    output: help_text.to_string(),
                    success: true,
                };
                self.add_command_entry(entry).await;
                true
            }
            "/help keys" => {
                let keys_help = "\n📋 TaskHub Keyboard Shortcuts\n\n🔄 Mode Switching:\n  q                 Switch to Terminal mode (from TaskList)\n  /task            Switch to TaskList mode\n\n📝 Text Editing:\n  Ctrl+A           Move cursor to beginning of line\n  Ctrl+E           Move cursor to end of line\n  Ctrl+B           Move cursor backward one character\n  Ctrl+K           Delete from cursor to end of line\n  Backspace        Delete character before cursor\n  Delete           Delete character at cursor\n\n🧭 Navigation:\n  ↑/↓ arrows       Navigate command history\n  ←/→ arrows       Move cursor left/right\n  Ctrl+←/→         Move cursor by word\n  Home/End         Move to beginning/end (or scroll history if empty)\n\n📜 Scrolling:\n  Shift+↑/↓        Scroll through terminal history\n  Page Up/Down     Scroll by 10 lines\n\n🔍 Search & Completion:\n  Ctrl+R           Reverse search through history\n  Ctrl+F           Search terminal output\n  Tab              Accept auto-suggestion or cycle completions\n  Right arrow      Accept next character from suggestion\n\n📋 Copy & Paste:\n  Ctrl+C           Copy selected text or interrupt command\n  Ctrl+V           Paste from clipboard\n  Middle Click     Paste from clipboard\n\n🖱️ Mouse:\n  Left Click       Start text selection\n  Left Drag        Extend text selection\n  Right Click      Clear selections\n\n⌨️ Command List (when typing /):\n  ↑/↓ arrows       Navigate command list\n  Enter            Select command\n  Esc              Cancel command selection\n\n🔍 Reverse Search (Ctrl+R):\n  ↑/↓ arrows       Navigate search results\n  Enter            Accept search result\n  Esc              Cancel reverse search\n\n🔍 Output Search (Ctrl+F):\n  Type text        Search terminal output\n  ↑/↓ arrows       Navigate between matches\n  Tab              Toggle case sensitivity ([Aa]/[aa])\n  Enter/Esc        Exit search mode\n\n🚪 Exit:\n  /quit            Exit application\n  Ctrl+C           Interrupt running command\n  Ctrl+L           Clear terminal screen";
                let entry = CommandEntry {
                    command: command.to_string(),
                    output: keys_help.to_string(),
                    success: true,
                };
                self.add_command_entry(entry).await;
                true
            }
            "/clear" => {
                self.clear_screen();
                true
            }
            _ if command.starts_with("/task add") => {
                self.handle_task_add_command(command).await;
                true
            }
            _ => false,
        }
    }

    /// Handle /task add command
    pub async fn handle_task_add_command(&mut self, command: &str) {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.len() < 3 {
            let entry = CommandEntry {
                command: command.to_string(),
                output: "Usage: /task add <title>".to_string(),
                success: false,
            };
            self.add_command_entry(entry).await;
            return;
        }

        let title = parts[2..].join(" ");
        let task = Task {
            id: Uuid::new_v4(),
            external_id: None,
            source: TaskSource::Markdown,
            title,
            description: None,
            status: TaskStatus::Open,
            priority: Priority::Medium,
            assignee: None,
            labels: Vec::new(),
            due_date: None,
            created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            custom_fields: HashMap::new(),
        };

        // Store the task to add asynchronously
        self.pending_task_add = Some(task);
        self.mode = AppMode::TaskList;
    }

    /// Handle adding a task asynchronously
    pub async fn handle_pending_task_add(&mut self) {
        if let Some(task) = self.pending_task_add.take() {
            match operations::create_task(&self.db_pool, &task).await {
                Ok(()) => {
                    let entry = CommandEntry {
                        command: format!("/task add {}", task.title),
                        output: format!("Task '{}' added successfully", task.title),
                        success: true,
                    };
                    self.add_command_entry(entry).await;
                    // Reload tasks to show the new task
                    if let Err(e) = self.load_tasks().await {
                        let error_entry = CommandEntry {
                            command: "reload_tasks".to_string(),
                            output: format!("Error reloading tasks: {e}"),
                            success: false,
                        };
                        self.add_command_entry(error_entry).await;
                    }
                }
                Err(e) => {
                    let entry = CommandEntry {
                        command: format!("/task add {}", task.title),
                        output: format!("Error adding task: {e}"),
                        success: false,
                    };
                    self.add_command_entry(entry).await;
                }
            }
        }
    }

    /// Reset command history navigation state
    fn reset_history_navigation(&mut self) {
        self.history_index = None;
        self.saved_input.clear();
    }

    /// Update auto-suggestion based on current input
    pub fn update_auto_suggestion(&mut self) {
        if self.current_input.is_empty() {
            self.auto_suggestion = None;
            return;
        }

        // Don't suggest while showing command list
        if self.show_command_list {
            self.auto_suggestion = None;
            return;
        }

        // Only show suggestions when cursor is at the end of input
        if self.cursor_position != self.current_input.chars().count() {
            self.auto_suggestion = None;
            return;
        }

        // Get combined history
        let combined_history = self.get_combined_command_history();

        // Find the most recent command that starts with current input
        // Search backwards through history (most recent first)
        for command in combined_history.iter().rev() {
            if command.starts_with(&self.current_input) && command != &self.current_input {
                self.auto_suggestion = Some(command.clone());
                return;
            }
        }

        self.auto_suggestion = None;
    }

    /// Accept the next character from auto-suggestion
    pub fn accept_next_suggestion_char(&mut self) {
        if let Some(suggestion) = &self.auto_suggestion.clone() {
            if self.cursor_position == self.current_input.chars().count() {
                let suggestion_chars: Vec<char> = suggestion.chars().collect();
                let input_chars: Vec<char> = self.current_input.chars().collect();

                // Find the next character to accept
                if input_chars.len() < suggestion_chars.len() {
                    let next_char = suggestion_chars[input_chars.len()];
                    self.current_input.push(next_char);
                    self.cursor_position += 1;

                    // Keep the same suggestion if there are more characters to accept
                    if self.current_input.len() < suggestion.len() {
                        // Keep the current suggestion
                        self.auto_suggestion = Some(suggestion.clone());
                    } else {
                        // We've accepted the entire suggestion, clear it
                        self.auto_suggestion = None;
                    }
                }
            }
        }
    }

    /// Start text selection at the given position
    pub fn start_selection(&mut self, line: usize, col: usize) {
        self.selection_start = Some((line, col));
        self.selection_end = Some((line, col));
        self.is_selecting = true;
    }

    /// Update text selection end position
    pub fn update_selection(&mut self, line: usize, col: usize) {
        if self.is_selecting {
            self.selection_end = Some((line, col));
        }
    }

    /// End text selection
    pub fn end_selection(&mut self) {
        self.is_selecting = false;
    }

    /// Clear text selection
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
        self.is_selecting = false;
    }

    /// Clear input selection
    pub fn clear_input_selection(&mut self) {
        self.input_selection_start = None;
        self.input_selection_end = None;
        self.is_selecting_input = false;
    }

    /// Start input text selection
    pub fn start_input_selection(&mut self, pos: usize) {
        self.input_selection_start = Some(pos);
        self.input_selection_end = Some(pos);
        self.is_selecting_input = true;
    }

    /// Update input text selection
    pub fn update_input_selection(&mut self, pos: usize) {
        if self.is_selecting_input {
            self.input_selection_end = Some(pos);
        }
    }

    /// End input text selection
    pub fn end_input_selection(&mut self) {
        self.is_selecting_input = false;
    }

    /// Get selected text from terminal history
    pub fn get_selected_text(&self) -> Option<String> {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            // Ensure start is before end
            let (start_line, start_col) =
                if start.0 <= end.0 || (start.0 == end.0 && start.1 <= end.1) {
                    start
                } else {
                    end
                };
            let (end_line, end_col) = if start.0 <= end.0 || (start.0 == end.0 && start.1 <= end.1)
            {
                end
            } else {
                start
            };

            // Convert command history to lines
            let mut lines = Vec::new();
            for entry in &self.command_history {
                lines.push(format!("> {}", entry.command));
                if !entry.output.is_empty() {
                    for line in entry.output.lines() {
                        lines.push(line.to_string());
                    }
                }
                lines.push(String::new()); // Empty line for spacing
            }

            // Extract selected text
            let mut selected_text = String::new();
            for line_idx in start_line..=end_line.min(lines.len().saturating_sub(1)) {
                if line_idx < lines.len() {
                    let line = &lines[line_idx];
                    if line_idx == start_line && line_idx == end_line {
                        // Single line selection
                        let start_pos = start_col.min(line.len());
                        let end_pos = end_col.min(line.len());
                        if start_pos < end_pos {
                            selected_text.push_str(&line[start_pos..end_pos]);
                        }
                    } else if line_idx == start_line {
                        // First line of multi-line selection
                        let start_pos = start_col.min(line.len());
                        if start_pos < line.len() {
                            selected_text.push_str(&line[start_pos..]);
                        }
                        selected_text.push('\n');
                    } else if line_idx == end_line {
                        // Last line of multi-line selection
                        let end_pos = end_col.min(line.len());
                        if end_pos > 0 {
                            selected_text.push_str(&line[..end_pos]);
                        }
                    } else {
                        // Middle lines
                        selected_text.push_str(line);
                        selected_text.push('\n');
                    }
                }
            }

            if !selected_text.is_empty() {
                Some(selected_text)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get selected input text
    pub fn get_selected_input_text(&self) -> Option<String> {
        if let (Some(start), Some(end)) = (self.input_selection_start, self.input_selection_end) {
            let chars: Vec<char> = self.current_input.chars().collect();
            let start_pos = start.min(end).min(chars.len());
            let end_pos = start.max(end).min(chars.len());

            if start_pos < end_pos {
                Some(chars[start_pos..end_pos].iter().collect())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Copy selected text to clipboard (handles both terminal and input text)
    pub fn copy_selected_text(&mut self) -> Result<(), String> {
        let text_to_copy = if let Some(input_text) = self.get_selected_input_text() {
            Some(input_text)
        } else {
            self.get_selected_text()
        };

        if let Some(text) = text_to_copy {
            // Initialize clipboard if not already done
            if self.clipboard.is_none() {
                match arboard::Clipboard::new() {
                    Ok(clipboard) => self.clipboard = Some(clipboard),
                    Err(_) => return Err("Failed to access clipboard".to_string()),
                }
            }

            // Use the persistent clipboard instance
            if let Some(ref mut clipboard) = self.clipboard {
                match clipboard.set_text(&text) {
                    Ok(_) => {
                        // Keep the clipboard instance alive and add a delay
                        // to ensure clipboard managers can register the content.
                        // This solves the "Clipboard was dropped very quickly" issue
                        std::thread::sleep(std::time::Duration::from_millis(50));

                        self.clear_selection();
                        self.clear_input_selection();
                        Ok(())
                    }
                    Err(_) => Err("Failed to set clipboard contents".to_string()),
                }
            } else {
                Err("Failed to access clipboard".to_string())
            }
        } else {
            Err("No text selected".to_string())
        }
    }

    /// Paste text from clipboard
    pub fn paste_from_clipboard(&mut self) -> Result<(), String> {
        // Initialize clipboard if not already done
        if self.clipboard.is_none() {
            match arboard::Clipboard::new() {
                Ok(clipboard) => self.clipboard = Some(clipboard),
                Err(_) => return Err("Failed to access clipboard".to_string()),
            }
        }

        // Use the persistent clipboard instance
        if let Some(ref mut clipboard) = self.clipboard {
            match clipboard.get_text() {
                Ok(text) => {
                    // Insert text at current cursor position
                    let mut chars: Vec<char> = self.current_input.chars().collect();
                    let cursor_pos = self.cursor_position.min(chars.len());

                    // Insert clipboard text character by character
                    let mut char_count = 0;
                    for ch in text.chars() {
                        // Skip newlines and other control characters for single-line input
                        if ch.is_control() && ch != '\t' {
                            continue;
                        }
                        chars.insert(cursor_pos + char_count, ch);
                        char_count += 1;
                    }

                    self.current_input = chars.into_iter().collect();
                    self.cursor_position = cursor_pos + char_count;

                    Ok(())
                }
                Err(_) => Err("Failed to get clipboard contents".to_string()),
            }
        } else {
            Err("Failed to access clipboard".to_string())
        }
    }

    /// Update terminal area height
    pub fn set_terminal_area_height(&mut self, height: u16) {
        self.terminal_area_height = height;
    }

    /// Update layout areas for proper mouse coordinate mapping
    pub fn update_layout_areas(
        &mut self,
        total_height: u16,
        show_command_list: bool,
        command_list_size: u16,
    ) {
        self.terminal_area_height = total_height;

        if show_command_list {
            // Three-area layout: history, command list, input
            self.history_area_start = 0;
            self.history_area_height = total_height.saturating_sub(command_list_size + 3);
            self.input_area_start = total_height.saturating_sub(3);
        } else {
            // Two-area layout: history, input
            self.history_area_start = 0;
            self.history_area_height = total_height.saturating_sub(3);
            self.input_area_start = total_height.saturating_sub(3);
        }
    }

    /// Handle mouse events with proper coordinate mapping
    pub fn on_mouse_event(&mut self, mouse: crossterm::event::MouseEvent) {
        use crossterm::event::{MouseButton, MouseEventKind};

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let mouse_row = mouse.row;

                // Determine which area was clicked
                if mouse_row >= self.input_area_start {
                    // Clicked in input area - start input selection
                    let input_pos = self.mouse_col_to_input_pos(mouse.column as usize);
                    self.clear_selection(); // Clear any terminal selection
                    self.start_input_selection(input_pos);
                } else if mouse_row >= self.history_area_start {
                    // Clicked in terminal history area
                    self.clear_input_selection(); // Clear any input selection

                    // Map mouse coordinates to visible content line
                    let content_row = self.map_mouse_to_content_line(mouse_row, mouse.column);
                    if let Some((line, col)) = content_row {
                        self.start_selection(line, col);
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                let mouse_row = mouse.row;

                if mouse_row >= self.input_area_start {
                    // Dragging in input area
                    if self.is_selecting_input {
                        let input_pos = self.mouse_col_to_input_pos(mouse.column as usize);
                        self.update_input_selection(input_pos);
                    }
                } else if mouse_row >= self.history_area_start {
                    // Dragging in terminal history area
                    if self.is_selecting {
                        let content_row = self.map_mouse_to_content_line(mouse_row, mouse.column);
                        if let Some((line, col)) = content_row {
                            self.update_selection(line, col);
                        }
                    }
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                // End selection and automatically copy
                if self.is_selecting || self.is_selecting_input {
                    self.end_selection();
                    self.end_input_selection();

                    // Automatically copy selected text
                    let _ = self.copy_selected_text();
                }
            }
            MouseEventKind::Down(MouseButton::Middle) => {
                // Middle mouse button paste
                let _ = self.paste_from_clipboard();
            }
            MouseEventKind::Down(MouseButton::Right) => {
                // Right click - clear all selections
                self.clear_selection();
                self.clear_input_selection();
            }
            _ => {}
        }
    }

    /// Convert mouse column to input position accounting for prompt and borders
    pub fn mouse_col_to_input_pos(&self, mouse_col: usize) -> usize {
        // Account for left border (1 char) and prompt
        let border_offset = 1;
        let prompt_len = self.get_prompt().len() + 1; // +1 for space after prompt
        let total_offset = border_offset + prompt_len;

        if mouse_col > total_offset {
            (mouse_col - total_offset).min(self.current_input.chars().count())
        } else {
            0
        }
    }

    /// Map mouse coordinates to actual content line accounting for scroll and visible area
    pub fn map_mouse_to_content_line(
        &self,
        mouse_row: u16,
        mouse_col: u16,
    ) -> Option<(usize, usize)> {
        // Calculate which line in the visible area was clicked
        let history_relative_row = mouse_row - self.history_area_start;

        // Account for top border - the first visible row is the border
        if history_relative_row == 0 {
            return None; // Clicked on border
        }

        let visible_line_index = (history_relative_row - 1) as usize; // -1 for top border

        // Now we need to map this visible line back to the actual content line
        // This requires reproducing the same logic as in draw_command_history

        // First, build the same all_items structure to understand the mapping
        let mut all_items_count = 0;
        for entry in &self.command_history {
            // Command line
            all_items_count += 1;

            // Output lines
            if !entry.output.is_empty() {
                all_items_count += entry.output.lines().count();
            }

            // Empty line for spacing
            all_items_count += 1;
        }

        // Calculate available height (same as in draw_command_history)
        let available_height = self.history_area_height.saturating_sub(2) as usize; // -2 for borders

        // Calculate which items are visible (same logic as in draw_command_history)
        let visible_start_index = if all_items_count <= available_height {
            0 // All items fit
        } else {
            // Need to scroll - show from bottom up with offset
            if self.scroll_offset >= all_items_count {
                0
            } else {
                all_items_count.saturating_sub(available_height + self.scroll_offset)
            }
        };

        // The actual content line is the visible line index plus the start offset
        let content_line = visible_start_index + visible_line_index;

        // Make sure we don't go beyond the actual content
        if content_line >= all_items_count {
            return None;
        }

        // Account for left border in column
        let content_col = if mouse_col > 0 {
            (mouse_col - 1) as usize
        } else {
            0
        };

        Some((content_line, content_col))
    }

    /// Start reverse search mode
    pub fn start_reverse_search(&mut self) {
        self.reverse_search_active = true;
        self.reverse_search_query.clear();
        self.reverse_search_results.clear();
        self.reverse_search_index = 0;
        self.update_reverse_search();
    }

    /// Cancel reverse search and restore original input
    pub fn cancel_reverse_search(&mut self) {
        self.reverse_search_active = false;
        self.reverse_search_query.clear();
        self.reverse_search_results.clear();
        self.reverse_search_index = 0;
    }

    /// Accept current reverse search result
    pub fn accept_reverse_search(&mut self) {
        if !self.reverse_search_results.is_empty()
            && self.reverse_search_index < self.reverse_search_results.len()
        {
            self.current_input = self.reverse_search_results[self.reverse_search_index].clone();
            self.cursor_position = self.current_input.chars().count();
        }
        self.reverse_search_active = false;
        self.reverse_search_query.clear();
        self.reverse_search_results.clear();
        self.reverse_search_index = 0;
    }

    /// Update reverse search results based on current query
    pub fn update_reverse_search(&mut self) {
        if self.reverse_search_query.is_empty() {
            self.reverse_search_results.clear();
            self.reverse_search_index = 0;
            return;
        }

        // Get combined history
        let combined_history = self.get_combined_command_history();

        // Filter history based on search query
        self.reverse_search_results = combined_history
            .iter()
            .filter(|cmd| {
                cmd.to_lowercase()
                    .contains(&self.reverse_search_query.to_lowercase())
            })
            .cloned()
            .collect();

        // Reverse the results so most recent matches come first
        self.reverse_search_results.reverse();

        // Reset index to first result
        self.reverse_search_index = 0;
    }

    /// Navigate to previous search result
    pub fn reverse_search_previous(&mut self) {
        if !self.reverse_search_results.is_empty() && self.reverse_search_index > 0 {
            self.reverse_search_index -= 1;
        }
    }

    /// Navigate to next search result
    pub fn reverse_search_next(&mut self) {
        if !self.reverse_search_results.is_empty()
            && self.reverse_search_index < self.reverse_search_results.len() - 1
        {
            self.reverse_search_index += 1;
        }
    }

    /// Get current search result for display
    pub fn get_current_search_result(&self) -> Option<&String> {
        if self.reverse_search_results.is_empty()
            || self.reverse_search_index >= self.reverse_search_results.len()
        {
            None
        } else {
            Some(&self.reverse_search_results[self.reverse_search_index])
        }
    }

    /// Get reverse search prompt text
    pub fn get_reverse_search_prompt(&self) -> String {
        if self.reverse_search_active {
            let match_info = if self.reverse_search_results.is_empty() {
                "no matches".to_string()
            } else {
                format!(
                    "{}/{}",
                    self.reverse_search_index + 1,
                    self.reverse_search_results.len()
                )
            };
            format!(
                "(reverse-i-search `{}`): {}",
                self.reverse_search_query, match_info
            )
        } else {
            String::new()
        }
    }

    /// Move cursor backward by word (Ctrl+Left)
    pub fn move_cursor_word_backward(&mut self) {
        if self.cursor_position == 0 {
            return;
        }

        let chars: Vec<char> = self.current_input.chars().collect();
        let mut pos = self.cursor_position.min(chars.len());

        // Move backward from current position
        pos = pos.saturating_sub(1);

        // Skip whitespace
        while pos > 0 && chars[pos].is_whitespace() {
            pos -= 1;
        }

        // Skip non-whitespace characters to find the beginning of the word
        while pos > 0 && !chars[pos - 1].is_whitespace() {
            pos -= 1;
        }

        self.cursor_position = pos;
        self.update_auto_suggestion();
    }

    /// Move cursor forward by word (Ctrl+Right)
    pub fn move_cursor_word_forward(&mut self) {
        let chars: Vec<char> = self.current_input.chars().collect();
        let char_count = chars.len();

        if self.cursor_position >= char_count {
            return;
        }

        let mut pos = self.cursor_position;

        // Skip current word (non-whitespace characters)
        while pos < char_count && !chars[pos].is_whitespace() {
            pos += 1;
        }

        // Skip whitespace to find the beginning of the next word
        while pos < char_count && chars[pos].is_whitespace() {
            pos += 1;
        }

        self.cursor_position = pos;
        self.update_auto_suggestion();
    }

    /// Clear the terminal screen and all command history
    pub fn clear_screen(&mut self) {
        // Clear entire command history (no scroll-back access)
        self.command_history.clear();

        // Reset display state to show a clean screen
        self.scroll_offset = 0; // Reset to bottom
        self.current_input.clear(); // Clear current input
        self.cursor_position = 0; // Reset cursor
        self.show_command_list = false; // Hide command list
        self.command_filter.clear(); // Clear command filter
        self.selected_command_index = 0; // Reset command selection
        self.reverse_search_active = false; // Exit reverse search mode
        self.reverse_search_query.clear(); // Clear search query
        self.reverse_search_results.clear(); // Clear search results
        self.reverse_search_index = 0; // Reset search index
        self.output_search_active = false; // Exit output search mode
        self.output_search_query.clear(); // Clear output search query
        self.output_search_matches.clear(); // Clear output search matches
        self.output_search_current_match = 0; // Reset output search index
        self.auto_suggestion = None; // Clear auto-suggestion
        self.reset_history_navigation(); // Reset history navigation

        // Clear any text selections
        self.clear_selection();
        self.clear_input_selection();
    }

    /// Start output search mode
    pub fn start_output_search(&mut self) {
        self.output_search_active = true;
        self.output_search_query.clear();
        self.output_search_matches.clear();
        self.output_search_current_match = 0;
        self.update_output_search();
    }

    /// Cancel output search and restore original state
    pub fn cancel_output_search(&mut self) {
        self.output_search_active = false;
        self.output_search_query.clear();
        self.output_search_matches.clear();
        self.output_search_current_match = 0;
    }

    /// Update output search results based on current query
    pub fn update_output_search(&mut self) {
        self.output_search_matches.clear();
        self.output_search_current_match = 0;

        if self.output_search_query.is_empty() {
            return;
        }

        let search_query = self.output_search_query.clone();
        let search_mode = self.output_search_mode.clone();
        let mut matches = Vec::new();
        let mut line_index = 0;

        // Search through command history
        for entry in &self.command_history {
            // Search in command line
            let command_text = format!("> {}", entry.command);
            Self::search_in_text_static(
                &command_text,
                line_index,
                &search_query,
                &search_mode,
                &mut matches,
            );
            line_index += 1;

            // Search in output lines
            if !entry.output.is_empty() {
                for line in entry.output.lines() {
                    Self::search_in_text_static(
                        line,
                        line_index,
                        &search_query,
                        &search_mode,
                        &mut matches,
                    );
                    line_index += 1;
                }
            }

            // Empty line for spacing
            line_index += 1;
        }

        self.output_search_matches = matches;

        // Auto-scroll to first match if any
        if !self.output_search_matches.is_empty() {
            self.scroll_to_search_match(0);
        }
    }

    fn search_in_text_static(
        text: &str,
        line_index: usize,
        search_query: &str,
        search_mode: &SearchMode,
        matches: &mut Vec<(usize, usize, usize)>,
    ) {
        match search_mode {
            SearchMode::CaseInsensitive => {
                let search_query = search_query.to_lowercase();
                let search_text = text.to_lowercase();
                let mut start = 0;
                while let Some(pos) = search_text[start..].find(&search_query) {
                    let actual_pos = start + pos;
                    let end_pos = actual_pos + search_query.len();
                    matches.push((line_index, actual_pos, end_pos));
                    start = actual_pos + 1;
                }
            }
            SearchMode::CaseSensitive => {
                let mut start = 0;
                while let Some(pos) = text[start..].find(search_query) {
                    let actual_pos = start + pos;
                    let end_pos = actual_pos + search_query.len();
                    matches.push((line_index, actual_pos, end_pos));
                    start = actual_pos + 1;
                }
            }
            SearchMode::Regex => {
                if let Ok(regex) = Regex::new(search_query) {
                    for mat in regex.find_iter(text) {
                        matches.push((line_index, mat.start(), mat.end()));
                    }
                }
                // If regex compilation fails, silently continue (matches will be empty)
            }
        }
    }

    /// Navigate to previous search match
    pub fn output_search_previous_match(&mut self) {
        if self.output_search_matches.is_empty() {
            return;
        }

        if self.output_search_current_match > 0 {
            self.output_search_current_match -= 1;
        } else {
            self.output_search_current_match = self.output_search_matches.len() - 1;
        }

        self.scroll_to_search_match(self.output_search_current_match);
    }

    /// Navigate to next search match
    pub fn output_search_next_match(&mut self) {
        if self.output_search_matches.is_empty() {
            return;
        }

        if self.output_search_current_match < self.output_search_matches.len() - 1 {
            self.output_search_current_match += 1;
        } else {
            self.output_search_current_match = 0;
        }

        self.scroll_to_search_match(self.output_search_current_match);
    }

    /// Scroll to make the specified search match visible
    fn scroll_to_search_match(&mut self, match_index: usize) {
        if match_index >= self.output_search_matches.len() {
            return;
        }

        let (target_line, _, _) = self.output_search_matches[match_index];
        let total_lines = self.get_total_history_lines();
        let available_height = self.history_area_height.saturating_sub(2) as usize;

        // Calculate scroll offset to center the target line
        let desired_scroll = if total_lines <= available_height {
            0 // All content fits, no scrolling needed
        } else {
            let center_offset = available_height / 2;
            if target_line >= center_offset {
                total_lines.saturating_sub(target_line + center_offset)
            } else {
                total_lines.saturating_sub(available_height)
            }
        };

        self.scroll_offset = desired_scroll;
    }

    /// Get search status text for display
    pub fn get_output_search_status(&self) -> String {
        if self.output_search_active {
            let mode_indicator = match self.output_search_mode {
                SearchMode::CaseInsensitive => "[aa]",
                SearchMode::CaseSensitive => "[Aa]",
                SearchMode::Regex => "[.*]",
            };

            if self.output_search_matches.is_empty() {
                if self.output_search_query.is_empty() {
                    format!("Search {mode_indicator}: (Type to search output, Tab to toggle mode)")
                } else {
                    format!(
                        "Search {} '{}' - No matches",
                        mode_indicator, self.output_search_query
                    )
                }
            } else {
                format!(
                    "Search {} '{}' - Match {}/{}",
                    mode_indicator,
                    self.output_search_query,
                    self.output_search_current_match + 1,
                    self.output_search_matches.len()
                )
            }
        } else {
            String::new()
        }
    }

    /// Toggle search mode for output search (case-insensitive -> case-sensitive -> regex)
    pub fn toggle_output_search_mode(&mut self) {
        self.output_search_mode = match self.output_search_mode {
            SearchMode::CaseInsensitive => SearchMode::CaseSensitive,
            SearchMode::CaseSensitive => SearchMode::Regex,
            SearchMode::Regex => SearchMode::CaseInsensitive,
        };
        self.update_output_search();
    }

    /// Get current search matches for highlighting
    pub fn get_output_search_matches(&self) -> &[(usize, usize, usize)] {
        &self.output_search_matches
    }

    /// Get current search match index
    pub fn get_current_search_match(&self) -> usize {
        self.output_search_current_match
    }
}
