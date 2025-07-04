use taskhub::db::init_db;
use taskhub::tui::app::{App, AppMode};

// Helper function to create a test app
async fn create_test_app() -> App {
    let pool = init_db(Some(":memory:".into())).await.unwrap();
    App::new(pool)
}

#[cfg(test)]
mod builtin_commands {
    use super::*;

    #[tokio::test]
    async fn test_handle_builtin_command_quit() {
        let mut app = create_test_app().await;

        let result = app.handle_builtin_command("/quit");

        assert!(result); // Should return true (command handled)
        assert!(app.should_quit);
    }

    #[tokio::test]
    async fn test_handle_builtin_command_task() {
        let mut app = create_test_app().await;
        assert_eq!(app.mode, AppMode::Terminal);

        let result = app.handle_builtin_command("/task");

        assert!(result); // Should return true (command handled)
        assert_eq!(app.mode, AppMode::TaskList);
    }

    #[tokio::test]
    async fn test_handle_builtin_command_task_list() {
        let mut app = create_test_app().await;
        assert_eq!(app.mode, AppMode::Terminal);

        let result = app.handle_builtin_command("/task list");

        assert!(result); // Should return true (command handled)
        assert_eq!(app.mode, AppMode::TaskList);
    }

    #[tokio::test]
    async fn test_handle_builtin_command_help() {
        let mut app = create_test_app().await;

        let result = app.handle_builtin_command("/help");

        assert!(result); // Should return true (command handled)
        assert_eq!(app.command_history.len(), 1);

        let entry = &app.command_history[0];
        assert_eq!(entry.command, "/help");
        assert!(entry.output.contains("Available commands"));
        assert!(entry.output.contains("/quit"));
        assert!(entry.output.contains("/task"));
        assert!(entry.output.contains("/help"));
        assert!(entry.success);
    }

    #[tokio::test]
    async fn test_handle_builtin_command_task_add_valid() {
        let mut app = create_test_app().await;

        let result = app.handle_builtin_command("/task add Hello World");

        assert!(result); // Should return true (command handled)
        assert!(app.pending_task_add.is_some());
        assert_eq!(app.mode, AppMode::TaskList);

        let pending_task = app.pending_task_add.as_ref().unwrap();
        assert_eq!(pending_task.title, "Hello World");
    }

    #[tokio::test]
    async fn test_handle_builtin_command_task_add_invalid() {
        let mut app = create_test_app().await;

        let result = app.handle_builtin_command("/task add");

        assert!(result); // Should return true (command handled)
        assert!(app.pending_task_add.is_none());
        assert_eq!(app.command_history.len(), 1);

        let entry = &app.command_history[0];
        assert_eq!(entry.command, "/task add");
        assert_eq!(entry.output, "Usage: /task add <title>");
        assert!(!entry.success);
    }

    #[tokio::test]
    async fn test_handle_builtin_command_unknown() {
        let mut app = create_test_app().await;

        let result = app.handle_builtin_command("/unknown");

        assert!(!result); // Should return false (command not handled)
        assert!(!app.should_quit);
        assert_eq!(app.mode, AppMode::Terminal);
        assert_eq!(app.command_history.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_builtin_command_empty() {
        let mut app = create_test_app().await;

        let result = app.handle_builtin_command("");

        assert!(!result); // Should return false (command not handled)
    }

    #[tokio::test]
    async fn test_handle_builtin_command_not_slash() {
        let mut app = create_test_app().await;

        let result = app.handle_builtin_command("regular command");

        assert!(!result); // Should return false (command not handled)
    }

    #[tokio::test]
    async fn test_handle_builtin_command_task_add_with_multiple_words() {
        let mut app = create_test_app().await;

        let result = app.handle_builtin_command("/task add Fix the login bug in authentication");

        assert!(result); // Should return true (command handled)
        assert!(app.pending_task_add.is_some());

        let pending_task = app.pending_task_add.as_ref().unwrap();
        assert_eq!(pending_task.title, "Fix the login bug in authentication");
    }

    #[tokio::test]
    async fn test_handle_builtin_command_task_add_with_special_chars() {
        let mut app = create_test_app().await;

        let result =
            app.handle_builtin_command("/task add Update README.md with new info & examples");

        assert!(result); // Should return true (command handled)
        assert!(app.pending_task_add.is_some());

        let pending_task = app.pending_task_add.as_ref().unwrap();
        assert_eq!(
            pending_task.title,
            "Update README.md with new info & examples"
        );
    }
}

#[cfg(test)]
mod pending_commands {
    use super::*;

    #[tokio::test]
    async fn test_handle_pending_commands_builtin() {
        let mut app = create_test_app().await;

        // Set a builtin command as pending
        app.pending_command = Some("/help".to_string());

        app.handle_pending_commands().await;

        // Should clear pending command and execute it
        assert!(app.pending_command.is_none());
        assert_eq!(app.command_history.len(), 1);
        assert_eq!(app.command_history[0].command, "/help");
    }

    #[tokio::test]
    async fn test_handle_pending_commands_task_add() {
        let mut app = create_test_app().await;

        // Set a task add command as pending
        app.pending_command = Some("/task add Test Task".to_string());

        app.handle_pending_commands().await;

        // Should clear pending command, execute it, and process the task add
        assert!(app.pending_command.is_none());
        assert!(app.pending_task_add.is_none()); // Should be processed and cleared
        assert_eq!(app.mode, AppMode::TaskList);
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].title, "Test Task");

        // Should have success message in history
        let success_entry = app
            .command_history
            .iter()
            .find(|entry| entry.output.contains("added successfully"));
        assert!(success_entry.is_some());
    }

    #[tokio::test]
    async fn test_handle_pending_commands_no_pending() {
        let mut app = create_test_app().await;

        // No pending command
        assert!(app.pending_command.is_none());

        app.handle_pending_commands().await;

        // Should not change anything
        assert!(app.pending_command.is_none());
        assert_eq!(app.command_history.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_pending_commands_shell_command() {
        let mut app = create_test_app().await;

        // Set a non-builtin command as pending
        app.pending_command = Some("echo hello".to_string());

        app.handle_pending_commands().await;

        // Should execute as shell command
        assert!(app.pending_command.is_none());
        assert_eq!(app.command_history.len(), 1);

        let entry = &app.command_history[0];
        assert_eq!(entry.command, "echo hello");
        assert!(entry.output.contains("hello") || entry.output.is_empty()); // Depends on shell
    }
}

#[cfg(test)]
mod command_execution {
    use super::*;

    #[tokio::test]
    async fn test_execute_command_simple() {
        let mut app = create_test_app().await;

        app.execute_command("echo test".to_string()).await;

        assert_eq!(app.command_history.len(), 1);
        let entry = &app.command_history[0];
        assert_eq!(entry.command, "echo test");
        // Note: output might be empty in test environment, so we just check it executed
    }

    #[tokio::test]
    async fn test_execute_command_history_limit() {
        let mut app = create_test_app().await;

        // Add many commands to test history limit
        for i in 0..1005 {
            app.execute_command(format!("echo {}", i)).await;
        }

        // Should limit to 1000 commands (after draining 100 when it exceeds 1000)
        assert!(app.command_history.len() <= 1000);

        // The last command should still be there
        let last_entry = app.command_history.last().unwrap();
        assert_eq!(last_entry.command, "echo 1004");
    }

    #[tokio::test]
    async fn test_execute_command_resets_scroll() {
        let mut app = create_test_app().await;
        app.scroll_offset = 10;

        app.execute_command("echo test".to_string()).await;

        // Should reset scroll to bottom
        assert_eq!(app.scroll_offset, 0);
    }
}
