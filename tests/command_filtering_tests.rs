use taskhub::db::init_db;
use taskhub::tui::app::App;

// Helper function to create a test app
async fn create_test_app() -> App {
    let pool = init_db(Some(":memory:".into())).await.unwrap();
    App::new(pool)
}

#[cfg(test)]
mod command_filtering {
    use super::*;

    #[tokio::test]
    async fn test_get_filtered_commands_empty_filter() {
        let app = create_test_app().await;

        // With empty filter, should only show top-level commands (no spaces)
        let filtered = app.get_filtered_commands();

        // Should exclude "/task add", "/task list", and "/help keys" (they contain spaces)
        let expected = vec!["/quit", "/task", "/help", "/clear"];
        assert_eq!(filtered, expected);
    }

    #[tokio::test]
    async fn test_get_filtered_commands_task_filter() {
        let mut app = create_test_app().await;

        // Set filter to "task"
        app.command_filter = "task".to_string();

        let filtered = app.get_filtered_commands();

        // Should include all task-related commands
        let expected = vec!["/task", "/task add", "/task list"];
        assert_eq!(filtered, expected);
    }

    #[tokio::test]
    async fn test_get_filtered_commands_task_add_filter() {
        let mut app = create_test_app().await;

        // Set filter to "task add"
        app.command_filter = "task add".to_string();

        let filtered = app.get_filtered_commands();

        // Should only include "/task add"
        let expected = vec!["/task add"];
        assert_eq!(filtered, expected);
    }

    #[tokio::test]
    async fn test_get_filtered_commands_quit_filter() {
        let mut app = create_test_app().await;

        // Set filter to "quit"
        app.command_filter = "quit".to_string();

        let filtered = app.get_filtered_commands();

        // Should only include "/quit"
        let expected = vec!["/quit"];
        assert_eq!(filtered, expected);
    }

    #[tokio::test]
    async fn test_get_filtered_commands_no_match() {
        let mut app = create_test_app().await;

        // Set filter to something that doesn't match
        app.command_filter = "nonexistent".to_string();

        let filtered = app.get_filtered_commands();

        // Should be empty
        assert!(filtered.is_empty());
    }

    #[tokio::test]
    async fn test_update_command_filtering_with_slash() {
        let mut app = create_test_app().await;

        // Set input that starts with slash
        app.current_input = "/task".to_string();

        app.update_command_filtering();

        assert!(app.show_command_list);
        assert_eq!(app.command_filter, "task");
        assert_eq!(app.selected_command_index, 0);
    }

    #[tokio::test]
    async fn test_update_command_filtering_without_slash() {
        let mut app = create_test_app().await;

        // Set input that doesn't start with slash
        app.current_input = "regular command".to_string();

        app.update_command_filtering();

        assert!(!app.show_command_list);
        assert_eq!(app.command_filter, "");
    }

    #[tokio::test]
    async fn test_update_command_filtering_empty_input() {
        let mut app = create_test_app().await;

        // Set empty input
        app.current_input = "".to_string();

        app.update_command_filtering();

        assert!(!app.show_command_list);
        assert_eq!(app.command_filter, "");
    }

    #[tokio::test]
    async fn test_update_command_filtering_slash_only() {
        let mut app = create_test_app().await;

        // Set input with just slash
        app.current_input = "/".to_string();

        app.update_command_filtering();

        assert!(app.show_command_list);
        assert_eq!(app.command_filter, "");
        assert_eq!(app.selected_command_index, 0);
    }
}

#[cfg(test)]
mod terminal_input {
    use super::*;

    #[tokio::test]
    async fn test_handle_terminal_input_regular_char() {
        let mut app = create_test_app().await;

        app.handle_terminal_input('a');

        assert_eq!(app.current_input, "a");
        assert_eq!(app.cursor_position, 1);
        assert!(!app.show_command_list);
    }

    #[tokio::test]
    async fn test_handle_terminal_input_slash_triggers_command_list() {
        let mut app = create_test_app().await;

        app.handle_terminal_input('/');

        assert_eq!(app.current_input, "/");
        assert_eq!(app.cursor_position, 1);
        assert!(app.show_command_list);
        assert_eq!(app.command_filter, "");
    }

    #[tokio::test]
    async fn test_handle_terminal_input_command_building() {
        let mut app = create_test_app().await;

        // Type "/task" character by character
        for ch in "/task".chars() {
            app.handle_terminal_input(ch);
        }

        assert_eq!(app.current_input, "/task");
        assert_eq!(app.cursor_position, 5);
        assert!(app.show_command_list);
        assert_eq!(app.command_filter, "task");
    }

    #[tokio::test]
    async fn test_handle_terminal_input_control_char_ignored() {
        let mut app = create_test_app().await;

        // Try to input a control character
        app.handle_terminal_input('\x01'); // Ctrl+A

        assert_eq!(app.current_input, "");
        assert_eq!(app.cursor_position, 0);
    }

    #[tokio::test]
    async fn test_handle_terminal_input_cursor_insertion() {
        let mut app = create_test_app().await;

        // First type "hello"
        for ch in "hello".chars() {
            app.handle_terminal_input(ch);
        }

        // Move cursor to position 2 (between 'e' and 'l')
        app.cursor_position = 2;

        // Insert 'X'
        app.handle_terminal_input('X');

        assert_eq!(app.current_input, "heXllo");
        assert_eq!(app.cursor_position, 3);
    }
}
