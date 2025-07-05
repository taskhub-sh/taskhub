use std::collections::HashMap;
use taskhub::db::init_db;
use taskhub::db::models::{Priority, Task, TaskSource, TaskStatus};
use taskhub::tui::app::{App, AppMode};
use taskhub::tui::views::terminal::CommandEntry;
use uuid::Uuid;

// Helper function to create a test app
async fn create_test_app() -> App {
    let pool = init_db(Some(":memory:".into())).await.unwrap();
    App::new(pool)
}

#[tokio::test]
async fn test_app_initialization() {
    let app = create_test_app().await;

    assert_eq!(app.mode, AppMode::Terminal);
    assert!(!app.should_quit);
    assert_eq!(app.tasks.len(), 0);
    assert_eq!(app.command_history.len(), 0);
    assert_eq!(app.current_input, "");
    assert_eq!(app.cursor_position, 0);
    assert!(!app.show_command_list);
    assert_eq!(app.command_filter, "");
    assert_eq!(app.selected_command_index, 0);
    assert!(app.pending_command.is_none());
    assert!(app.pending_task_add.is_none());
    assert_eq!(app.scroll_offset, 0);

    // Check available commands
    let expected_commands = vec!["/quit", "/task", "/task add", "/task list", "/help"];
    assert_eq!(app.available_commands, expected_commands);
}

#[tokio::test]
async fn test_load_tasks() {
    let mut app = create_test_app().await;

    // Initially no tasks
    assert_eq!(app.tasks.len(), 0);

    // Load tasks (should be empty initially)
    app.load_tasks().await.unwrap();
    assert_eq!(app.tasks.len(), 0);

    // Add a task manually to database and reload
    let task = Task {
        id: Uuid::new_v4(),
        external_id: None,
        source: TaskSource::Markdown,
        title: "Test Task".to_string(),
        description: Some("Test Description".to_string()),
        status: TaskStatus::Open,
        priority: Priority::High,
        assignee: None,
        labels: vec!["test".to_string()],
        due_date: None,
        created_at: "2025-01-01T00:00:00Z".to_string(),
        updated_at: "2025-01-01T00:00:00Z".to_string(),
        custom_fields: HashMap::new(),
    };

    taskhub::db::operations::create_task(&app.db_pool, &task)
        .await
        .unwrap();
    app.load_tasks().await.unwrap();

    assert_eq!(app.tasks.len(), 1);
    assert_eq!(app.tasks[0].title, "Test Task");
}

#[tokio::test]
async fn test_mode_switching() {
    let mut app = create_test_app().await;

    // Start in Terminal mode
    assert_eq!(app.mode, AppMode::Terminal);

    // Switch to TaskList mode via /task command
    app.handle_builtin_command("/task").await;
    assert_eq!(app.mode, AppMode::TaskList);

    // Switch back via keyboard in TaskList mode
    app.on_key('q');
    assert_eq!(app.mode, AppMode::Terminal);
}

#[tokio::test]
async fn test_quit_functionality() {
    let mut app = create_test_app().await;

    assert!(!app.should_quit);

    app.handle_builtin_command("/quit").await;
    assert!(app.should_quit);
}

#[tokio::test]
async fn test_command_history() {
    let mut app = create_test_app().await;

    assert_eq!(app.command_history.len(), 0);

    // Test help command adds to history
    app.handle_builtin_command("/help").await;
    assert_eq!(app.command_history.len(), 1);

    let entry = &app.command_history[0];
    assert_eq!(entry.command, "/help");
    assert!(entry.output.contains("Available commands"));
    assert!(entry.success);

    // Test adding multiple entries
    let manual_entry = CommandEntry {
        command: "test".to_string(),
        output: "test output".to_string(),
        success: true,
    };
    app.command_history.push(manual_entry);

    assert_eq!(app.command_history.len(), 2);
}

#[tokio::test]
async fn test_get_total_history_lines() {
    let mut app = create_test_app().await;

    // No history initially
    assert_eq!(app.get_total_history_lines(), 0);

    // Add command with single line output
    let entry1 = CommandEntry {
        command: "cmd1".to_string(),
        output: "single line".to_string(),
        success: true,
    };
    app.command_history.push(entry1);

    // Should be 3 lines: command + output + spacing
    assert_eq!(app.get_total_history_lines(), 3);

    // Add command with multi-line output
    let entry2 = CommandEntry {
        command: "cmd2".to_string(),
        output: "line1\nline2\nline3".to_string(),
        success: false,
    };
    app.command_history.push(entry2);

    // Should be 3 + 5 = 8 lines: (cmd1 + output + spacing) + (cmd2 + 3 output lines + spacing)
    assert_eq!(app.get_total_history_lines(), 8);
}

#[cfg(test)]
mod task_operations {
    use super::*;

    #[tokio::test]
    async fn test_handle_task_add_command_valid() {
        let mut app = create_test_app().await;

        // Test valid task add command
        app.handle_task_add_command("/task add Test Task Title")
            .await;

        // Should set pending task add and switch to TaskList mode
        assert!(app.pending_task_add.is_some());
        assert_eq!(app.mode, AppMode::TaskList);

        let pending_task = app.pending_task_add.as_ref().unwrap();
        assert_eq!(pending_task.title, "Test Task Title");
        assert_eq!(pending_task.status, TaskStatus::Open);
        assert_eq!(pending_task.priority, Priority::Medium);
        assert_eq!(pending_task.source, TaskSource::Markdown);
    }

    #[tokio::test]
    async fn test_handle_task_add_command_invalid() {
        let mut app = create_test_app().await;

        // Test invalid task add command (no title)
        app.handle_task_add_command("/task add").await;

        // Should not set pending task add
        assert!(app.pending_task_add.is_none());

        // Should add error message to history
        assert_eq!(app.command_history.len(), 1);
        let entry = &app.command_history[0];
        assert_eq!(entry.command, "/task add");
        assert_eq!(entry.output, "Usage: /task add <title>");
        assert!(!entry.success);
    }

    #[tokio::test]
    async fn test_handle_pending_task_add() {
        let mut app = create_test_app().await;

        // Create a pending task
        let task = Task {
            id: Uuid::new_v4(),
            external_id: None,
            source: TaskSource::Markdown,
            title: "Pending Task".to_string(),
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

        app.pending_task_add = Some(task);

        // Process the pending task
        app.handle_pending_task_add().await;

        // Should clear pending task
        assert!(app.pending_task_add.is_none());

        // Should add success message to history
        assert_eq!(app.command_history.len(), 1);
        let entry = &app.command_history[0];
        assert!(entry.output.contains("added successfully"));
        assert!(entry.success);

        // Should load the task
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].title, "Pending Task");
    }
}
