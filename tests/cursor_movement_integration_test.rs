use crossterm::event::{KeyCode, KeyModifiers};
use taskhub::db::init_db;
use taskhub::tui::app::App;

async fn create_test_app() -> App {
    let pool = init_db(Some(":memory:".into())).await.unwrap();
    App::new(pool)
}

#[tokio::test]
async fn test_ctrl_shortcuts_work_with_key_code_handling() {
    let mut app = create_test_app().await;
    app.current_input = "hello world test".to_string();
    app.cursor_position = 11; // After "hello world"

    // Test that Ctrl+A goes to beginning
    app.on_key_code(KeyCode::Char('a'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 0);
    assert_eq!(app.current_input, "hello world test");

    // Test that Ctrl+E goes to end
    app.on_key_code(KeyCode::Char('e'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 16); // End of "hello world test"
    assert_eq!(app.current_input, "hello world test");

    // Test Ctrl+Left for word movement
    app.on_key_code(KeyCode::Left, KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 12); // Beginning of "test"
    assert_eq!(app.current_input, "hello world test");

    // Test Ctrl+K kills to end
    app.on_key_code(KeyCode::Char('k'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 12);
    assert_eq!(app.current_input, "hello world ");

    // Test Ctrl+A again
    app.on_key_code(KeyCode::Char('a'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 0);

    // Test Ctrl+Right for word movement
    app.on_key_code(KeyCode::Right, KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 6); // Beginning of "world"

    // Test Ctrl+F for single character forward
    app.on_key_code(KeyCode::Char('f'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 7);

    // Test Ctrl+B for single character backward
    app.on_key_code(KeyCode::Char('b'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 6);
}

#[tokio::test]
async fn test_regular_characters_still_work_without_modifiers() {
    let mut app = create_test_app().await;
    app.current_input = String::new();
    app.cursor_position = 0;

    // Test that regular 'a' still works (without Ctrl)
    app.on_key('a');
    assert_eq!(app.current_input, "a");
    assert_eq!(app.cursor_position, 1);

    // Test that regular 'e' still works (without Ctrl)
    app.on_key('e');
    assert_eq!(app.current_input, "ae");
    assert_eq!(app.cursor_position, 2);

    // Test that regular 'k' still works (without Ctrl)
    app.on_key('k');
    assert_eq!(app.current_input, "aek");
    assert_eq!(app.cursor_position, 3);
}

#[tokio::test]
async fn test_modifier_combinations_work_correctly() {
    let mut app = create_test_app().await;
    app.current_input = "test input".to_string();
    app.cursor_position = 5; // After "test "

    // Test that Ctrl+F moves forward one character
    app.on_key_code(KeyCode::Char('f'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 6);

    // Test that Ctrl+B moves backward one character
    app.on_key_code(KeyCode::Char('b'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 5);

    // Test word movement with multiple words
    app.on_key_code(KeyCode::Right, KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 10); // End of input

    app.on_key_code(KeyCode::Left, KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 5); // Beginning of "input"
}

#[tokio::test]
async fn test_edge_cases_with_ctrl_shortcuts() {
    let mut app = create_test_app().await;

    // Test with empty input
    app.current_input = String::new();
    app.cursor_position = 0;

    app.on_key_code(KeyCode::Char('a'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 0);

    app.on_key_code(KeyCode::Char('e'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 0);

    app.on_key_code(KeyCode::Char('k'), KeyModifiers::CONTROL);
    assert_eq!(app.current_input, "");

    // Test with single character
    app.current_input = "x".to_string();
    app.cursor_position = 0;

    app.on_key_code(KeyCode::Char('e'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 1);

    app.on_key_code(KeyCode::Char('a'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 0);

    app.on_key_code(KeyCode::Char('k'), KeyModifiers::CONTROL);
    assert_eq!(app.current_input, "");
}
