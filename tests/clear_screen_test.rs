use taskhub::tui::app::App;
use taskhub::tui::views::terminal::CommandEntry;

#[tokio::test]
async fn test_clear_screen_functionality() {
    // Create an in-memory database for testing
    let pool = taskhub::db::init_db(Some(":memory:".into())).await.unwrap();

    let mut app = App::new(pool);

    // Add some command history and set various states
    app.command_history.push(CommandEntry {
        command: "ls".to_string(),
        output: "file1.txt\nfile2.txt".to_string(),
        success: true,
    });
    app.command_history.push(CommandEntry {
        command: "echo hello".to_string(),
        output: "hello".to_string(),
        success: true,
    });

    // Set some state that should be cleared
    app.current_input = "some input".to_string();
    app.cursor_position = 5;
    app.scroll_offset = 3;
    app.show_command_list = true;
    app.command_filter = "filter".to_string();
    app.selected_command_index = 1;
    app.reverse_search_active = true;
    app.reverse_search_query = "search".to_string();
    app.reverse_search_results = vec!["result1".to_string(), "result2".to_string()];
    app.reverse_search_index = 1;
    app.auto_suggestion = Some("suggestion".to_string());
    app.history_index = Some(0);
    app.saved_input = "saved".to_string();

    // Call clear_screen
    app.clear_screen();

    // Verify that display state is reset
    assert_eq!(app.current_input, "");
    assert_eq!(app.cursor_position, 0);
    assert_eq!(app.scroll_offset, 0);
    assert!(!app.show_command_list);
    assert_eq!(app.command_filter, "");
    assert_eq!(app.selected_command_index, 0);
    assert!(!app.reverse_search_active);
    assert_eq!(app.reverse_search_query, "");
    assert!(app.reverse_search_results.is_empty());
    assert_eq!(app.reverse_search_index, 0);
    assert!(app.auto_suggestion.is_none());
    assert!(app.history_index.is_none());
    assert_eq!(app.saved_input, "");

    // Verify that command history is completely cleared
    assert_eq!(app.command_history.len(), 0);
}

#[tokio::test]
async fn test_clear_command_builtin() {
    let pool = taskhub::db::init_db(Some(":memory:".into())).await.unwrap();

    let mut app = App::new(pool);

    // Add some state that should be cleared
    app.current_input = "test input".to_string();
    app.cursor_position = 3;
    app.scroll_offset = 2;

    // Test that /clear command is handled as a builtin
    let is_builtin = app.handle_builtin_command("/clear").await;
    assert!(is_builtin);

    // Verify state was cleared
    assert_eq!(app.current_input, "");
    assert_eq!(app.cursor_position, 0);
    assert_eq!(app.scroll_offset, 0);
}

#[tokio::test]
async fn test_clear_command_in_available_commands() {
    let pool = taskhub::db::init_db(Some(":memory:".into())).await.unwrap();
    let app = App::new(pool);

    // Verify that /clear is in the available commands list
    assert!(app.available_commands.contains(&"/clear".to_string()));
}
