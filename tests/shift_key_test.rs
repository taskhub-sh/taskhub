use crossterm::event::{KeyCode, KeyModifiers};
use taskhub::db::init_db;
use taskhub::tui::app::App;

/// Test that shift+letter combinations produce uppercase letters
#[tokio::test]
async fn test_shift_key_uppercase_letters() {
    let mut app = create_test_app().await;

    // Test shift+h should produce 'H'
    app.on_key('H'); // Crossterm should provide uppercase H when shift is pressed
    assert_eq!(app.current_input, "H");
    assert_eq!(app.cursor_position, 1);

    // Test multiple uppercase letters
    app.on_key('E');
    app.on_key('L');
    app.on_key('L');
    app.on_key('O');
    assert_eq!(app.current_input, "HELLO");
    assert_eq!(app.cursor_position, 5);
}

/// Test that regular lowercase letters still work
#[tokio::test]
async fn test_regular_lowercase_letters() {
    let mut app = create_test_app().await;

    app.on_key('h');
    app.on_key('e');
    app.on_key('l');
    app.on_key('l');
    app.on_key('o');
    assert_eq!(app.current_input, "hello");
    assert_eq!(app.cursor_position, 5);
}

/// Test mixed case input
#[tokio::test]
async fn test_mixed_case_input() {
    let mut app = create_test_app().await;

    app.on_key('H');
    app.on_key('e');
    app.on_key('L');
    app.on_key('l');
    app.on_key('o');
    app.on_key(' ');
    app.on_key('W');
    app.on_key('o');
    app.on_key('r');
    app.on_key('L');
    app.on_key('d');
    assert_eq!(app.current_input, "HeLlo WorLd");
    assert_eq!(app.cursor_position, 11);
}

/// Test that Ctrl combinations still work correctly
#[tokio::test]
async fn test_ctrl_combinations_still_work() {
    let mut app = create_test_app().await;

    // Add some text first
    app.on_key('h');
    app.on_key('e');
    app.on_key('l');
    app.on_key('l');
    app.on_key('o');
    assert_eq!(app.current_input, "hello");

    // Test Ctrl+A (move to beginning)
    app.on_key_code(KeyCode::Char('a'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 0);

    // Test Ctrl+E (move to end)
    app.on_key_code(KeyCode::Char('e'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 5);
}

/// Test special characters with shift
#[tokio::test]
async fn test_special_characters_with_shift() {
    let mut app = create_test_app().await;

    // These would be shift+number combinations producing symbols
    app.on_key('!'); // shift+1
    app.on_key('@'); // shift+2
    app.on_key('#'); // shift+3
    app.on_key('$'); // shift+4
    assert_eq!(app.current_input, "!@#$");
    assert_eq!(app.cursor_position, 4);
}

async fn create_test_app() -> App {
    let db_pool = init_db(Some(":memory:".into()))
        .await
        .expect("Failed to create test database");
    App::new(db_pool)
}
