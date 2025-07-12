use crossterm::event::{KeyCode, KeyModifiers};
use taskhub::db::init_db;
use taskhub::tui::app::App;

#[tokio::test]
async fn test_ctrl_r_key_event_handling() {
    let db_pool = init_db(None).await.unwrap();
    let mut app = App::new(db_pool.clone());

    // Add some test history first
    app.persistent_command_history = vec![
        "git status".to_string(),
        "cargo build".to_string(),
        "ls -la".to_string(),
    ];

    // Verify reverse search is initially inactive
    assert!(!app.reverse_search_active);

    // Simulate Ctrl+R key event (this is what the main event loop would call)
    app.on_key_code(KeyCode::Char('r'), KeyModifiers::CONTROL);

    // Verify reverse search is now active
    assert!(app.reverse_search_active);
    assert_eq!(app.reverse_search_query, "");
    assert_eq!(app.reverse_search_results, Vec::<String>::new());

    // Test that we can add to the search query
    app.handle_terminal_input('g');
    app.handle_terminal_input('i');
    app.handle_terminal_input('t');
    assert_eq!(app.reverse_search_query, "git");
    assert_eq!(app.reverse_search_results.len(), 1); // "git status"
    assert_eq!(app.reverse_search_results[0], "git status");

    // Test that we can navigate and accept
    let current_result = app.get_current_search_result().unwrap().clone();
    app.accept_reverse_search();
    assert!(!app.reverse_search_active);
    assert_eq!(app.current_input, current_result);
}

#[tokio::test]
async fn test_escape_cancels_reverse_search() {
    let db_pool = init_db(None).await.unwrap();
    let mut app = App::new(db_pool.clone());

    // Start reverse search
    app.on_key_code(KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert!(app.reverse_search_active);

    // Type some query
    app.handle_terminal_input('t');
    app.handle_terminal_input('e');
    app.handle_terminal_input('s');
    app.handle_terminal_input('t');
    assert_eq!(app.reverse_search_query, "test");

    // Press Escape to cancel
    app.on_key_code(KeyCode::Esc, KeyModifiers::empty());
    assert!(!app.reverse_search_active);
    assert_eq!(app.reverse_search_query, "");
    assert_eq!(app.reverse_search_results, Vec::<String>::new());
}

#[tokio::test]
async fn test_reverse_search_navigation() {
    let db_pool = init_db(None).await.unwrap();
    let mut app = App::new(db_pool.clone());

    // Add test history with multiple matching commands
    app.persistent_command_history = vec![
        "git log".to_string(),
        "git status".to_string(),
        "git commit".to_string(),
    ];

    // Start reverse search and search for "git"
    app.on_key_code(KeyCode::Char('r'), KeyModifiers::CONTROL);
    app.handle_terminal_input('g');
    app.handle_terminal_input('i');
    app.handle_terminal_input('t');

    // Should have 3 results, starting with most recent (git commit)
    assert_eq!(app.reverse_search_results.len(), 3);
    assert_eq!(app.reverse_search_index, 0);
    assert_eq!(app.get_current_search_result().unwrap(), "git commit");

    // Navigate down (to next older match)
    app.on_key_code(KeyCode::Down, KeyModifiers::empty());
    assert_eq!(app.reverse_search_index, 1);
    assert_eq!(app.get_current_search_result().unwrap(), "git status");

    // Navigate down again
    app.on_key_code(KeyCode::Down, KeyModifiers::empty());
    assert_eq!(app.reverse_search_index, 2);
    assert_eq!(app.get_current_search_result().unwrap(), "git log");

    // Navigate up (to newer match)
    app.on_key_code(KeyCode::Up, KeyModifiers::empty());
    assert_eq!(app.reverse_search_index, 1);
    assert_eq!(app.get_current_search_result().unwrap(), "git status");
}
