use crossterm::event::KeyCode;
use taskhub::db::init_db;
use taskhub::tui::app::{App, AppMode};

// Helper function to create a test app
async fn create_test_app() -> App {
    let pool = init_db(Some(":memory:".into())).await.unwrap();
    App::new(pool)
}

#[cfg(test)]
mod key_handling {
    use super::*;

    #[tokio::test]
    async fn test_on_key_task_list_mode_q() {
        let mut app = create_test_app().await;
        app.mode = AppMode::TaskList;

        app.on_key('q');

        assert_eq!(app.mode, AppMode::Terminal);
    }

    #[tokio::test]
    async fn test_on_key_task_list_mode_t() {
        let mut app = create_test_app().await;
        app.mode = AppMode::TaskList;

        app.on_key('t');

        assert_eq!(app.mode, AppMode::Terminal);
    }

    #[tokio::test]
    async fn test_on_key_terminal_mode_regular_char() {
        let mut app = create_test_app().await;
        app.mode = AppMode::Terminal;

        app.on_key('a');

        assert_eq!(app.current_input, "a");
        assert_eq!(app.cursor_position, 1);
    }

    #[tokio::test]
    async fn test_on_key_code_escape_clears_command_list() {
        let mut app = create_test_app().await;

        // Set up command list state
        app.show_command_list = true;
        app.command_filter = "test".to_string();
        app.selected_command_index = 2;

        app.on_key_code(KeyCode::Esc);

        assert!(!app.show_command_list);
        assert_eq!(app.command_filter, "");
        assert_eq!(app.selected_command_index, 0);
    }

    #[tokio::test]
    async fn test_on_key_code_enter_with_empty_input() {
        let mut app = create_test_app().await;
        app.current_input = "".to_string();

        app.on_key_code(KeyCode::Enter);

        // Should not set pending command
        assert!(app.pending_command.is_none());
    }

    #[tokio::test]
    async fn test_on_key_code_enter_with_command() {
        let mut app = create_test_app().await;
        app.current_input = "/help".to_string();

        app.on_key_code(KeyCode::Enter);

        // Should set pending command and clear input
        assert_eq!(app.pending_command, Some("/help".to_string()));
        assert_eq!(app.current_input, "");
        assert_eq!(app.cursor_position, 0);
        assert_eq!(app.scroll_offset, 0);
    }

    #[tokio::test]
    async fn test_on_key_code_enter_with_command_list_complete_command() {
        let mut app = create_test_app().await;
        app.current_input = "/task add hello".to_string();
        app.show_command_list = true;

        app.on_key_code(KeyCode::Enter);

        // Should execute command directly since it's complete
        assert_eq!(app.pending_command, Some("/task add hello".to_string()));
        assert_eq!(app.current_input, "");
        assert!(!app.show_command_list);
        assert_eq!(app.command_filter, "");
        assert_eq!(app.selected_command_index, 0);
    }

    #[tokio::test]
    async fn test_on_key_code_enter_with_command_list_incomplete_command() {
        let mut app = create_test_app().await;
        app.current_input = "/ta".to_string();
        app.show_command_list = true;
        app.selected_command_index = 0;

        // Mock filtered commands (this would normally be populated by get_filtered_commands)
        let _mock_filtered = vec!["/task".to_string()];

        // Since we can't easily mock get_filtered_commands, we'll test the behavior differently
        // by setting up a state where the command would be selected
        let _initial_input = app.current_input.clone();

        app.on_key_code(KeyCode::Enter);

        // Since "/ta" doesn't match any complete command patterns, it should try to select from list
        // The exact behavior depends on get_filtered_commands implementation
    }

    #[tokio::test]
    async fn test_on_key_code_backspace() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 5;

        app.on_key_code(KeyCode::Backspace);

        assert_eq!(app.current_input, "hell");
        assert_eq!(app.cursor_position, 4);
    }

    #[tokio::test]
    async fn test_on_key_code_backspace_at_beginning() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 0;

        app.on_key_code(KeyCode::Backspace);

        // Should not change anything
        assert_eq!(app.current_input, "hello");
        assert_eq!(app.cursor_position, 0);
    }

    #[tokio::test]
    async fn test_on_key_code_delete() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 2; // Between 'e' and 'l'

        app.on_key_code(KeyCode::Delete);

        assert_eq!(app.current_input, "helo");
        assert_eq!(app.cursor_position, 2);
    }

    #[tokio::test]
    async fn test_on_key_code_delete_at_end() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 5; // At end

        app.on_key_code(KeyCode::Delete);

        // Should not change anything
        assert_eq!(app.current_input, "hello");
        assert_eq!(app.cursor_position, 5);
    }

    #[tokio::test]
    async fn test_on_key_code_left_arrow() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 3;

        app.on_key_code(KeyCode::Left);

        assert_eq!(app.cursor_position, 2);
    }

    #[tokio::test]
    async fn test_on_key_code_left_arrow_at_beginning() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 0;

        app.on_key_code(KeyCode::Left);

        // Should not move beyond beginning
        assert_eq!(app.cursor_position, 0);
    }

    #[tokio::test]
    async fn test_on_key_code_right_arrow() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 2;

        app.on_key_code(KeyCode::Right);

        assert_eq!(app.cursor_position, 3);
    }

    #[tokio::test]
    async fn test_on_key_code_right_arrow_at_end() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 5;

        app.on_key_code(KeyCode::Right);

        // Should not move beyond end
        assert_eq!(app.cursor_position, 5);
    }

    #[tokio::test]
    async fn test_on_key_code_up_arrow_with_command_list() {
        let mut app = create_test_app().await;
        app.show_command_list = true;
        app.selected_command_index = 2;

        app.on_key_code(KeyCode::Up);

        assert_eq!(app.selected_command_index, 1);
    }

    #[tokio::test]
    async fn test_on_key_code_up_arrow_at_top_of_command_list() {
        let mut app = create_test_app().await;
        app.show_command_list = true;
        app.selected_command_index = 0;

        app.on_key_code(KeyCode::Up);

        // Should not move beyond top
        assert_eq!(app.selected_command_index, 0);
    }

    #[tokio::test]
    async fn test_on_key_code_down_arrow_with_command_list() {
        let mut app = create_test_app().await;
        app.show_command_list = true;
        app.selected_command_index = 0;

        // Note: The actual behavior depends on get_filtered_commands
        // This test verifies the method doesn't crash
        app.on_key_code(KeyCode::Down);

        // The exact result depends on the filtered commands length
        // We mainly test that it doesn't panic
    }

    #[tokio::test]
    async fn test_on_key_code_page_up() {
        let mut app = create_test_app().await;
        app.scroll_offset = 5;

        // Add some history so there's something to scroll through
        app.command_history
            .push(taskhub::tui::views::terminal::CommandEntry {
                command: "test1".to_string(),
                output: "output1".to_string(),
                timestamp: "12:00:00".to_string(),
                success: true,
            });
        app.command_history
            .push(taskhub::tui::views::terminal::CommandEntry {
                command: "test2".to_string(),
                output: "output2".to_string(),
                timestamp: "12:00:01".to_string(),
                success: true,
            });

        app.on_key_code(KeyCode::PageUp);

        // Should increase scroll offset by 10 (or to max)
        assert!(app.scroll_offset >= 5);
    }

    #[tokio::test]
    async fn test_on_key_code_page_down() {
        let mut app = create_test_app().await;
        app.scroll_offset = 15;

        app.on_key_code(KeyCode::PageDown);

        // Should decrease scroll offset by 10
        assert_eq!(app.scroll_offset, 5);
    }

    #[tokio::test]
    async fn test_on_key_code_home_with_empty_input() {
        let mut app = create_test_app().await;
        app.current_input = "".to_string();
        app.scroll_offset = 5;

        app.on_key_code(KeyCode::Home);

        // Should go to top of history
        let expected_offset = app.get_total_history_lines().saturating_sub(1);
        assert_eq!(app.scroll_offset, expected_offset);
    }

    #[tokio::test]
    async fn test_on_key_code_home_with_input() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 3;

        app.on_key_code(KeyCode::Home);

        // Should move cursor to beginning
        assert_eq!(app.cursor_position, 0);
    }

    #[tokio::test]
    async fn test_on_key_code_end_with_empty_input() {
        let mut app = create_test_app().await;
        app.current_input = "".to_string();
        app.scroll_offset = 10;

        app.on_key_code(KeyCode::End);

        // Should go to bottom of history
        assert_eq!(app.scroll_offset, 0);
    }

    #[tokio::test]
    async fn test_on_key_code_end_with_input() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 2;

        app.on_key_code(KeyCode::End);

        // Should move cursor to end
        assert_eq!(app.cursor_position, 5);
    }
}
