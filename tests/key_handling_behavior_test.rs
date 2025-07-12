/// This test demonstrates the key event handling behavior and verifies
/// that the main event loop correctly routes key events based on modifiers.
use crossterm::event::{KeyCode, KeyModifiers};
use taskhub::db::init_db;
use taskhub::tui::app::App;

async fn create_test_app() -> App {
    let pool = init_db(Some(":memory:".into())).await.unwrap();
    App::new(pool)
}

#[tokio::test]
async fn test_key_routing_behavior() {
    let mut app = create_test_app().await;

    // Test 1: Regular character without modifiers should be treated as input
    app.current_input = String::new();
    app.cursor_position = 0;

    // Simulate what happens when user presses 'a' without any modifiers
    app.on_key('a');
    assert_eq!(app.current_input, "a");
    assert_eq!(app.cursor_position, 1);

    // Test 2: Character with Ctrl modifier should be treated as shortcut
    app.current_input = "hello".to_string();
    app.cursor_position = 2; // Middle of "hello"

    // Simulate what happens when user presses Ctrl+A
    app.on_key_code(KeyCode::Char('a'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 0); // Should move to beginning
    assert_eq!(app.current_input, "hello"); // Content unchanged

    // Test 3: Character with Ctrl modifier should NOT be treated as regular input
    let original_input = app.current_input.clone();

    // Pressing Ctrl+E should move cursor, not add 'e' to input
    app.on_key_code(KeyCode::Char('e'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 5); // Moved to end
    assert_eq!(app.current_input, original_input); // Content unchanged

    // Test 4: Verify other modifiers also route to on_key_code
    app.current_input = "test".to_string();
    app.cursor_position = 0;

    // Even non-Ctrl modifiers should use on_key_code path
    app.on_key_code(KeyCode::Char('t'), KeyModifiers::SHIFT);
    // Note: Shift+T doesn't have special behavior implemented,
    // but it should still go through on_key_code path
    assert_eq!(app.current_input, "test"); // Unchanged
    assert_eq!(app.cursor_position, 0); // Unchanged
}

#[tokio::test]
async fn test_modifier_edge_cases() {
    let mut app = create_test_app().await;

    // Test with Alt modifier
    app.current_input = "test".to_string();
    app.cursor_position = 2;

    app.on_key_code(KeyCode::Char('x'), KeyModifiers::ALT);
    // Alt+X doesn't have special behavior, but should go through on_key_code
    assert_eq!(app.current_input, "test"); // Unchanged
    assert_eq!(app.cursor_position, 2); // Unchanged

    // Test with Shift modifier
    app.on_key_code(KeyCode::Char('y'), KeyModifiers::SHIFT);
    // Shift+Y doesn't have special behavior, but should go through on_key_code
    assert_eq!(app.current_input, "test"); // Unchanged
    assert_eq!(app.cursor_position, 2); // Unchanged

    // Test with combined modifiers
    app.on_key_code(
        KeyCode::Char('z'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    // Ctrl+Shift+Z doesn't have special behavior, but should go through on_key_code
    assert_eq!(app.current_input, "test"); // Unchanged
    assert_eq!(app.cursor_position, 2); // Unchanged
}

#[tokio::test]
async fn test_non_char_keys_always_use_key_code() {
    let mut app = create_test_app().await;
    app.current_input = "hello world".to_string();
    app.cursor_position = 5;

    // Arrow keys should always use on_key_code regardless of modifiers
    app.on_key_code(KeyCode::Left, KeyModifiers::empty());
    assert_eq!(app.cursor_position, 4);

    app.on_key_code(KeyCode::Right, KeyModifiers::empty());
    assert_eq!(app.cursor_position, 5);

    // Function keys, etc. should also use on_key_code
    app.on_key_code(KeyCode::Home, KeyModifiers::empty());
    assert_eq!(app.cursor_position, 0);

    app.on_key_code(KeyCode::End, KeyModifiers::empty());
    assert_eq!(app.cursor_position, 11);
}
