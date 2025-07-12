use taskhub::db::init_db;
use taskhub::tui::app::App;

#[tokio::test]
async fn test_reverse_search_functionality() {
    let db_pool = init_db(None).await.unwrap();
    let mut app = App::new(db_pool.clone());

    // Add some test history
    app.persistent_command_history = vec![
        "ls -la".to_string(),
        "git status".to_string(),
        "cargo build".to_string(),
        "ls -la /home".to_string(),
        "git commit -m 'test'".to_string(),
    ];

    // Test starting reverse search
    assert!(!app.reverse_search_active);
    app.start_reverse_search();
    assert!(app.reverse_search_active);
    assert_eq!(app.reverse_search_query, "");
    assert_eq!(app.reverse_search_results, Vec::<String>::new());

    // Test searching for "git"
    app.reverse_search_query = "git".to_string();
    app.update_reverse_search();
    assert_eq!(app.reverse_search_results.len(), 2);
    assert_eq!(app.reverse_search_results[0], "git commit -m 'test'"); // Most recent first
    assert_eq!(app.reverse_search_results[1], "git status");

    // Test navigation
    assert_eq!(app.reverse_search_index, 0);
    app.reverse_search_next();
    assert_eq!(app.reverse_search_index, 1);
    app.reverse_search_previous();
    assert_eq!(app.reverse_search_index, 0);

    // Test bounds checking
    app.reverse_search_previous();
    assert_eq!(app.reverse_search_index, 0); // Should not go below 0
    app.reverse_search_next();
    app.reverse_search_next();
    assert_eq!(app.reverse_search_index, 1); // Should not go beyond length

    // Test accepting search result
    let selected_result = app.get_current_search_result().unwrap().clone();
    app.accept_reverse_search();
    assert!(!app.reverse_search_active);
    assert_eq!(app.current_input, selected_result);
    assert_eq!(app.cursor_position, selected_result.chars().count());

    // Test canceling search
    app.start_reverse_search();
    app.reverse_search_query = "test".to_string();
    app.cancel_reverse_search();
    assert!(!app.reverse_search_active);
    assert_eq!(app.reverse_search_query, "");
    assert_eq!(app.reverse_search_results, Vec::<String>::new());
}

#[tokio::test]
async fn test_reverse_search_case_insensitive() {
    let db_pool = init_db(None).await.unwrap();
    let mut app = App::new(db_pool.clone());

    app.persistent_command_history = vec![
        "Git Status".to_string(),
        "GIT COMMIT".to_string(),
        "git log".to_string(),
    ];

    app.start_reverse_search();
    app.reverse_search_query = "git".to_string();
    app.update_reverse_search();

    assert_eq!(app.reverse_search_results.len(), 3);
    assert!(app.reverse_search_results.contains(&"git log".to_string()));
    assert!(
        app.reverse_search_results
            .contains(&"GIT COMMIT".to_string())
    );
    assert!(
        app.reverse_search_results
            .contains(&"Git Status".to_string())
    );
}

#[tokio::test]
async fn test_reverse_search_no_matches() {
    let db_pool = init_db(None).await.unwrap();
    let mut app = App::new(db_pool.clone());

    app.persistent_command_history = vec!["ls -la".to_string(), "cargo build".to_string()];

    app.start_reverse_search();
    app.reverse_search_query = "nonexistent".to_string();
    app.update_reverse_search();

    assert_eq!(app.reverse_search_results.len(), 0);
    assert!(app.get_current_search_result().is_none());
}

#[tokio::test]
async fn test_reverse_search_prompt_generation() {
    let db_pool = init_db(None).await.unwrap();
    let mut app = App::new(db_pool.clone());

    // Test when not active
    assert!(!app.reverse_search_active);
    assert_eq!(app.get_reverse_search_prompt(), "");

    // Test when active with no query
    app.start_reverse_search();
    let prompt = app.get_reverse_search_prompt();
    assert!(prompt.contains("reverse-i-search"));
    assert!(prompt.contains("no matches"));

    // Test when active with results
    app.persistent_command_history = vec!["test command".to_string()];
    app.reverse_search_query = "test".to_string();
    app.update_reverse_search();
    let prompt = app.get_reverse_search_prompt();
    assert!(prompt.contains("reverse-i-search"));
    assert!(prompt.contains("1/1"));
    assert!(prompt.contains("test"));
}
