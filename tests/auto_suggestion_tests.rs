use crossterm::event::{KeyCode, KeyModifiers};
use sqlx::SqlitePool;
use taskhub::tui::app::App;

#[tokio::test]
async fn test_auto_suggestion_basic() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let mut app = App::new(pool);

    // Add some commands to persistent history
    app.persistent_command_history.push("ls -la".to_string());
    app.persistent_command_history
        .push("echo hello".to_string());
    app.persistent_command_history.push("ls".to_string());

    // Start typing "l" - should suggest "ls" (most recent match)
    app.current_input = "l".to_string();
    app.cursor_position = 1;
    app.update_auto_suggestion();

    assert_eq!(app.auto_suggestion, Some("ls".to_string()));

    // Type "ls " - should suggest "ls -la"
    app.current_input = "ls ".to_string();
    app.cursor_position = 3;
    app.update_auto_suggestion();

    assert_eq!(app.auto_suggestion, Some("ls -la".to_string()));

    // Type "echo" - should suggest "echo hello"
    app.current_input = "echo".to_string();
    app.cursor_position = 4;
    app.update_auto_suggestion();

    assert_eq!(app.auto_suggestion, Some("echo hello".to_string()));
}

#[tokio::test]
async fn test_auto_suggestion_no_match() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let mut app = App::new(pool);

    // Add some commands to persistent history
    app.persistent_command_history.push("ls".to_string());
    app.persistent_command_history
        .push("echo hello".to_string());

    // Type something that doesn't match - should have no suggestion
    app.current_input = "xyz".to_string();
    app.cursor_position = 3;
    app.update_auto_suggestion();

    assert_eq!(app.auto_suggestion, None);
}

#[tokio::test]
async fn test_auto_suggestion_exact_match() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let mut app = App::new(pool);

    // Add some commands to persistent history
    app.persistent_command_history.push("ls".to_string());

    // Type exact match - should have no suggestion (no point suggesting the same thing)
    app.current_input = "ls".to_string();
    app.cursor_position = 2;
    app.update_auto_suggestion();

    assert_eq!(app.auto_suggestion, None);
}

#[tokio::test]
async fn test_auto_suggestion_empty_input() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let mut app = App::new(pool);

    // Add some commands to persistent history
    app.persistent_command_history.push("ls".to_string());

    // Empty input - should have no suggestion
    app.current_input = "".to_string();
    app.cursor_position = 0;
    app.update_auto_suggestion();

    assert_eq!(app.auto_suggestion, None);
}

#[tokio::test]
async fn test_auto_suggestion_with_command_list() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let mut app = App::new(pool);

    // Add some commands to persistent history
    app.persistent_command_history.push("ls".to_string());

    // Show command list (typing "/") - should have no suggestion
    app.current_input = "/".to_string();
    app.cursor_position = 1;
    app.show_command_list = true;
    app.update_auto_suggestion();

    assert_eq!(app.auto_suggestion, None);
}

#[tokio::test]
async fn test_auto_suggestion_most_recent() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let mut app = App::new(pool);

    // Add commands in order - should suggest the most recent match
    app.persistent_command_history.push("ls -a".to_string());
    app.persistent_command_history.push("ls -la".to_string());
    app.persistent_command_history.push("ls".to_string());

    // Type "ls" - should suggest "ls" (most recent exact match should be ignored)
    // Actually, let's type "ls " to get a suggestion
    app.current_input = "ls ".to_string();
    app.cursor_position = 3;
    app.update_auto_suggestion();

    // Should suggest the most recent command that starts with "ls " which is "ls -la"
    assert_eq!(app.auto_suggestion, Some("ls -la".to_string()));
}

#[tokio::test]
async fn test_auto_suggestion_acceptance_with_tab() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let mut app = App::new(pool);

    // Add commands to history
    app.persistent_command_history
        .push("echo hello world".to_string());

    // Start typing "echo"
    app.current_input = "echo".to_string();
    app.cursor_position = 4;
    app.update_auto_suggestion();

    assert_eq!(app.auto_suggestion, Some("echo hello world".to_string()));

    // Press Tab to accept suggestion
    app.handle_tab_completion();

    assert_eq!(app.current_input, "echo hello world");
    assert_eq!(app.cursor_position, 16);
    assert_eq!(app.auto_suggestion, None);
}

#[tokio::test]
async fn test_auto_suggestion_acceptance_with_right_arrow() {
    use crossterm::event::{KeyCode, KeyModifiers};

    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let mut app = App::new(pool);

    // Add commands to history
    app.persistent_command_history
        .push("git status".to_string());

    // Start typing "git"
    app.current_input = "git".to_string();
    app.cursor_position = 3;
    app.update_auto_suggestion();

    assert_eq!(app.auto_suggestion, Some("git status".to_string()));

    // Press Right arrow at end of input to accept next character
    app.on_key_code(KeyCode::Right, KeyModifiers::empty());

    assert_eq!(app.current_input, "git ");
    assert_eq!(app.cursor_position, 4);
    assert_eq!(app.auto_suggestion, Some("git status".to_string()));

    // Press Right arrow again to accept next character
    app.on_key_code(KeyCode::Right, KeyModifiers::empty());

    assert_eq!(app.current_input, "git s");
    assert_eq!(app.cursor_position, 5);
    assert_eq!(app.auto_suggestion, Some("git status".to_string()));
}

#[tokio::test]
async fn test_auto_suggestion_full_acceptance_with_tab() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let mut app = App::new(pool);

    // Add commands to history
    app.persistent_command_history
        .push("git status".to_string());

    // Start typing "git"
    app.current_input = "git".to_string();
    app.cursor_position = 3;
    app.update_auto_suggestion();

    assert_eq!(app.auto_suggestion, Some("git status".to_string()));

    // Press Tab to accept full suggestion
    app.on_key_code(KeyCode::Tab, KeyModifiers::empty());

    assert_eq!(app.current_input, "git status");
    assert_eq!(app.cursor_position, 10);
    assert_eq!(app.auto_suggestion, None);
}

#[tokio::test]
async fn test_auto_suggestion_cursor_position_behavior() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let mut app = App::new(pool);

    // Add commands to history
    app.persistent_command_history
        .push("git status".to_string());

    // Start typing "git"
    app.current_input = "git".to_string();
    app.cursor_position = 3;
    app.update_auto_suggestion();

    // Should show suggestion when cursor is at end
    assert_eq!(app.auto_suggestion, Some("git status".to_string()));

    // Move cursor to middle of input
    app.on_key_code(KeyCode::Left, KeyModifiers::empty());

    // Cursor should be at position 2, and suggestion should disappear
    assert_eq!(app.cursor_position, 2);
    assert_eq!(app.auto_suggestion, None);

    // Move cursor back to end
    app.on_key_code(KeyCode::Right, KeyModifiers::empty());

    // Cursor should be at position 3, and suggestion should reappear
    assert_eq!(app.cursor_position, 3);
    assert_eq!(app.auto_suggestion, Some("git status".to_string()));
}
