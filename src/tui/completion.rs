use crate::db::models::Task;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum CompletionType {
    Command,
    FilePath,
    TaskTitle,
    TaskId,
    Bash,
}

#[derive(Debug, Clone)]
pub struct Completion {
    pub text: String,
    pub completion_type: CompletionType,
    pub display_text: Option<String>, // For showing additional info
}

impl Completion {
    pub fn new(text: String, completion_type: CompletionType) -> Self {
        Self {
            text,
            completion_type,
            display_text: None,
        }
    }

    pub fn with_display(text: String, completion_type: CompletionType, display: String) -> Self {
        Self {
            text,
            completion_type,
            display_text: Some(display),
        }
    }
}

#[derive(Debug)]
pub struct CompletionState {
    pub completions: Vec<Completion>,
    pub current_index: usize,
    pub original_input: String,
    pub prefix: String,
    pub is_active: bool,
}

impl Default for CompletionState {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionState {
    pub fn new() -> Self {
        Self {
            completions: Vec::new(),
            current_index: 0,
            original_input: String::new(),
            prefix: String::new(),
            is_active: false,
        }
    }

    pub fn start(&mut self, input: &str, completions: Vec<Completion>, word_start: usize) {
        self.original_input = input.to_string();
        self.completions = completions;
        self.current_index = 0;
        self.is_active = !self.completions.is_empty();

        // The prefix is everything before the word being completed
        self.prefix = input[..word_start].to_string();
    }

    pub fn cycle_next(&mut self) -> Option<String> {
        if !self.is_active || self.completions.is_empty() {
            return None;
        }

        let completion = &self.completions[self.current_index];
        self.current_index = (self.current_index + 1) % self.completions.len();

        // Get the word that's being completed
        let word_start = self.prefix.len();
        let word_end = self.original_input.len();
        let word = &self.original_input[word_start..word_end];

        // Replace the word with the completed word
        Some(format!("{}{}{}", self.prefix, word, completion.text))
    }

    pub fn cycle_previous(&mut self) -> Option<String> {
        if !self.is_active || self.completions.is_empty() {
            return None;
        }

        if self.current_index == 0 {
            self.current_index = self.completions.len() - 1;
        } else {
            self.current_index -= 1;
        }

        let completion = &self.completions[self.current_index];

        // Get the word that's being completed
        let word_start = self.prefix.len();
        let word_end = self.original_input.len();
        let word = &self.original_input[word_start..word_end];

        // Replace the word with the completed word
        Some(format!("{}{}{}", self.prefix, word, completion.text))
    }

    pub fn current_completion(&self) -> Option<&Completion> {
        if self.is_active && !self.completions.is_empty() {
            self.completions.get(self.current_index)
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        self.is_active = false;
        self.completions.clear();
        self.current_index = 0;
        self.original_input.clear();
        self.prefix.clear();
    }
}

pub struct CompletionEngine {
    available_commands: Vec<String>,
}

impl CompletionEngine {
    pub fn new(available_commands: Vec<String>) -> Self {
        Self { available_commands }
    }

    pub fn update_commands(&mut self, commands: Vec<String>) {
        self.available_commands = commands;
    }

    pub fn get_completions(
        &self,
        input: &str,
        cursor_pos: usize,
        tasks: &[Task],
    ) -> Vec<Completion> {
        let mut completions = Vec::new();

        // Get the word being completed
        let word_start = self.find_word_start(input, cursor_pos);
        let word = &input[word_start..cursor_pos];

        // Determine completion context
        if self.is_task_context(input) {
            // Task completion (for commands like "/task Fix")
            completions.extend(self.complete_tasks(word, tasks));
        } else if input.starts_with('/') {
            // Built-in command completion
            completions.extend(self.complete_builtin_commands(word));
        } else if self.is_file_path_context(input, word_start) {
            // File path completion
            completions.extend(self.complete_file_paths(word));
        } else {
            // General command completion (bash commands)
            completions.extend(self.complete_bash_commands(word));
        }

        completions
    }

    pub fn find_word_start(&self, input: &str, cursor_pos: usize) -> usize {
        let chars: Vec<char> = input.chars().collect();
        let mut start = cursor_pos;

        while start > 0 {
            let prev_char = chars.get(start - 1);
            match prev_char {
                Some(' ') | Some('\t') => break,
                _ => start -= 1,
            }
        }

        start
    }

    pub fn is_file_path_context(&self, input: &str, word_start: usize) -> bool {
        if word_start == 0 {
            return false;
        }

        let before_word = &input[..word_start];

        // Check if the word looks like a path or if the command typically takes file arguments
        before_word.ends_with(' ')
            && (before_word.contains("cat ")
                || before_word.contains("ls ")
                || before_word.contains("cd ")
                || before_word.contains("mkdir ")
                || before_word.contains("touch ")
                || before_word.contains("rm ")
                || before_word.contains("cp ")
                || before_word.contains("mv ")
                || before_word.contains("grep ")
                || before_word.contains("find "))
    }

    pub fn is_task_context(&self, input: &str) -> bool {
        input.contains("/task ") || input.contains("/done ") || input.contains("/progress ")
    }

    fn complete_builtin_commands(&self, word: &str) -> Vec<Completion> {
        self.available_commands
            .iter()
            .filter_map(|cmd| {
                cmd.strip_prefix(word)
                    .map(|stripped| Completion::new(stripped.to_string(), CompletionType::Command))
            })
            .collect()
    }

    fn complete_file_paths(&self, word: &str) -> Vec<Completion> {
        let mut completions = Vec::new();

        let path = if word.is_empty() {
            PathBuf::from(".")
        } else {
            PathBuf::from(word)
        };

        let (dir, filename_prefix) = if path.is_dir() {
            (path, String::new())
        } else {
            let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
            let filename_prefix = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            (dir, filename_prefix)
        };

        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let filename = entry.file_name().to_string_lossy().to_string();

                if filename.starts_with(&filename_prefix) && !filename.starts_with('.') {
                    let mut completion_text = filename[filename_prefix.len()..].to_string();

                    // Add trailing slash for directories
                    if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        completion_text.push('/');
                    }

                    let display_text = if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        format!("{filename} (dir)")
                    } else {
                        filename.clone()
                    };

                    completions.push(Completion::with_display(
                        completion_text,
                        CompletionType::FilePath,
                        display_text,
                    ));
                }
            }
        }

        completions.sort_by(|a, b| a.text.cmp(&b.text));
        completions
    }

    fn complete_tasks(&self, word: &str, tasks: &[Task]) -> Vec<Completion> {
        let mut completions = Vec::new();

        for task in tasks {
            // Complete by task title
            if task.title.to_lowercase().contains(&word.to_lowercase()) {
                let display = format!("{} ({})", task.title, task.status);
                completions.push(Completion::with_display(
                    task.title.clone(),
                    CompletionType::TaskTitle,
                    display,
                ));
            }

            // Complete by task ID (first 8 chars of UUID)
            let short_id = task.id.to_string()[..8].to_string();
            if short_id.starts_with(word) {
                let display = format!("{} - {}", short_id, task.title);
                completions.push(Completion::with_display(
                    short_id,
                    CompletionType::TaskId,
                    display,
                ));
            }
        }

        completions.sort_by(|a, b| a.text.cmp(&b.text));
        completions
    }

    fn complete_bash_commands(&self, word: &str) -> Vec<Completion> {
        let mut completions = Vec::new();

        // Common bash commands
        let common_commands = [
            "ls", "cd", "pwd", "mkdir", "rmdir", "rm", "cp", "mv", "touch", "cat", "less", "more",
            "head", "tail", "grep", "find", "which", "echo", "export", "env", "ps", "kill", "jobs",
            "fg", "bg", "git", "cargo", "npm", "python", "node", "curl", "wget",
        ];

        for cmd in &common_commands {
            if let Some(stripped) = cmd.strip_prefix(word) {
                completions.push(Completion::new(stripped.to_string(), CompletionType::Bash));
            }
        }

        // Try to get commands from PATH
        if let Ok(path_var) = std::env::var("PATH") {
            for path_dir in path_var.split(':') {
                if let Ok(entries) = std::fs::read_dir(path_dir) {
                    for entry in entries.flatten() {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_file() {
                                let filename = entry.file_name().to_string_lossy().to_string();
                                if filename.starts_with(word) && !filename.contains('.') {
                                    completions.push(Completion::new(
                                        filename[word.len()..].to_string(),
                                        CompletionType::Bash,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Remove duplicates and sort
        completions.sort_by(|a, b| a.text.cmp(&b.text));
        completions.dedup_by(|a, b| a.text == b.text);

        // Limit to reasonable number
        completions.truncate(50);
        completions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{Priority, TaskSource, TaskStatus};
    use std::collections::HashMap;
    use uuid::Uuid;

    fn create_test_task(title: &str) -> Task {
        Task {
            id: Uuid::new_v4(),
            external_id: None,
            source: TaskSource::Markdown,
            title: title.to_string(),
            description: None,
            status: TaskStatus::Open,
            priority: Priority::Medium,
            assignee: None,
            labels: Vec::new(),
            due_date: None,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
            custom_fields: HashMap::new(),
        }
    }

    #[test]
    fn test_completion_state_cycling() {
        let mut state = CompletionState::new();
        let completions = vec![
            Completion::new("t1".to_string(), CompletionType::Command),
            Completion::new("t2".to_string(), CompletionType::Command),
            Completion::new("t3".to_string(), CompletionType::Command),
        ];

        state.start("tes", completions, 0);

        assert_eq!(state.cycle_next(), Some("test1".to_string()));
        assert_eq!(state.cycle_next(), Some("test2".to_string()));
        assert_eq!(state.cycle_next(), Some("test3".to_string()));
        assert_eq!(state.cycle_next(), Some("test1".to_string())); // Wrap around
    }

    #[test]
    fn test_builtin_command_completion() {
        let commands = vec![
            "/task".to_string(),
            "/help".to_string(),
            "/quit".to_string(),
        ];
        let engine = CompletionEngine::new(commands);
        let tasks = Vec::new();

        let completions = engine.get_completions("/ta", 3, &tasks);
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].text, "sk");
        assert_eq!(completions[0].completion_type, CompletionType::Command);
    }

    #[test]
    fn test_task_completion() {
        let commands = vec!["/task".to_string()];
        let engine = CompletionEngine::new(commands);
        let tasks = vec![
            create_test_task("Fix login bug"),
            create_test_task("Update documentation"),
        ];

        let completions = engine.get_completions("/task Fix", 9, &tasks);
        assert!(!completions.is_empty());

        let has_login_task = completions.iter().any(|c| c.text.contains("Fix login bug"));
        assert!(has_login_task);
    }

    #[test]
    fn test_word_start_finding() {
        let commands = Vec::new();
        let engine = CompletionEngine::new(commands);

        assert_eq!(engine.find_word_start("hello world", 11), 6);
        assert_eq!(engine.find_word_start("hello world", 5), 0);
        assert_eq!(engine.find_word_start("/task add test", 14), 10);
    }
}
