use crate::db::models::Task;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum CompletionType {
    Command,
    FilePath,
    TaskTitle,
    TaskId,
    Bash,
    BashSubcommand,
    BashSwitch,
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
    command_cache: Mutex<HashMap<String, CachedCompletion>>,
    path_commands: Mutex<Option<(Vec<String>, Instant)>>,
}

#[derive(Debug, Clone)]
struct CachedCompletion {
    completions: Vec<String>,
    timestamp: Instant,
}

const CACHE_DURATION: Duration = Duration::from_secs(300); // 5 minutes
const PATH_CACHE_DURATION: Duration = Duration::from_secs(60); // 1 minute

impl CompletionEngine {
    pub fn new(available_commands: Vec<String>) -> Self {
        Self {
            available_commands,
            command_cache: Mutex::new(HashMap::new()),
            path_commands: Mutex::new(None),
        }
    }

    pub fn update_commands(&mut self, commands: Vec<String>) {
        self.available_commands = commands;
    }

    /// Get all available commands from PATH, with caching
    pub fn get_path_commands(&self) -> Vec<String> {
        let mut path_commands = self.path_commands.lock().unwrap();

        // Check if we have cached data and it's still valid
        if let Some((ref commands, timestamp)) = *path_commands {
            if timestamp.elapsed() < PATH_CACHE_DURATION {
                return commands.clone();
            }
        }

        // Cache is expired or doesn't exist, rebuild it
        let mut commands = Vec::new();

        if let Ok(path_var) = std::env::var("PATH") {
            for path_dir in path_var.split(':') {
                if let Ok(entries) = std::fs::read_dir(path_dir) {
                    for entry in entries.flatten() {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_file() {
                                let filename = entry.file_name().to_string_lossy().to_string();
                                // Filter out files with extensions and hidden files
                                if !filename.contains('.') && !filename.starts_with('.') {
                                    commands.push(filename);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Remove duplicates and sort
        commands.sort();
        commands.dedup();

        // Cache the result
        *path_commands = Some((commands.clone(), Instant::now()));
        commands
    }

    /// Execute bash completion for a given command line
    pub fn execute_bash_completion(&self, line: &str, cursor_pos: usize) -> Option<Vec<String>> {
        // Skip bash completion in test environment - check this FIRST
        if cfg!(test) {
            return None;
        }

        let cache_key = format!("{line}-{cursor_pos}");

        // Check cache second
        {
            let cache = self.command_cache.lock().unwrap();
            if let Some(cached) = cache.get(&cache_key) {
                if cached.timestamp.elapsed() < CACHE_DURATION {
                    return Some(cached.completions.clone());
                }
            }
        }

        // Try to get completion from bash with timeout
        let completions = self.get_bash_completion_results_with_timeout(line, cursor_pos);

        // Cache the result if we got one
        if let Some(ref comps) = completions {
            let mut cache = self.command_cache.lock().unwrap();
            cache.insert(
                cache_key,
                CachedCompletion {
                    completions: comps.clone(),
                    timestamp: Instant::now(),
                },
            );
        }

        completions
    }

    /// Get bash completion results with timeout for tests
    fn get_bash_completion_results_with_timeout(
        &self,
        input: &str,
        cursor_pos: usize,
    ) -> Option<Vec<String>> {
        // Skip bash completion in test environment to avoid slow tests
        if cfg!(test) {
            return None;
        }

        // Check if this is a cargo command - cargo completion is notoriously slow
        // so skip it and use built-in completions instead
        let words: Vec<&str> = input.split_whitespace().collect();
        if !words.is_empty() && words[0] == "cargo" {
            return None; // Force fallback to built-in cargo completions
        }

        // For other commands, use the full bash completion
        self.get_bash_completion_results(input, cursor_pos)
    }

    /// Get completions using bash's programmable completion
    fn get_bash_completion_results(&self, input: &str, cursor_pos: usize) -> Option<Vec<String>> {
        let words: Vec<&str> = input.split_whitespace().collect();
        if words.is_empty() {
            return None;
        }

        let command = words[0];
        let current_word = if cursor_pos < input.len() {
            // Find the word at cursor position
            let before_cursor = &input[..cursor_pos];
            before_cursor.split_whitespace().last().unwrap_or("")
        } else {
            words.last().unwrap_or(&"")
        };

        // Create a more robust bash script that properly sets up completion

        let bash_script = format!(
            r#"
#!/bin/bash
set +H

# Setup completion environment
export COMP_LINE='{}'
export COMP_POINT={}

# Parse words properly
readarray -t COMP_WORDS <<< '{}'
export COMP_WORDS
export COMP_CWORD={}

# Source the bash completion system
if [ -f /usr/share/bash-completion/bash_completion ]; then
    source /usr/share/bash-completion/bash_completion 2>/dev/null
elif [ -f /etc/bash_completion ]; then
    source /etc/bash_completion 2>/dev/null
fi

# Try to load specific completion for the command
if [ -f "/usr/share/bash-completion/completions/{}" ]; then
    source "/usr/share/bash-completion/completions/{}" 2>/dev/null
fi

# Get the completion function
COMP_FUNC=$(complete -p '{}' 2>/dev/null | sed -n 's/.*-F \([^ ]*\).*/\1/p')

if [ -n "$COMP_FUNC" ] && [ "$COMP_FUNC" != "complete" ]; then
    # Clear COMPREPLY and run the completion function
    unset COMPREPLY
    declare -a COMPREPLY

    # Call the completion function with proper arguments
    $COMP_FUNC "{}" "{}" "{}"

    # Output completions
    printf '%s\n' "${{COMPREPLY[@]}}"
else
    # Fallback: try compgen with various options
    compgen -W "$(compgen -c | grep '^{}'*)" -- '{}' 2>/dev/null || \
    compgen -f -- '{}' 2>/dev/null || \
    compgen -d -- '{}' 2>/dev/null || true
fi
"#,
            input.replace('"', "\\\"''\\\"").replace('`', "\\`"), // Escape quotes and backticks
            cursor_pos,
            words
                .iter()
                .map(|w| w.replace('"', "\\\"''\\\""))
                .collect::<Vec<_>>()
                .join("\n"), // Each word on new line
            words.len().saturating_sub(1),
            command,
            command,
            command,
            command,      // First argument to completion function
            current_word, // Second argument (current word being completed)
            words.get(words.len().saturating_sub(2)).unwrap_or(&""), // Previous word
            command,
            current_word,
            current_word,
            current_word
        );

        let output = Command::new("bash")
            .arg("-c")
            .arg(&bash_script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .ok()?;

        if output.status.success() {
            let completions_text = String::from_utf8_lossy(&output.stdout);
            let _stderr_text = String::from_utf8_lossy(&output.stderr);

            let completions: Vec<String> = completions_text
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty() && line.starts_with(current_word))
                .map(|line| {
                    // Return only the suffix that completes the current word
                    if line.len() > current_word.len() {
                        line[current_word.len()..].to_string()
                    } else {
                        String::new()
                    }
                })
                .filter(|line| !line.is_empty())
                .collect();

            if !completions.is_empty() {
                return Some(completions);
            }
        }

        None
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
        } else if self.is_switch_context(input, word_start) {
            // Switch/option completion (for commands like "git checkout --")
            completions.extend(self.complete_switches(input, word, word_start));
        } else if self.is_subcommand_context(input, word_start) {
            // Bash subcommand completion (for commands like "git checkout")
            completions.extend(self.complete_bash_subcommands(input, word, word_start, cursor_pos));
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

    pub fn is_switch_context(&self, input: &str, word_start: usize) -> bool {
        // Check if we're completing a switch/option (starting with - or --)
        if word_start == 0 {
            return false;
        }

        let word = &input[word_start..];

        // Check if the current word starts with - or --
        if word.starts_with('-') {
            return true;
        }

        // Check if the previous word was a switch that expects a value
        let words: Vec<&str> = input[..word_start].split_whitespace().collect();
        if let Some(prev_word) = words.last() {
            // Some switches take values, so we should offer file/value completion
            matches!(
                *prev_word,
                "--file" | "--config" | "--output" | "-f" | "-o" | "-c"
            )
        } else {
            false
        }
    }

    pub fn is_subcommand_context(&self, input: &str, word_start: usize) -> bool {
        // Check if we're completing a subcommand (after the first word)
        if word_start == 0 {
            return false;
        }

        // Don't treat switch completion as subcommand completion
        let word = &input[word_start..];
        if word.starts_with('-') {
            return false;
        }

        // Split the input to find the first command
        let words: Vec<&str> = input[..word_start].split_whitespace().collect();
        if words.is_empty() {
            return false;
        }

        let first_command = words[0];

        // Check if this command has a completion script available
        // This is a good indicator that it has subcommands
        let completion_path = format!("/usr/share/bash-completion/completions/{first_command}");
        if std::path::Path::new(&completion_path).exists() {
            return true;
        }

        // Fallback to known commands that have subcommands
        matches!(
            first_command,
            "git" | "cargo" | "npm" | "docker" | "kubectl" | "helm" | "aws" | "gcloud" | "az"
        )
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

    pub fn complete_bash_commands(&self, word: &str) -> Vec<Completion> {
        let mut completions = Vec::new();

        // Get all available commands from PATH
        let path_commands = self.get_path_commands();

        for cmd in &path_commands {
            if let Some(stripped) = cmd.strip_prefix(word) {
                completions.push(Completion::new(stripped.to_string(), CompletionType::Bash));
            }
        }

        // Remove duplicates and sort
        completions.sort_by(|a, b| a.text.cmp(&b.text));
        completions.dedup_by(|a, b| a.text == b.text);

        // Limit to reasonable number
        completions.truncate(50);
        completions
    }

    fn complete_bash_subcommands(
        &self,
        input: &str,
        word: &str,
        word_start: usize,
        cursor_pos: usize,
    ) -> Vec<Completion> {
        let mut completions = Vec::new();

        // Parse the command line to get the main command
        let words: Vec<&str> = input[..word_start].split_whitespace().collect();
        if words.is_empty() {
            return completions;
        }

        let main_command = words[0];

        // Try to use bash completion if available
        if let Some(bash_completions) = self.execute_bash_completion(input, cursor_pos) {
            for completion in bash_completions {
                // The bash completion function already returns the suffix,
                // so we don't need to strip the prefix again
                completions.push(Completion::new(completion, CompletionType::BashSubcommand));
            }
        } else {
            // Fallback to built-in subcommand knowledge
            completions.extend(self.get_builtin_subcommands(main_command, word));
        }

        completions
    }

    fn complete_switches(&self, input: &str, word: &str, word_start: usize) -> Vec<Completion> {
        let mut completions = Vec::new();

        // Try to use bash completion first
        if let Some(bash_completions) = self.execute_bash_completion(input, word_start + word.len())
        {
            for completion in bash_completions {
                if let Some(stripped) = completion.strip_prefix(word) {
                    completions.push(Completion::new(
                        stripped.to_string(),
                        CompletionType::BashSwitch,
                    ));
                }
            }
        }

        // If no bash completions or we got empty results, fall back to built-in
        if completions.is_empty() {
            let words: Vec<&str> = input[..word_start].split_whitespace().collect();
            if !words.is_empty() {
                let main_command = words[0];
                let subcommand = words.get(1).copied();
                let switches = self.get_builtin_switches(main_command, subcommand);

                for switch in switches {
                    if let Some(stripped) = switch.strip_prefix(word) {
                        completions.push(Completion::new(
                            stripped.to_string(),
                            CompletionType::BashSwitch,
                        ));
                    }
                }
            }
        }

        completions
    }

    fn get_builtin_switches(
        &self,
        main_command: &str,
        subcommand: Option<&str>,
    ) -> Vec<&'static str> {
        match (main_command, subcommand) {
            ("git", Some("checkout")) => vec![
                "--quiet",
                "--force",
                "--ours",
                "--theirs",
                "--track",
                "--no-track",
                "--detach",
                "--orphan",
                "--ignore-skip-worktree-bits",
                "--merge",
                "--conflict",
                "--patch",
                "--ignore-other-worktrees",
                "--overwrite-ignore",
                "--recurse-submodules",
                "--no-recurse-submodules",
                "-q",
                "-f",
                "-b",
                "-B",
                "-t",
                "-l",
                "-d",
                "-m",
                "-p",
            ],
            ("git", Some("commit")) => vec![
                "--message",
                "--all",
                "--interactive",
                "--patch",
                "--include",
                "--only",
                "--verbose",
                "--untracked-files",
                "--dry-run",
                "--short",
                "--branch",
                "--porcelain",
                "--long",
                "--null",
                "--amend",
                "--no-edit",
                "--fixup",
                "--squash",
                "--reset-author",
                "--signoff",
                "--no-verify",
                "--allow-empty",
                "--allow-empty-message",
                "--cleanup",
                "--edit",
                "--no-status",
                "--gpg-sign",
                "--no-gpg-sign",
                "-m",
                "-a",
                "-i",
                "-p",
                "-v",
                "-u",
                "-n",
                "-s",
                "-e",
                "-S",
            ],
            ("git", Some("push")) => vec![
                "--all",
                "--mirror",
                "--delete",
                "--tags",
                "--dry-run",
                "--porcelain",
                "--quiet",
                "--verbose",
                "--progress",
                "--no-progress",
                "--recurse-submodules",
                "--verify",
                "--no-verify",
                "--follow-tags",
                "--signed",
                "--no-signed",
                "--atomic",
                "--no-atomic",
                "--push-option",
                "--receive-pack",
                "--exec",
                "--force-with-lease",
                "--force",
                "--repository",
                "--set-upstream",
                "-u",
                "-f",
                "-d",
                "-n",
                "-v",
                "-q",
                "--prune",
            ],
            ("git", Some("pull")) => vec![
                "--quiet",
                "--verbose",
                "--progress",
                "--no-progress",
                "--recurse-submodules",
                "--no-recurse-submodules",
                "--commit",
                "--no-commit",
                "--edit",
                "--no-edit",
                "--ff",
                "--no-ff",
                "--ff-only",
                "--log",
                "--no-log",
                "--signoff",
                "--no-signoff",
                "--stat",
                "--no-stat",
                "--squash",
                "--no-squash",
                "--strategy",
                "--strategy-option",
                "--verify-signatures",
                "--no-verify-signatures",
                "--summary",
                "--no-summary",
                "--allow-unrelated-histories",
                "--rebase",
                "--no-rebase",
                "--autostash",
                "--no-autostash",
                "-q",
                "-v",
                "-n",
                "-s",
                "-X",
                "-S",
                "-r",
            ],
            ("git", Some("log")) => vec![
                "--oneline",
                "--decorate",
                "--source",
                "--use-mailmap",
                "--full-diff",
                "--log-size",
                "--follow",
                "--no-decorate",
                "--decorate-refs",
                "--decorate-refs-exclude",
                "--graph",
                "--no-graph",
                "--topo-order",
                "--date-order",
                "--reverse",
                "--walk-reflogs",
                "--merge",
                "--boundary",
                "--simplify-by-decoration",
                "--show-linear-break",
                "--pretty",
                "--format",
                "--abbrev-commit",
                "--no-abbrev-commit",
                "--oneline",
                "--encoding",
                "--notes",
                "--no-notes",
                "--show-notes",
                "--standard-notes",
                "--no-standard-notes",
                "--show-signature",
                "--relative-date",
                "--date",
                "--parents",
                "--children",
                "--left-right",
                "--cherry-pick",
                "--cherry-mark",
                "--full-history",
                "--dense",
                "--sparse",
                "--simplify-merges",
                "--ancestry-path",
                "--since",
                "--after",
                "--until",
                "--before",
                "--author",
                "--committer",
                "--grep",
                "--all-match",
                "--invert-grep",
                "--regexp-ignore-case",
                "--basic-regexp",
                "--extended-regexp",
                "--fixed-strings",
                "--perl-regexp",
                "--remove-empty",
                "--merges",
                "--no-merges",
                "--min-parents",
                "--max-parents",
                "--no-min-parents",
                "--no-max-parents",
                "--first-parent",
                "--not",
                "--all",
                "--branches",
                "--tags",
                "--remotes",
                "--glob",
                "--exclude",
                "--ignore-missing",
                "--bisect",
                "--stdin",
                "--cherry",
                "--patch",
                "--no-patch",
                "--unified",
                "--raw",
                "--patch-with-raw",
                "--indent-heuristic",
                "--no-indent-heuristic",
                "--minimal",
                "--patience",
                "--histogram",
                "--diff-algorithm",
                "--stat",
                "--numstat",
                "--shortstat",
                "--dirstat",
                "--summary",
                "--patch-with-stat",
                "--name-only",
                "--name-status",
                "--submodule",
                "--color",
                "--no-color",
                "--word-diff",
                "--word-diff-regex",
                "--color-words",
                "--no-renames",
                "--check",
                "--ws-error-highlight",
                "--full-index",
                "--binary",
                "--abbrev",
                "--break-rewrites",
                "--find-renames",
                "--find-copies",
                "--find-copies-harder",
                "--irreversible-delete",
                "--diff-filter",
                "--pickaxe-all",
                "--pickaxe-regex",
                "--skip",
                "--max-count",
                "--since",
                "--after",
                "--until",
                "--before",
                "-n",
                "-p",
                "-u",
                "--stat",
                "--graph",
            ],
            ("git", Some("branch")) => vec![
                "--list",
                "--create-reflog",
                "--edit-description",
                "--no-edit-description",
                "--delete",
                "--force",
                "--move",
                "--copy",
                "--color",
                "--no-color",
                "--column",
                "--no-column",
                "--sort",
                "--points-at",
                "--format",
                "--verbose",
                "--quiet",
                "--abbrev",
                "--no-abbrev",
                "--track",
                "--no-track",
                "--set-upstream",
                "--unset-upstream",
                "--set-upstream-to",
                "--contains",
                "--no-contains",
                "--merged",
                "--no-merged",
                "--all",
                "--remotes",
                "-l",
                "-d",
                "-D",
                "-f",
                "-m",
                "-M",
                "-c",
                "-C",
                "-r",
                "-a",
                "-v",
                "-vv",
                "-q",
                "-t",
                "-u",
            ],
            ("git", Some("status")) => vec![
                "--short",
                "--branch",
                "--porcelain",
                "--long",
                "--verbose",
                "--untracked-files",
                "--ignore-submodules",
                "--ignored",
                "--column",
                "--no-column",
                "--ahead-behind",
                "--no-ahead-behind",
                "--renames",
                "--no-renames",
                "--find-renames",
                "-s",
                "-b",
                "-v",
                "-u",
                "--porcelain=v1",
                "--porcelain=v2",
            ],
            ("git", Some("diff")) => vec![
                "--cached",
                "--staged",
                "--no-index",
                "--name-only",
                "--name-status",
                "--check",
                "--stat",
                "--numstat",
                "--shortstat",
                "--dirstat",
                "--summary",
                "--patch-with-stat",
                "--patch",
                "--no-patch",
                "--unified",
                "--raw",
                "--patch-with-raw",
                "--minimal",
                "--patience",
                "--histogram",
                "--diff-algorithm",
                "--word-diff",
                "--word-diff-regex",
                "--color-words",
                "--no-renames",
                "--color",
                "--no-color",
                "--ws-error-highlight",
                "--full-index",
                "--binary",
                "--abbrev",
                "--break-rewrites",
                "--find-renames",
                "--find-copies",
                "--find-copies-harder",
                "--irreversible-delete",
                "--diff-filter",
                "--pickaxe-all",
                "--pickaxe-regex",
                "--no-prefix",
                "--src-prefix",
                "--dst-prefix",
                "--line-prefix",
                "--ita-invisible-in-index",
                "--ita-visible-in-index",
                "-p",
                "-u",
                "-w",
                "-b",
                "--ignore-space-at-eol",
                "--ignore-space-change",
                "--ignore-all-space",
                "--ignore-blank-lines",
                "--ignore-cr-at-eol",
                "--function-context",
                "-W",
                "--exit-code",
                "--quiet",
                "--ext-diff",
                "--no-ext-diff",
                "--textconv",
                "--no-textconv",
                "--ignore-submodules",
                "--submodule",
                "--src-prefix",
                "--dst-prefix",
                "--no-prefix",
            ],
            ("git", Some("add")) => vec![
                "--verbose",
                "--dry-run",
                "--force",
                "--interactive",
                "--patch",
                "--edit",
                "--all",
                "--ignore-removal",
                "--update",
                "--no-ignore-removal",
                "--intent-to-add",
                "--refresh",
                "--ignore-errors",
                "--ignore-missing",
                "--no-all",
                "--no-ignore-removal",
                "--chmod",
                "--pathspec-from-file",
                "--pathspec-file-nul",
                "-v",
                "-n",
                "-f",
                "-i",
                "-p",
                "-e",
                "-A",
                "-u",
                "-N",
            ],
            ("git", Some("reset")) => vec![
                "--soft",
                "--mixed",
                "--hard",
                "--merge",
                "--keep",
                "--patch",
                "--quiet",
                "--no-quiet",
                "--pathspec-from-file",
                "--pathspec-file-nul",
                "-q",
                "-p",
            ],
            ("git", Some("merge")) => vec![
                "--commit",
                "--no-commit",
                "--edit",
                "--no-edit",
                "--ff",
                "--no-ff",
                "--ff-only",
                "--log",
                "--no-log",
                "--signoff",
                "--no-signoff",
                "--stat",
                "--no-stat",
                "--squash",
                "--no-squash",
                "--strategy",
                "--strategy-option",
                "--verify-signatures",
                "--no-verify-signatures",
                "--summary",
                "--no-summary",
                "--quiet",
                "--verbose",
                "--progress",
                "--no-progress",
                "--allow-unrelated-histories",
                "--into-name",
                "--file",
                "--message",
                "--rerere-autoupdate",
                "--no-rerere-autoupdate",
                "--overwrite-ignore",
                "--signoff",
                "--no-signoff",
                "--verify",
                "--no-verify",
                "-n",
                "-v",
                "-q",
                "-m",
                "-F",
                "-e",
                "-s",
                "-X",
                "-S",
            ],
            ("git", Some("rebase")) => vec![
                "--onto",
                "--continue",
                "--abort",
                "--quit",
                "--skip",
                "--edit-todo",
                "--show-current-patch",
                "--interactive",
                "--preserve-merges",
                "--exec",
                "--root",
                "--autosquash",
                "--no-autosquash",
                "--autostash",
                "--no-autostash",
                "--no-verify",
                "--verify",
                "--keep-empty",
                "--skip-empty",
                "--signoff",
                "--strategy",
                "--strategy-option",
                "--gpg-sign",
                "--no-gpg-sign",
                "--quiet",
                "--verbose",
                "--stat",
                "--no-stat",
                "--no-verify",
                "--verify",
                "--force-rebase",
                "--no-force-rebase",
                "--fork-point",
                "--no-fork-point",
                "--ignore-whitespace",
                "--whitespace",
                "--committer-date-is-author-date",
                "--ignore-date",
                "--reset-author-date",
                "--rerere-autoupdate",
                "--no-rerere-autoupdate",
                "-i",
                "-p",
                "-x",
                "-r",
                "-f",
                "-q",
                "-v",
                "-n",
                "-s",
                "-S",
                "-X",
            ],
            ("git", Some("remote")) => vec![
                "--verbose",
                "add",
                "rename",
                "remove",
                "rm",
                "set-head",
                "set-branches",
                "get-url",
                "set-url",
                "show",
                "prune",
                "update",
                "-v",
            ],
            ("git", Some("tag")) => vec![
                "--annotate",
                "--sign",
                "--no-sign",
                "--local-user",
                "--force",
                "--delete",
                "--verify",
                "--no-verify",
                "--list",
                "--sort",
                "--format",
                "--color",
                "--no-color",
                "--column",
                "--no-column",
                "--contains",
                "--no-contains",
                "--merged",
                "--no-merged",
                "--points-at",
                "--message",
                "--file",
                "--cleanup",
                "--create-reflog",
                "--edit",
                "--no-edit",
                "-a",
                "-s",
                "-u",
                "-f",
                "-d",
                "-v",
                "-n",
                "-l",
                "-m",
                "-F",
                "-e",
            ],
            ("git", Some("stash")) => vec![
                "push",
                "pop",
                "apply",
                "drop",
                "clear",
                "list",
                "show",
                "branch",
                "create",
                "store",
                "--patch",
                "--keep-index",
                "--no-keep-index",
                "--include-untracked",
                "--all",
                "--quiet",
                "--index",
                "--message",
                "-p",
                "-k",
                "-u",
                "-a",
                "-q",
                "-m",
            ],
            ("git", Some("config")) => vec![
                "--system",
                "--global",
                "--local",
                "--worktree",
                "--file",
                "--blob",
                "--get",
                "--get-all",
                "--get-regexp",
                "--get-urlmatch",
                "--replace-all",
                "--add",
                "--unset",
                "--unset-all",
                "--rename-section",
                "--remove-section",
                "--list",
                "--type",
                "--bool",
                "--int",
                "--bool-or-int",
                "--path",
                "--expiry-date",
                "--null",
                "--name-only",
                "--includes",
                "--no-includes",
                "--show-origin",
                "--show-scope",
                "--get-colorbool",
                "--get-color",
                "--edit",
                "--fixed-value",
                "-z",
                "-l",
                "-e",
                "-f",
            ],
            ("git", None) => vec![
                "--version",
                "--help",
                "--exec-path",
                "--html-path",
                "--man-path",
                "--info-path",
                "--paginate",
                "--no-pager",
                "--git-dir",
                "--work-tree",
                "--namespace",
                "--super-prefix",
                "--bare",
                "--no-replace-objects",
                "--literal-pathspecs",
                "--glob-pathspecs",
                "--noglob-pathspecs",
                "--icase-pathspecs",
                "--no-optional-locks",
                "--list-cmds",
            ],
            ("cargo", Some("build")) => vec![
                "--package",
                "--workspace",
                "--exclude",
                "--lib",
                "--bin",
                "--bins",
                "--example",
                "--examples",
                "--test",
                "--tests",
                "--bench",
                "--benches",
                "--all-targets",
                "--release",
                "--profile",
                "--features",
                "--all-features",
                "--no-default-features",
                "--target",
                "--target-dir",
                "--out-dir",
                "--manifest-path",
                "--message-format",
                "--verbose",
                "--quiet",
                "--color",
                "--frozen",
                "--locked",
                "--offline",
                "--config",
                "--help",
                "--jobs",
                "--keep-going",
                "--future-incompat-report",
                "--timings",
                "-p",
                "-j",
                "-v",
                "-q",
                "-h",
                "--unit-graph",
                "--build-plan",
            ],
            ("cargo", Some("test")) => vec![
                "--package",
                "--workspace",
                "--exclude",
                "--lib",
                "--bin",
                "--bins",
                "--example",
                "--examples",
                "--test",
                "--tests",
                "--bench",
                "--benches",
                "--all-targets",
                "--doc",
                "--no-run",
                "--no-fail-fast",
                "--release",
                "--profile",
                "--features",
                "--all-features",
                "--no-default-features",
                "--target",
                "--target-dir",
                "--manifest-path",
                "--message-format",
                "--verbose",
                "--quiet",
                "--color",
                "--frozen",
                "--locked",
                "--offline",
                "--config",
                "--help",
                "--jobs",
                "--keep-going",
                "--future-incompat-report",
                "--timings",
                "-p",
                "-j",
                "-v",
                "-q",
                "-h",
            ],
            ("cargo", Some("run")) => vec![
                "--package",
                "--bin",
                "--example",
                "--release",
                "--profile",
                "--features",
                "--all-features",
                "--no-default-features",
                "--target",
                "--target-dir",
                "--manifest-path",
                "--message-format",
                "--verbose",
                "--quiet",
                "--color",
                "--frozen",
                "--locked",
                "--offline",
                "--config",
                "--help",
                "--jobs",
                "--keep-going",
                "--future-incompat-report",
                "--timings",
                "-p",
                "-j",
                "-v",
                "-q",
                "-h",
            ],
            ("cargo", Some("check")) => vec![
                "--package",
                "--workspace",
                "--exclude",
                "--lib",
                "--bin",
                "--bins",
                "--example",
                "--examples",
                "--test",
                "--tests",
                "--bench",
                "--benches",
                "--all-targets",
                "--release",
                "--profile",
                "--features",
                "--all-features",
                "--no-default-features",
                "--target",
                "--target-dir",
                "--manifest-path",
                "--message-format",
                "--verbose",
                "--quiet",
                "--color",
                "--frozen",
                "--locked",
                "--offline",
                "--config",
                "--help",
                "--jobs",
                "--keep-going",
                "--future-incompat-report",
                "--timings",
                "-p",
                "-j",
                "-v",
                "-q",
                "-h",
            ],
            ("cargo", Some("clippy")) => vec![
                "--package",
                "--workspace",
                "--exclude",
                "--lib",
                "--bin",
                "--bins",
                "--example",
                "--examples",
                "--test",
                "--tests",
                "--bench",
                "--benches",
                "--all-targets",
                "--release",
                "--profile",
                "--features",
                "--all-features",
                "--no-default-features",
                "--target",
                "--target-dir",
                "--manifest-path",
                "--message-format",
                "--verbose",
                "--quiet",
                "--color",
                "--frozen",
                "--locked",
                "--offline",
                "--config",
                "--help",
                "--jobs",
                "--keep-going",
                "--future-incompat-report",
                "--timings",
                "--fix",
                "--allow-dirty",
                "--allow-staged",
                "--broken-code",
                "-p",
                "-j",
                "-v",
                "-q",
                "-h",
            ],
            ("cargo", Some("fmt")) => vec![
                "--package",
                "--manifest-path",
                "--message-format",
                "--verbose",
                "--quiet",
                "--color",
                "--frozen",
                "--locked",
                "--offline",
                "--config",
                "--help",
                "--all",
                "--check",
                "--emit",
                "--backup",
                "--config-path",
                "--edition",
                "--print-config",
                "--skip-children",
                "-p",
                "-v",
                "-q",
                "-h",
            ],
            ("cargo", None) => vec![
                "--version",
                "--list",
                "--explain",
                "--verbose",
                "--quiet",
                "--color",
                "--frozen",
                "--locked",
                "--offline",
                "--config",
                "--help",
                "-V",
                "-v",
                "-q",
                "-h",
            ],
            ("ls", None) => vec![
                "--all",
                "--almost-all",
                "--author",
                "--escape",
                "--block-size",
                "--ignore-backups",
                "--color",
                "--directory",
                "--classify",
                "--file-type",
                "--format",
                "--full-time",
                "--group-directories-first",
                "--no-group",
                "--human-readable",
                "--si",
                "--dereference-command-line",
                "--dereference-command-line-symlink-to-dir",
                "--hide",
                "--indicator-style",
                "--inode",
                "--ignore",
                "--kibibytes",
                "--dereference",
                "--literal",
                "--numeric-uid-gid",
                "--no-group",
                "--indicator-style",
                "--hide-control-chars",
                "--show-control-chars",
                "--quote-name",
                "--quoting-style",
                "--reverse",
                "--recursive",
                "--size",
                "--sort",
                "--time",
                "--time-style",
                "--tabsize",
                "--width",
                "--context",
                "--help",
                "--version",
                "-a",
                "-A",
                "-b",
                "-c",
                "-C",
                "-d",
                "-D",
                "-f",
                "-F",
                "-g",
                "-G",
                "-h",
                "-H",
                "-i",
                "-I",
                "-k",
                "-l",
                "-L",
                "-m",
                "-n",
                "-N",
                "-o",
                "-p",
                "-q",
                "-Q",
                "-r",
                "-R",
                "-s",
                "-S",
                "-t",
                "-T",
                "-u",
                "-U",
                "-v",
                "-w",
                "-x",
                "-X",
                "-Z",
                "-1",
            ],
            ("grep", None) => vec![
                "--extended-regexp",
                "--fixed-strings",
                "--basic-regexp",
                "--perl-regexp",
                "--regexp",
                "--file",
                "--ignore-case",
                "--no-ignore-case",
                "--word-regexp",
                "--line-regexp",
                "--null-data",
                "--no-messages",
                "--invert-match",
                "--version",
                "--help",
                "--max-count",
                "--byte-offset",
                "--line-number",
                "--line-buffered",
                "--with-filename",
                "--no-filename",
                "--label",
                "--only-matching",
                "--quiet",
                "--silent",
                "--binary-files",
                "--text",
                "--directories",
                "--devices",
                "--recursive",
                "--include",
                "--exclude",
                "--exclude-from",
                "--exclude-dir",
                "--files-without-match",
                "--files-with-matches",
                "--count",
                "--initial-tab",
                "--null",
                "--before-context",
                "--after-context",
                "--context",
                "--color",
                "--colour",
                "--binary",
                "-E",
                "-F",
                "-G",
                "-P",
                "-e",
                "-f",
                "-i",
                "-v",
                "-w",
                "-x",
                "-z",
                "-s",
                "-V",
                "-m",
                "-b",
                "-n",
                "-H",
                "-h",
                "-o",
                "-q",
                "-a",
                "-I",
                "-d",
                "-D",
                "-r",
                "-R",
                "--include",
                "--exclude",
                "-L",
                "-l",
                "-c",
                "-T",
                "-u",
                "-Z",
                "-A",
                "-B",
                "-C",
            ],
            ("curl", None) => vec![
                "--user",
                "--basic",
                "--digest",
                "--ntlm",
                "--negotiate",
                "--anyauth",
                "--user-agent",
                "--cookie",
                "--cookie-jar",
                "--data",
                "--data-ascii",
                "--data-binary",
                "--data-raw",
                "--data-urlencode",
                "--form",
                "--form-string",
                "--get",
                "--head",
                "--include",
                "--location",
                "--location-trusted",
                "--max-redirs",
                "--output",
                "--remote-name",
                "--remote-name-all",
                "--remote-header-name",
                "--remote-time",
                "--fail",
                "--fail-early",
                "--silent",
                "--show-error",
                "--verbose",
                "--trace",
                "--trace-ascii",
                "--trace-time",
                "--write-out",
                "--config",
                "--request",
                "--http1.0",
                "--http1.1",
                "--http2",
                "--http2-prior-knowledge",
                "--compressed",
                "--connect-timeout",
                "--max-time",
                "--retry",
                "--retry-delay",
                "--retry-max-time",
                "--insecure",
                "--cacert",
                "--capath",
                "--cert",
                "--cert-type",
                "--key",
                "--key-type",
                "--pass",
                "--engine",
                "--ciphers",
                "--dns-servers",
                "--interface",
                "--local-port",
                "--proxy",
                "--proxy-user",
                "--proxy-basic",
                "--proxy-digest",
                "--proxy-ntlm",
                "--proxy-negotiate",
                "--proxy-anyauth",
                "--noproxy",
                "--socks4",
                "--socks4a",
                "--socks5",
                "--socks5-hostname",
                "--socks5-gssapi-service",
                "--socks5-gssapi-nec",
                "--tcp-nodelay",
                "--tcp-fastopen",
                "--unix-socket",
                "--abstract-unix-socket",
                "--happy-eyeballs-timeout-ms",
                "--resolve",
                "--alt-svc",
                "--hsts",
                "--help",
                "--manual",
                "--version",
                "-u",
                "-b",
                "-c",
                "-d",
                "-F",
                "-G",
                "-H",
                "-I",
                "-L",
                "-o",
                "-O",
                "-R",
                "-f",
                "-s",
                "-S",
                "-v",
                "-w",
                "-K",
                "-X",
                "-0",
                "-1",
                "-2",
                "-3",
                "-4",
                "-6",
                "-k",
                "-E",
                "-T",
                "-m",
                "-r",
                "-y",
                "-Y",
                "-z",
                "-#",
                "-:",
                "-;",
                "-N",
                "-J",
                "-j",
                "-C",
                "-t",
                "-n",
                "-a",
                "-A",
                "-e",
                "-U",
                "-x",
                "-p",
                "-P",
                "-Q",
                "-q",
                "-M",
                "-V",
            ],
            ("docker", Some("run")) => vec![
                "--attach",
                "--detach",
                "--interactive",
                "--tty",
                "--name",
                "--hostname",
                "--domainname",
                "--user",
                "--workdir",
                "--env",
                "--env-file",
                "--expose",
                "--publish",
                "--publish-all",
                "--link",
                "--volume",
                "--volumes-from",
                "--mount",
                "--tmpfs",
                "--read-only",
                "--memory",
                "--memory-swap",
                "--memory-swappiness",
                "--memory-reservation",
                "--kernel-memory",
                "--cpu-shares",
                "--cpus",
                "--cpuset-cpus",
                "--cpuset-mems",
                "--cpu-period",
                "--cpu-quota",
                "--cpu-rt-period",
                "--cpu-rt-runtime",
                "--blkio-weight",
                "--blkio-weight-device",
                "--device-read-bps",
                "--device-write-bps",
                "--device-read-iops",
                "--device-write-iops",
                "--oom-kill-disable",
                "--oom-score-adj",
                "--memory-swappiness",
                "--shm-size",
                "--restart",
                "--rm",
                "--runtime",
                "--security-opt",
                "--stop-signal",
                "--stop-timeout",
                "--sysctl",
                "--ulimit",
                "--userns",
                "--uts",
                "--ipc",
                "--pid",
                "--net",
                "--network",
                "--network-alias",
                "--add-host",
                "--mac-address",
                "--ip",
                "--ip6",
                "--dns",
                "--dns-search",
                "--dns-opt",
                "--entrypoint",
                "--device",
                "--device-cgroup-rule",
                "--privileged",
                "--cap-add",
                "--cap-drop",
                "--group-add",
                "--label",
                "--label-file",
                "--log-driver",
                "--log-opt",
                "--cgroup-parent",
                "--cidfile",
                "--init",
                "--platform",
                "--isolation",
                "--pull",
                "--quiet",
                "--disable-content-trust",
                "--help",
                "-a",
                "-d",
                "-i",
                "-t",
                "-h",
                "-u",
                "-w",
                "-e",
                "-p",
                "-P",
                "-v",
                "-m",
                "-c",
                "--rm",
            ],
            ("docker", Some("build")) => vec![
                "--build-arg",
                "--cache-from",
                "--disable-content-trust",
                "--file",
                "--force-rm",
                "--iidfile",
                "--isolation",
                "--label",
                "--memory",
                "--memory-swap",
                "--network",
                "--no-cache",
                "--platform",
                "--pull",
                "--quiet",
                "--rm",
                "--security-opt",
                "--shm-size",
                "--squash",
                "--tag",
                "--target",
                "--ulimit",
                "--compress",
                "--cpu-period",
                "--cpu-quota",
                "--cpu-shares",
                "--cpuset-cpus",
                "--cpuset-mems",
                "--add-host",
                "--build-context",
                "--builder",
                "--progress",
                "--secret",
                "--ssh",
                "--output",
                "--metadata-file",
                "--attest",
                "--sbom",
                "--provenance",
                "--help",
                "-f",
                "-t",
                "-q",
                "--rm",
                "-m",
            ],
            ("docker", Some("ps")) => vec![
                "--all",
                "--filter",
                "--format",
                "--last",
                "--latest",
                "--no-trunc",
                "--quiet",
                "--size",
                "--help",
                "-a",
                "-f",
                "-n",
                "-l",
                "-q",
                "-s",
            ],
            ("docker", Some("images")) => vec![
                "--all",
                "--digests",
                "--filter",
                "--format",
                "--no-trunc",
                "--quiet",
                "--help",
                "-a",
                "-q",
                "-f",
            ],
            ("docker", Some("logs")) => vec![
                "--details",
                "--follow",
                "--since",
                "--until",
                "--tail",
                "--timestamps",
                "--help",
                "-f",
                "-t",
            ],
            ("docker", Some("exec")) => vec![
                "--detach",
                "--detach-keys",
                "--env",
                "--env-file",
                "--interactive",
                "--privileged",
                "--tty",
                "--user",
                "--workdir",
                "--help",
                "-d",
                "-e",
                "-i",
                "-t",
                "-u",
                "-w",
            ],
            ("docker", None) => vec![
                "--config",
                "--context",
                "--debug",
                "--host",
                "--log-level",
                "--tls",
                "--tlscacert",
                "--tlscert",
                "--tlskey",
                "--tlsverify",
                "--version",
                "--help",
                "-c",
                "-D",
                "-H",
                "-l",
                "-v",
            ],
            ("npm", Some("install")) => vec![
                "--save",
                "--save-dev",
                "--save-optional",
                "--save-exact",
                "--no-save",
                "--dry-run",
                "--package-lock",
                "--no-package-lock",
                "--package-lock-only",
                "--global",
                "--global-style",
                "--ignore-scripts",
                "--legacy-bundling",
                "--link",
                "--no-bin-links",
                "--no-optional",
                "--no-shrinkwrap",
                "--nodedir",
                "--only",
                "--optional",
                "--prefer-offline",
                "--prefer-online",
                "--production",
                "--progress",
                "--no-progress",
                "--registry",
                "--silent",
                "--tag",
                "--tmp",
                "--unsafe-perm",
                "--update-notifier",
                "--verbose",
                "--audit",
                "--no-audit",
                "--fund",
                "--no-fund",
                "--help",
                "-S",
                "-D",
                "-O",
                "-E",
                "-g",
                "-f",
                "-q",
                "-d",
                "-dd",
                "-ddd",
                "-s",
                "-v",
            ],
            ("npm", Some("run")) => vec![
                "--silent",
                "--if-present",
                "--ignore-scripts",
                "--script-shell",
                "--workspace",
                "--workspaces",
                "--include-workspace-root",
                "--help",
                "-s",
                "-q",
                "-w",
                "-ws",
                "-iwr",
            ],
            ("npm", Some("test")) => vec![
                "--silent",
                "--if-present",
                "--ignore-scripts",
                "--script-shell",
                "--workspace",
                "--workspaces",
                "--include-workspace-root",
                "--help",
                "-s",
                "-q",
                "-w",
                "-ws",
                "-iwr",
            ],
            ("npm", Some("start")) => vec![
                "--silent",
                "--if-present",
                "--ignore-scripts",
                "--script-shell",
                "--help",
                "-s",
                "-q",
            ],
            ("npm", Some("build")) => vec![
                "--silent",
                "--if-present",
                "--ignore-scripts",
                "--script-shell",
                "--workspace",
                "--workspaces",
                "--include-workspace-root",
                "--help",
                "-s",
                "-q",
                "-w",
                "-ws",
                "-iwr",
            ],
            ("npm", None) => vec![
                "--version",
                "--help",
                "--silent",
                "--loglevel",
                "--registry",
                "--userconfig",
                "--globalconfig",
                "--init-module",
                "--cache",
                "--cache-min",
                "--cache-max",
                "--tmp",
                "--prefix",
                "--global",
                "--unsafe-perm",
                "--ca",
                "--cafile",
                "--cert",
                "--key",
                "--https-proxy",
                "--proxy",
                "--registry",
                "--scope",
                "--tag",
                "--git",
                "--git-tag-version",
                "--commit-hooks",
                "--sign-git-tag",
                "--sign-git-commit",
                "--sso-poll-frequency",
                "--sso-type",
                "--strict-ssl",
                "--dry-run",
                "--otp",
                "--workspace",
                "--workspaces",
                "--include-workspace-root",
                "-v",
                "-h",
                "-s",
                "-q",
                "-d",
                "-dd",
                "-ddd",
                "-g",
                "-f",
                "-l",
                "-w",
                "-ws",
                "-iwr",
            ],
            _ => vec![],
        }
    }

    pub fn get_builtin_subcommands(&self, main_command: &str, word: &str) -> Vec<Completion> {
        let subcommands = match main_command {
            "git" => vec![
                "add",
                "branch",
                "checkout",
                "clone",
                "commit",
                "diff",
                "fetch",
                "init",
                "log",
                "merge",
                "pull",
                "push",
                "rebase",
                "remote",
                "reset",
                "show",
                "status",
                "tag",
                "config",
                "help",
                "version",
                "stash",
                "cherry-pick",
                "revert",
                "bisect",
                "grep",
                "mv",
                "rm",
                "clean",
                "describe",
                "shortlog",
                "archive",
                "bundle",
                "gc",
                "fsck",
                "reflog",
                "filter-branch",
                "subtree",
            ],
            "cargo" => vec![
                "build",
                "check",
                "clean",
                "doc",
                "new",
                "init",
                "run",
                "test",
                "bench",
                "update",
                "search",
                "publish",
                "install",
                "uninstall",
                "version",
                "help",
                "clippy",
                "fmt",
                "tree",
                "audit",
                "fix",
                "metadata",
                "vendor",
                "verify-project",
            ],
            "npm" => vec![
                "install",
                "init",
                "start",
                "test",
                "run",
                "build",
                "update",
                "uninstall",
                "publish",
                "version",
                "list",
                "search",
                "info",
                "config",
                "cache",
                "audit",
                "outdated",
                "doctor",
                "fund",
                "help",
                "login",
                "logout",
                "whoami",
                "link",
                "unlink",
            ],
            "docker" => vec![
                "build", "run", "start", "stop", "restart", "pause", "unpause", "kill", "rm",
                "rmi", "pull", "push", "commit", "tag", "images", "ps", "logs", "exec", "attach",
                "cp", "create", "diff", "export", "history", "import", "info", "inspect", "load",
                "network", "node", "plugin", "port", "rename", "save", "search", "stats", "top",
                "update", "version", "volume", "wait",
            ],
            "kubectl" => vec![
                "get",
                "describe",
                "create",
                "apply",
                "delete",
                "edit",
                "patch",
                "replace",
                "expose",
                "run",
                "set",
                "explain",
                "logs",
                "attach",
                "exec",
                "port-forward",
                "proxy",
                "cp",
                "auth",
                "config",
                "cluster-info",
                "top",
                "cordon",
                "uncordon",
                "drain",
                "taint",
                "label",
                "annotate",
                "completion",
                "version",
                "api-versions",
            ],
            _ => vec![],
        };

        subcommands
            .into_iter()
            .filter_map(|cmd| {
                cmd.strip_prefix(word).map(|stripped| {
                    Completion::new(stripped.to_string(), CompletionType::BashSubcommand)
                })
            })
            .collect()
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

    #[test]
    fn test_switch_context_detection() {
        let commands = Vec::new();
        let engine = CompletionEngine::new(commands);

        // Test switch detection - word_start should be the start of the switch being completed
        let input1 = "git checkout --";
        let word_start1 = engine.find_word_start(input1, input1.len());
        assert!(engine.is_switch_context(input1, word_start1));

        let input2 = "git commit -";
        let word_start2 = engine.find_word_start(input2, input2.len());
        assert!(engine.is_switch_context(input2, word_start2));

        let input3 = "cargo build --";
        let word_start3 = engine.find_word_start(input3, input3.len());
        assert!(engine.is_switch_context(input3, word_start3));

        // Test non-switch contexts
        assert!(!engine.is_switch_context("git checkout", 12));
        assert!(!engine.is_switch_context("git checkout branch", 19));
        assert!(!engine.is_switch_context("", 0));
    }

    #[test]
    fn test_switch_completion() {
        let commands = Vec::new();
        let engine = CompletionEngine::new(commands);

        // Test git checkout switches
        let input1 = "git checkout --";
        let word_start1 = engine.find_word_start(input1, input1.len());
        let word1 = &input1[word_start1..];
        let completions = engine.complete_switches(input1, word1, word_start1);
        assert!(!completions.is_empty());

        let switch_texts: Vec<&str> = completions.iter().map(|c| c.text.as_str()).collect();
        assert!(switch_texts.contains(&"quiet"));
        assert!(switch_texts.contains(&"force"));
        assert!(switch_texts.contains(&"track"));

        // Test partial matching
        let input2 = "git checkout --q";
        let word_start2 = engine.find_word_start(input2, input2.len());
        let word2 = &input2[word_start2..];
        let completions = engine.complete_switches(input2, word2, word_start2);
        assert!(!completions.is_empty());
        let switch_texts: Vec<&str> = completions.iter().map(|c| c.text.as_str()).collect();
        assert!(switch_texts.contains(&"uiet")); // Should complete "--quiet"
    }

    #[test]
    fn test_cargo_switch_completion() {
        let commands = Vec::new();
        let engine = CompletionEngine::new(commands);

        // Test cargo build switches
        let input = "cargo build --";
        let word_start = engine.find_word_start(input, input.len());
        let word = &input[word_start..];
        let completions = engine.complete_switches(input, word, word_start);
        assert!(!completions.is_empty());

        let switch_texts: Vec<&str> = completions.iter().map(|c| c.text.as_str()).collect();
        assert!(switch_texts.contains(&"release"));
        assert!(switch_texts.contains(&"verbose"));
        assert!(switch_texts.contains(&"package"));
    }
}
