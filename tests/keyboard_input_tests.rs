use crossterm::event::{KeyCode, KeyModifiers};
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

        app.on_key_code(KeyCode::Esc, KeyModifiers::NONE);

        assert!(!app.show_command_list);
        assert_eq!(app.command_filter, "");
        assert_eq!(app.selected_command_index, 0);
    }

    #[tokio::test]
    async fn test_on_key_code_enter_with_empty_input() {
        let mut app = create_test_app().await;
        app.current_input = "".to_string();

        app.on_key_code(KeyCode::Enter, KeyModifiers::NONE);

        // Should not set pending command
        assert!(app.pending_command.is_none());
    }

    #[tokio::test]
    async fn test_on_key_code_enter_with_command() {
        let mut app = create_test_app().await;
        app.current_input = "/help".to_string();

        app.on_key_code(KeyCode::Enter, KeyModifiers::NONE);

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

        app.on_key_code(KeyCode::Enter, KeyModifiers::NONE);

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

        app.on_key_code(KeyCode::Enter, KeyModifiers::NONE);

        // Since "/ta" doesn't match any complete command patterns, it should try to select from list
        // The exact behavior depends on get_filtered_commands implementation
    }

    #[tokio::test]
    async fn test_on_key_code_backspace() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 5;

        app.on_key_code(KeyCode::Backspace, KeyModifiers::NONE);

        assert_eq!(app.current_input, "hell");
        assert_eq!(app.cursor_position, 4);
    }

    #[tokio::test]
    async fn test_on_key_code_backspace_at_beginning() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 0;

        app.on_key_code(KeyCode::Backspace, KeyModifiers::NONE);

        // Should not change anything
        assert_eq!(app.current_input, "hello");
        assert_eq!(app.cursor_position, 0);
    }

    #[tokio::test]
    async fn test_on_key_code_delete() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 2; // Between 'e' and 'l'

        app.on_key_code(KeyCode::Delete, KeyModifiers::NONE);

        assert_eq!(app.current_input, "helo");
        assert_eq!(app.cursor_position, 2);
    }

    #[tokio::test]
    async fn test_on_key_code_delete_at_end() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 5; // At end

        app.on_key_code(KeyCode::Delete, KeyModifiers::NONE);

        // Should not change anything
        assert_eq!(app.current_input, "hello");
        assert_eq!(app.cursor_position, 5);
    }

    #[tokio::test]
    async fn test_on_key_code_left_arrow() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 3;

        app.on_key_code(KeyCode::Left, KeyModifiers::NONE);

        assert_eq!(app.cursor_position, 2);
    }

    #[tokio::test]
    async fn test_on_key_code_left_arrow_at_beginning() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 0;

        app.on_key_code(KeyCode::Left, KeyModifiers::NONE);

        // Should not move beyond beginning
        assert_eq!(app.cursor_position, 0);
    }

    #[tokio::test]
    async fn test_on_key_code_right_arrow() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 2;

        app.on_key_code(KeyCode::Right, KeyModifiers::NONE);

        assert_eq!(app.cursor_position, 3);
    }

    #[tokio::test]
    async fn test_on_key_code_right_arrow_at_end() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 5;

        app.on_key_code(KeyCode::Right, KeyModifiers::NONE);

        // Should not move beyond end
        assert_eq!(app.cursor_position, 5);
    }

    #[tokio::test]
    async fn test_on_key_code_up_arrow_with_command_list() {
        let mut app = create_test_app().await;
        app.show_command_list = true;
        app.selected_command_index = 2;

        app.on_key_code(KeyCode::Up, KeyModifiers::NONE);

        assert_eq!(app.selected_command_index, 1);
    }

    #[tokio::test]
    async fn test_on_key_code_up_arrow_at_top_of_command_list() {
        let mut app = create_test_app().await;
        app.show_command_list = true;
        app.selected_command_index = 0;

        app.on_key_code(KeyCode::Up, KeyModifiers::NONE);

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
        app.on_key_code(KeyCode::Down, KeyModifiers::NONE);

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

        app.on_key_code(KeyCode::PageUp, KeyModifiers::NONE);

        // Should increase scroll offset by 10 (or to max)
        assert!(app.scroll_offset >= 5);
    }

    #[tokio::test]
    async fn test_on_key_code_page_down() {
        let mut app = create_test_app().await;
        app.scroll_offset = 15;

        app.on_key_code(KeyCode::PageDown, KeyModifiers::NONE);

        // Should decrease scroll offset by 10
        assert_eq!(app.scroll_offset, 5);
    }

    #[tokio::test]
    async fn test_on_key_code_home_with_empty_input() {
        let mut app = create_test_app().await;
        app.current_input = "".to_string();
        app.scroll_offset = 5;

        app.on_key_code(KeyCode::Home, KeyModifiers::NONE);

        // Should go to top of history
        let expected_offset = app.get_total_history_lines().saturating_sub(1);
        assert_eq!(app.scroll_offset, expected_offset);
    }

    #[tokio::test]
    async fn test_on_key_code_home_with_input() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 3;

        app.on_key_code(KeyCode::Home, KeyModifiers::NONE);

        // Should move cursor to beginning
        assert_eq!(app.cursor_position, 0);
    }

    #[tokio::test]
    async fn test_on_key_code_end_with_empty_input() {
        let mut app = create_test_app().await;
        app.current_input = "".to_string();
        app.scroll_offset = 10;

        app.on_key_code(KeyCode::End, KeyModifiers::NONE);

        // Should go to bottom of history
        assert_eq!(app.scroll_offset, 0);
    }

    #[tokio::test]
    async fn test_on_key_code_end_with_input() {
        let mut app = create_test_app().await;
        app.current_input = "hello".to_string();
        app.cursor_position = 2;

        app.on_key_code(KeyCode::End, KeyModifiers::NONE);

        // Should move cursor to end
        assert_eq!(app.cursor_position, 5);
    }

    #[tokio::test]
    async fn test_command_history_navigation_up_arrow() {
        let mut app = create_test_app().await;

        // Add some command history
        app.command_history
            .push(taskhub::tui::views::terminal::CommandEntry {
                command: "ls".to_string(),
                output: "file1.txt".to_string(),
                timestamp: "12:00:00".to_string(),
                success: true,
            });
        app.command_history
            .push(taskhub::tui::views::terminal::CommandEntry {
                command: "pwd".to_string(),
                output: "/home/user".to_string(),
                timestamp: "12:00:01".to_string(),
                success: true,
            });
        app.command_history
            .push(taskhub::tui::views::terminal::CommandEntry {
                command: "echo hello".to_string(),
                output: "hello".to_string(),
                timestamp: "12:00:02".to_string(),
                success: true,
            });

        // Start with empty input
        app.current_input = "".to_string();
        app.cursor_position = 0;

        // Press Up arrow - should get most recent command
        app.on_key_code(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.current_input, "echo hello");
        assert_eq!(app.cursor_position, 10);
        assert_eq!(app.history_index, Some(2));
        assert_eq!(app.saved_input, "");

        // Press Up arrow again - should get previous command
        app.on_key_code(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.current_input, "pwd");
        assert_eq!(app.cursor_position, 3);
        assert_eq!(app.history_index, Some(1));

        // Press Up arrow again - should get oldest command
        app.on_key_code(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.current_input, "ls");
        assert_eq!(app.cursor_position, 2);
        assert_eq!(app.history_index, Some(0));

        // Press Up arrow again - should stay at oldest
        app.on_key_code(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.current_input, "ls");
        assert_eq!(app.cursor_position, 2);
        assert_eq!(app.history_index, Some(0));
    }

    #[tokio::test]
    async fn test_command_history_navigation_down_arrow() {
        let mut app = create_test_app().await;

        // Add some command history
        app.command_history
            .push(taskhub::tui::views::terminal::CommandEntry {
                command: "ls".to_string(),
                output: "file1.txt".to_string(),
                timestamp: "12:00:00".to_string(),
                success: true,
            });
        app.command_history
            .push(taskhub::tui::views::terminal::CommandEntry {
                command: "pwd".to_string(),
                output: "/home/user".to_string(),
                timestamp: "12:00:01".to_string(),
                success: true,
            });

        // Start navigation from oldest command
        app.history_index = Some(0);
        app.current_input = "ls".to_string();
        app.saved_input = "partial".to_string();

        // Press Down arrow - should get newer command
        app.on_key_code(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(app.current_input, "pwd");
        assert_eq!(app.cursor_position, 3);
        assert_eq!(app.history_index, Some(1));

        // Press Down arrow again - should restore saved input
        app.on_key_code(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(app.current_input, "partial");
        assert_eq!(app.cursor_position, 7);
        assert_eq!(app.history_index, None);

        // Press Down arrow again - should do nothing (no navigation active)
        app.on_key_code(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(app.current_input, "partial");
        assert_eq!(app.history_index, None);
    }

    #[tokio::test]
    async fn test_command_history_navigation_with_partial_input() {
        let mut app = create_test_app().await;

        // Add command history
        app.command_history
            .push(taskhub::tui::views::terminal::CommandEntry {
                command: "ls -la".to_string(),
                output: "files".to_string(),
                timestamp: "12:00:00".to_string(),
                success: true,
            });

        // Start with partial input
        app.current_input = "partial command".to_string();
        app.cursor_position = 15;

        // Press Up arrow - should save partial input and show history
        app.on_key_code(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.current_input, "ls -la");
        assert_eq!(app.cursor_position, 6);
        assert_eq!(app.history_index, Some(0));
        assert_eq!(app.saved_input, "partial command");

        // Press Down arrow - should restore partial input
        app.on_key_code(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(app.current_input, "partial command");
        assert_eq!(app.cursor_position, 15);
        assert_eq!(app.history_index, None);
    }

    #[tokio::test]
    async fn test_command_history_navigation_reset_on_typing() {
        let mut app = create_test_app().await;

        // Add command history
        app.command_history
            .push(taskhub::tui::views::terminal::CommandEntry {
                command: "ls".to_string(),
                output: "files".to_string(),
                timestamp: "12:00:00".to_string(),
                success: true,
            });

        // Start history navigation
        app.on_key_code(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.current_input, "ls");
        assert_eq!(app.history_index, Some(0));

        // Type a character - should reset history navigation
        app.on_key('a');
        assert_eq!(app.current_input, "lsa");
        assert_eq!(app.history_index, None);
        assert_eq!(app.saved_input, "");
    }

    #[tokio::test]
    async fn test_command_history_navigation_reset_on_backspace() {
        let mut app = create_test_app().await;

        // Add command history
        app.command_history
            .push(taskhub::tui::views::terminal::CommandEntry {
                command: "ls".to_string(),
                output: "files".to_string(),
                timestamp: "12:00:00".to_string(),
                success: true,
            });

        // Start history navigation
        app.on_key_code(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.current_input, "ls");
        assert_eq!(app.history_index, Some(0));

        // Press backspace - should reset history navigation
        app.on_key_code(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(app.current_input, "l");
        assert_eq!(app.history_index, None);
        assert_eq!(app.saved_input, "");
    }

    #[tokio::test]
    async fn test_command_history_navigation_reset_on_delete() {
        let mut app = create_test_app().await;

        // Add command history
        app.command_history
            .push(taskhub::tui::views::terminal::CommandEntry {
                command: "ls".to_string(),
                output: "files".to_string(),
                timestamp: "12:00:00".to_string(),
                success: true,
            });

        // Start history navigation
        app.on_key_code(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.current_input, "ls");
        assert_eq!(app.history_index, Some(0));
        app.cursor_position = 1; // Position between 'l' and 's'

        // Press delete - should reset history navigation
        app.on_key_code(KeyCode::Delete, KeyModifiers::NONE);
        assert_eq!(app.current_input, "l");
        assert_eq!(app.history_index, None);
        assert_eq!(app.saved_input, "");
    }

    #[tokio::test]
    async fn test_command_history_navigation_reset_on_enter() {
        let mut app = create_test_app().await;

        // Add command history
        app.command_history
            .push(taskhub::tui::views::terminal::CommandEntry {
                command: "ls".to_string(),
                output: "files".to_string(),
                timestamp: "12:00:00".to_string(),
                success: true,
            });

        // Start history navigation
        app.on_key_code(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.current_input, "ls");
        assert_eq!(app.history_index, Some(0));

        // Press enter - should reset history navigation and execute command
        app.on_key_code(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.current_input, "");
        assert_eq!(app.history_index, None);
        assert_eq!(app.saved_input, "");
        assert_eq!(app.pending_command, Some("ls".to_string()));
    }

    #[tokio::test]
    async fn test_command_history_navigation_empty_history() {
        let mut app = create_test_app().await;

        // No command history
        assert!(app.command_history.is_empty());

        // Press Up arrow - should do nothing
        app.on_key_code(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.current_input, "");
        assert_eq!(app.history_index, None);
        assert_eq!(app.saved_input, "");

        // Press Down arrow - should do nothing
        app.on_key_code(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(app.current_input, "");
        assert_eq!(app.history_index, None);
        assert_eq!(app.saved_input, "");
    }

    #[tokio::test]
    async fn test_shift_up_arrow_scrolls_up() {
        let mut app = create_test_app().await;

        // Add some command history to make scrolling possible
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

        app.scroll_offset = 0;

        // Press Shift+Up - should scroll up
        app.on_key_code(KeyCode::Up, KeyModifiers::SHIFT);
        assert!(app.scroll_offset > 0);

        // Current input should remain unchanged
        assert_eq!(app.current_input, "");
        assert_eq!(app.history_index, None);
    }

    #[tokio::test]
    async fn test_shift_down_arrow_scrolls_down() {
        let mut app = create_test_app().await;

        // Start with some scroll offset
        app.scroll_offset = 10;

        // Press Shift+Down - should scroll down
        app.on_key_code(KeyCode::Down, KeyModifiers::SHIFT);
        assert_eq!(app.scroll_offset, 9);

        // Current input should remain unchanged
        assert_eq!(app.current_input, "");
        assert_eq!(app.history_index, None);
    }

    #[tokio::test]
    async fn test_shift_down_arrow_at_bottom() {
        let mut app = create_test_app().await;

        // Start at bottom (scroll_offset = 0)
        app.scroll_offset = 0;

        // Press Shift+Down - should stay at bottom
        app.on_key_code(KeyCode::Down, KeyModifiers::SHIFT);
        assert_eq!(app.scroll_offset, 0);
    }

    #[tokio::test]
    async fn test_history_navigation_with_command_list_active() {
        let mut app = create_test_app().await;

        // Add command history
        app.command_history
            .push(taskhub::tui::views::terminal::CommandEntry {
                command: "ls".to_string(),
                output: "files".to_string(),
                timestamp: "12:00:00".to_string(),
                success: true,
            });

        // Activate command list
        app.show_command_list = true;
        app.selected_command_index = 1;

        // Press Up arrow - should navigate command list, not history
        app.on_key_code(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.selected_command_index, 0);
        assert_eq!(app.current_input, ""); // History navigation should not activate
        assert_eq!(app.history_index, None);

        // Press Down arrow - should navigate command list
        app.on_key_code(KeyCode::Down, KeyModifiers::NONE);
        // The exact behavior depends on get_filtered_commands, but history should not activate
        assert_eq!(app.history_index, None);
    }
}
