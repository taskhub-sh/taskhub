use crossterm::event::{KeyCode, KeyModifiers};
use taskhub::tui::app::App;
use taskhub::tui::views::terminal::CommandEntry;

#[tokio::test]
async fn test_clear_command_user_experience() {
    // Create an in-memory database for testing
    let pool = taskhub::db::init_db(Some(":memory:".into())).await.unwrap();
    let mut app = App::new(pool);

    // Add some command history to simulate user activity
    app.command_history.push(CommandEntry {
        command: "ls".to_string(),
        output: "file1.txt\nfile2.txt\nfile3.txt".to_string(),
        success: true,
    });
    app.command_history.push(CommandEntry {
        command: "echo hello world".to_string(),
        output: "hello world".to_string(),
        success: true,
    });

    // Simulate typing "/clear" and pressing Enter
    for ch in "/clear".chars() {
        app.handle_terminal_input(ch);
    }

    // Verify that the command list is showing and "/clear" is an option
    assert!(app.show_command_list);
    let filtered_commands = app.get_filtered_commands();
    assert!(filtered_commands.contains(&"/clear".to_string()));

    // Simulate pressing Enter to execute the command
    app.on_key_code(KeyCode::Enter, KeyModifiers::NONE);

    // Process any pending commands
    app.handle_pending_commands().await;

    // Verify that clear was executed:
    // 1. Command history should be completely cleared
    assert_eq!(app.command_history.len(), 0);

    // 2. App state should be reset
    assert_eq!(app.current_input, "");
    assert_eq!(app.cursor_position, 0);
    assert_eq!(app.scroll_offset, 0);
    assert!(!app.show_command_list);

    println!("Clear command executed successfully!");
    println!("History now has {} entries", app.command_history.len());
}

#[tokio::test]
async fn test_ctrl_l_user_experience() {
    // Create an in-memory database for testing
    let pool = taskhub::db::init_db(Some(":memory:".into())).await.unwrap();
    let mut app = App::new(pool);

    // Add some command history to simulate user activity
    app.command_history.push(CommandEntry {
        command: "pwd".to_string(),
        output: "/home/user".to_string(),
        success: true,
    });

    // Type some input
    for ch in "some input text".chars() {
        app.handle_terminal_input(ch);
    }

    // Verify input is there
    assert_eq!(app.current_input, "some input text");
    assert!(app.cursor_position > 0);

    // Simulate pressing Ctrl+L
    app.on_key_code(KeyCode::Char('l'), KeyModifiers::CONTROL);

    // Verify that clear was executed:
    // 1. Command history should be completely cleared
    assert_eq!(app.command_history.len(), 0);

    // 2. App state should be reset
    assert_eq!(app.current_input, "");
    assert_eq!(app.cursor_position, 0);
    assert_eq!(app.scroll_offset, 0);

    println!("Ctrl+L executed successfully!");
    println!("History now has {} entries", app.command_history.len());
}
