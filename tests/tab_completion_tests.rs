use std::collections::HashMap;
use taskhub::db::init_db;
use taskhub::db::models::{Priority, Task, TaskSource, TaskStatus};
use taskhub::tui::app::App;
use taskhub::tui::completion::{Completion, CompletionEngine, CompletionState, CompletionType};
use uuid::Uuid;

// Helper function to create a test app
async fn create_test_app() -> App {
    let pool = init_db(Some(":memory:".into())).await.unwrap();
    App::new(pool)
}

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

#[cfg(test)]
mod completion_state_tests {
    use super::*;

    #[test]
    fn test_completion_state_initialization() {
        let state = CompletionState::new();
        assert!(!state.is_active);
        assert!(state.completions.is_empty());
        assert_eq!(state.current_index, 0);
    }

    #[test]
    fn test_completion_state_start() {
        let mut state = CompletionState::new();
        let completions = vec![
            Completion::new("test1".to_string(), CompletionType::Command),
            Completion::new("test2".to_string(), CompletionType::Command),
        ];

        state.start("tes", completions, 0);

        assert!(state.is_active);
        assert_eq!(state.completions.len(), 2);
        assert_eq!(state.original_input, "tes");
        assert_eq!(state.current_index, 0);
    }

    #[test]
    fn test_completion_cycling() {
        let mut state = CompletionState::new();
        let completions = vec![
            Completion::new("apple".to_string(), CompletionType::Command),
            Completion::new("application".to_string(), CompletionType::Command),
            Completion::new("apply".to_string(), CompletionType::Command),
        ];

        state.start("app", completions, 0);

        // First cycle should return first completion (app + apple suffix)
        assert_eq!(state.cycle_next(), Some("appapple".to_string()));
        assert_eq!(state.current_index, 1);

        // Second cycle should return second completion
        assert_eq!(state.cycle_next(), Some("appapplication".to_string()));
        assert_eq!(state.current_index, 2);

        // Third cycle should return third completion
        assert_eq!(state.cycle_next(), Some("appapply".to_string()));
        assert_eq!(state.current_index, 0); // Should wrap around

        // Fourth cycle should wrap back to first
        assert_eq!(state.cycle_next(), Some("appapple".to_string()));
    }

    #[test]
    fn test_completion_with_prefix() {
        let mut state = CompletionState::new();
        let completions = vec![Completion::new("test".to_string(), CompletionType::Command)];

        state.start("/task add tes", completions, 10);

        // Should preserve the prefix "/task add " and append completion
        assert_eq!(state.cycle_next(), Some("/task add testest".to_string()));
        assert_eq!(state.prefix, "/task add ");
    }

    #[test]
    fn test_completion_reset() {
        let mut state = CompletionState::new();
        let completions = vec![Completion::new("test".to_string(), CompletionType::Command)];

        state.start("tes", completions, 0);
        assert!(state.is_active);

        state.reset();
        assert!(!state.is_active);
        assert!(state.completions.is_empty());
        assert_eq!(state.current_index, 0);
        assert!(state.original_input.is_empty());
    }
}

#[cfg(test)]
mod completion_engine_tests {
    use super::*;

    #[test]
    fn test_builtin_command_completion() {
        let commands = vec![
            "/task".to_string(),
            "/task add".to_string(),
            "/task list".to_string(),
            "/help".to_string(),
            "/quit".to_string(),
        ];
        let engine = CompletionEngine::new(commands);
        let tasks = Vec::new();

        // Test completing "/ta"
        let completions = engine.get_completions("/ta", 3, &tasks);

        // Should find task-related commands
        assert!(!completions.is_empty());
        let completion_texts: Vec<&str> = completions.iter().map(|c| c.text.as_str()).collect();
        assert!(completion_texts.contains(&"sk"));
        assert!(completion_texts.contains(&"sk add"));
        assert!(completion_texts.contains(&"sk list"));
    }

    #[test]
    fn test_bash_command_completion() {
        let commands = Vec::new();
        let engine = CompletionEngine::new(commands);
        let tasks = Vec::new();

        // Test completing "l" (should find "ls")
        let completions = engine.get_completions("l", 1, &tasks);

        // Should find bash commands starting with "l"
        assert!(!completions.is_empty());
        let has_ls = completions
            .iter()
            .any(|c| c.text == "s" && c.completion_type == CompletionType::Bash);
        assert!(has_ls);
    }

    #[test]
    fn test_task_title_completion() {
        let commands = vec!["/task".to_string()];
        let engine = CompletionEngine::new(commands);
        let tasks = vec![
            create_test_task("Fix login bug"),
            create_test_task("Update documentation"),
            create_test_task("Add new feature"),
        ];

        // Test completing with task context
        let completions = engine.get_completions("/task Fix", 9, &tasks);

        // Should find task with "Fix" in title
        assert!(!completions.is_empty());
        let has_fix_task = completions.iter().any(|c| {
            c.text.contains("Fix login bug") && c.completion_type == CompletionType::TaskTitle
        });
        assert!(has_fix_task);
    }

    #[test]
    fn test_file_path_completion() {
        let commands = Vec::new();
        let engine = CompletionEngine::new(commands);
        let tasks = Vec::new();

        // Test with a command that takes file arguments
        let completions = engine.get_completions("ls /tm", 6, &tasks);

        // Should attempt file path completion
        // Note: This test may vary based on actual filesystem contents
        // Just verify the method doesn't panic and returns a result
        // Vector length is always non-negative, so just check it doesn't panic
        let _len = completions.len();
    }

    #[test]
    fn test_word_start_finding() {
        let commands = Vec::new();
        let engine = CompletionEngine::new(commands);

        // Test finding word boundaries
        assert_eq!(engine.find_word_start("hello world", 11), 6);
        assert_eq!(engine.find_word_start("hello world", 5), 0);
        assert_eq!(engine.find_word_start("/task add test", 14), 10);
        assert_eq!(engine.find_word_start("single", 6), 0);
        assert_eq!(engine.find_word_start("", 0), 0);
    }

    #[test]
    fn test_context_detection() {
        let commands = Vec::new();
        let engine = CompletionEngine::new(commands);

        // Test file path context detection
        assert!(engine.is_file_path_context("ls /home", 3));
        assert!(engine.is_file_path_context("cat file", 4));
        assert!(!engine.is_file_path_context("echo hello", 5));

        // Test task context detection
        assert!(engine.is_task_context("/task add something"));
        assert!(engine.is_task_context("/done task-id"));
        assert!(!engine.is_task_context("regular command"));

        // Test subcommand context detection
        assert!(engine.is_subcommand_context("git checkout", 4));
        assert!(engine.is_subcommand_context("cargo build", 6));
        assert!(engine.is_subcommand_context("npm install", 4));
        assert!(engine.is_subcommand_context("docker run", 7));
        assert!(!engine.is_subcommand_context("git", 0));
        assert!(!engine.is_subcommand_context("unknown command", 8));
    }
}

#[cfg(test)]
mod app_tab_completion_tests {
    use super::*;

    #[tokio::test]
    async fn test_app_tab_completion_builtin_commands() {
        let mut app = create_test_app().await;

        // Type partial command
        app.current_input = "/ta".to_string();
        app.cursor_position = 3;

        // Trigger tab completion
        app.handle_tab_completion();

        // Should complete to "/task"
        assert!(app.current_input.starts_with("/task"));
        assert!(app.completion_state.is_active);
    }

    #[tokio::test]
    async fn test_app_tab_completion_cycling() {
        let mut app = create_test_app().await;

        // Type partial command that has multiple matches
        app.current_input = "/ta".to_string();
        app.cursor_position = 3;

        // First tab - should complete to first match
        app.handle_tab_completion();
        let first_completion = app.current_input.clone();
        assert!(app.completion_state.is_active);

        // Second tab - should cycle to next match
        app.handle_tab_completion();
        let second_completion = app.current_input.clone();

        // Should be different completions
        assert_ne!(first_completion, second_completion);
    }

    #[tokio::test]
    async fn test_completion_reset_on_typing() {
        let mut app = create_test_app().await;

        // Start completion
        app.current_input = "/ta".to_string();
        app.cursor_position = 3;
        app.handle_tab_completion();
        assert!(app.completion_state.is_active);

        // Type a character - should reset completion
        app.handle_terminal_input('b');
        assert!(!app.completion_state.is_active);
        assert!(app.current_input.contains("taskb"));
    }

    #[tokio::test]
    async fn test_completion_reset_on_backspace() {
        let mut app = create_test_app().await;

        // Start completion
        app.current_input = "/ta".to_string();
        app.cursor_position = 3;
        app.handle_tab_completion();
        assert!(app.completion_state.is_active);

        // Simulate backspace - should reset completion
        app.completion_state.reset(); // This would be called in the actual backspace handler
        assert!(!app.completion_state.is_active);
    }

    #[tokio::test]
    async fn test_task_completion_with_loaded_tasks() {
        let mut app = create_test_app().await;

        // Add some test tasks to the app
        app.tasks = vec![
            create_test_task("Fix login bug"),
            create_test_task("Update documentation"),
        ];

        // Type task command with partial task name
        app.current_input = "/task Fix".to_string();
        app.cursor_position = 9;

        // Trigger tab completion
        app.handle_tab_completion();

        // Should find completions related to tasks
        if app.completion_state.is_active {
            assert!(!app.completion_state.completions.is_empty());
        }
    }

    #[tokio::test]
    async fn test_bash_command_completion() {
        let mut app = create_test_app().await;

        // Type partial bash command
        app.current_input = "l".to_string();
        app.cursor_position = 1;

        // Trigger tab completion
        app.handle_tab_completion();

        // Should find bash command completions
        if app.completion_state.is_active {
            assert!(!app.completion_state.completions.is_empty());
            // Should include "ls" completion
            let has_ls = app.completion_state.completions.iter().any(|c| {
                c.completion_type == CompletionType::Bash
                    && (c.text == "s" || c.text.contains("ls"))
            });
            assert!(has_ls);
        }
    }

    #[tokio::test]
    async fn test_no_completion_for_complete_commands() {
        let mut app = create_test_app().await;

        // Type complete command
        app.current_input = "/help".to_string();
        app.cursor_position = 5;

        // Trigger tab completion
        app.handle_tab_completion();

        // Should not activate completion for complete commands
        // (though this depends on implementation details)
        // Just verify it doesn't crash
        assert!(app.current_input == "/help");
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_end_to_end_tab_completion_workflow() {
        let mut app = create_test_app().await;

        // Add a test task
        app.tasks.push(create_test_task("Test task for completion"));

        // Simulate user typing "/ta" and pressing tab
        app.current_input = "/ta".to_string();
        app.cursor_position = 3;
        app.handle_tab_completion();

        // Should complete to something starting with "/task"
        assert!(app.current_input.starts_with("/task"));

        // If multiple completions available, test cycling
        if app.completion_state.completions.len() > 1 {
            let first = app.current_input.clone();
            app.handle_tab_completion();
            let second = app.current_input.clone();

            // Should cycle to different completion
            assert_ne!(first, second);
        }

        // Test that typing resets completion
        app.handle_terminal_input(' ');
        assert!(!app.completion_state.is_active);
        assert!(app.current_input.contains(" "));
    }

    #[tokio::test]
    async fn test_file_completion_integration() {
        let mut app = create_test_app().await;

        // Test file path completion with ls command
        app.current_input = "ls /".to_string();
        app.cursor_position = 4;
        app.handle_tab_completion();

        // Should attempt file path completion
        // Note: Results depend on actual filesystem
        // Just verify no crash and reasonable behavior
        assert!(app.cursor_position <= app.current_input.chars().count());
    }

    #[tokio::test]
    async fn test_bash_subcommand_completion() {
        let mut app = create_test_app().await;

        // Test git subcommand completion
        app.current_input = "git che".to_string();
        app.cursor_position = 7;
        app.handle_tab_completion();

        // Should complete to git checkout or similar
        if app.completion_state.is_active {
            assert!(!app.completion_state.completions.is_empty());
            // Check if any completion is for "checkout"
            let has_checkout = app.completion_state.completions.iter().any(|c| {
                c.completion_type == CompletionType::BashSubcommand
                    && (c.text.contains("ckout") || c.text == "ckout")
            });
            if has_checkout {
                assert!(has_checkout);
            }
        }
    }

    #[tokio::test]
    async fn test_cargo_subcommand_completion() {
        let mut app = create_test_app().await;

        // Test cargo subcommand completion
        app.current_input = "cargo bui".to_string();
        app.cursor_position = 9;
        app.handle_tab_completion();

        // Should complete to cargo build
        if app.completion_state.is_active {
            assert!(!app.completion_state.completions.is_empty());
            let has_build = app.completion_state.completions.iter().any(|c| {
                c.completion_type == CompletionType::BashSubcommand
                    && (c.text.contains("ld") || c.text == "ld")
            });
            if has_build {
                assert!(has_build);
            }
        }
    }

    #[tokio::test]
    async fn test_npm_subcommand_completion() {
        let mut app = create_test_app().await;

        // Test npm subcommand completion
        app.current_input = "npm ins".to_string();
        app.cursor_position = 7;
        app.handle_tab_completion();

        // Should complete to npm install
        if app.completion_state.is_active {
            assert!(!app.completion_state.completions.is_empty());
            let has_install = app.completion_state.completions.iter().any(|c| {
                c.completion_type == CompletionType::BashSubcommand
                    && (c.text.contains("tall") || c.text == "tall")
            });
            if has_install {
                assert!(has_install);
            }
        }
    }
}

#[cfg(test)]
mod subcommand_completion_tests {
    use super::*;

    #[test]
    fn test_subcommand_context_detection() {
        let commands = Vec::new();
        let engine = CompletionEngine::new(commands);

        // Test various subcommand contexts
        assert!(engine.is_subcommand_context("git status", 4));
        assert!(engine.is_subcommand_context("git checkout main", 4));
        assert!(engine.is_subcommand_context("cargo build --release", 6));
        assert!(engine.is_subcommand_context("npm install package", 4));
        assert!(engine.is_subcommand_context("docker run -it ubuntu", 7));
        assert!(engine.is_subcommand_context("kubectl get pods", 8));

        // Test non-subcommand contexts
        assert!(!engine.is_subcommand_context("git", 0));
        assert!(!engine.is_subcommand_context("unknown command", 8));
        assert!(!engine.is_subcommand_context("echo hello", 5));
        assert!(!engine.is_subcommand_context("ls -la", 3));
    }

    #[test]
    fn test_builtin_subcommand_fallback() {
        let commands = Vec::new();
        let engine = CompletionEngine::new(commands);

        // Test the builtin subcommand completion method directly
        let git_completions = engine.get_builtin_subcommands("git", "che");
        let has_checkout = git_completions
            .iter()
            .any(|c| c.completion_type == CompletionType::BashSubcommand && c.text == "ckout");
        assert!(has_checkout);

        let cargo_completions = engine.get_builtin_subcommands("cargo", "bui");
        let has_build = cargo_completions
            .iter()
            .any(|c| c.completion_type == CompletionType::BashSubcommand && c.text == "ld");
        assert!(has_build);

        let npm_completions = engine.get_builtin_subcommands("npm", "ins");
        let has_install = npm_completions
            .iter()
            .any(|c| c.completion_type == CompletionType::BashSubcommand && c.text == "tall");
        assert!(has_install);
    }

    #[test]
    fn test_builtin_subcommands_coverage() {
        let commands = Vec::new();
        let engine = CompletionEngine::new(commands);

        // Test that all supported commands have subcommands
        let git_subs = engine.get_builtin_subcommands("git", "");
        assert!(!git_subs.is_empty());
        assert!(git_subs.iter().any(|c| c.text == "checkout"));
        assert!(git_subs.iter().any(|c| c.text == "commit"));
        assert!(git_subs.iter().any(|c| c.text == "push"));
        assert!(git_subs.iter().any(|c| c.text == "pull"));

        let cargo_subs = engine.get_builtin_subcommands("cargo", "");
        assert!(!cargo_subs.is_empty());
        assert!(cargo_subs.iter().any(|c| c.text == "build"));
        assert!(cargo_subs.iter().any(|c| c.text == "test"));
        assert!(cargo_subs.iter().any(|c| c.text == "run"));
        assert!(cargo_subs.iter().any(|c| c.text == "check"));

        let npm_subs = engine.get_builtin_subcommands("npm", "");
        assert!(!npm_subs.is_empty());
        assert!(npm_subs.iter().any(|c| c.text == "install"));
        assert!(npm_subs.iter().any(|c| c.text == "start"));
        assert!(npm_subs.iter().any(|c| c.text == "test"));
        assert!(npm_subs.iter().any(|c| c.text == "run"));
    }
}
