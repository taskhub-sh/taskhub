use crossterm::event::{KeyCode, KeyModifiers};
use taskhub::db::init_db;
use taskhub::tui::app::App;

async fn create_test_app() -> App {
    let pool = init_db(Some(":memory:".into())).await.unwrap();
    App::new(pool)
}

#[tokio::test]
async fn test_ctrl_a_moves_cursor_to_beginning() {
    let mut app = create_test_app().await;
    app.current_input = "hello world".to_string();
    app.cursor_position = 5; // Start in the middle

    app.on_key_code(KeyCode::Char('a'), KeyModifiers::CONTROL);

    assert_eq!(app.cursor_position, 0);
    assert_eq!(app.current_input, "hello world"); // Input unchanged
}

#[tokio::test]
async fn test_ctrl_e_moves_cursor_to_end() {
    let mut app = create_test_app().await;
    app.current_input = "hello world".to_string();
    app.cursor_position = 5; // Start in the middle

    app.on_key_code(KeyCode::Char('e'), KeyModifiers::CONTROL);

    assert_eq!(app.cursor_position, 11); // End of "hello world"
    assert_eq!(app.current_input, "hello world"); // Input unchanged
}

#[tokio::test]
async fn test_ctrl_f_starts_output_search() {
    let mut app = create_test_app().await;
    app.current_input = "hello".to_string();
    app.cursor_position = 2;

    app.on_key_code(KeyCode::Char('f'), KeyModifiers::CONTROL);

    assert_eq!(app.cursor_position, 2); // Position unchanged
    assert_eq!(app.current_input, "hello"); // Input unchanged
    assert!(app.output_search_active); // Search should be active
}

#[tokio::test]
async fn test_ctrl_f_at_end_starts_search() {
    let mut app = create_test_app().await;
    app.current_input = "hello".to_string();
    app.cursor_position = 5; // At end

    app.on_key_code(KeyCode::Char('f'), KeyModifiers::CONTROL);

    assert_eq!(app.cursor_position, 5); // Unchanged
    assert_eq!(app.current_input, "hello"); // Input unchanged
    assert!(app.output_search_active); // Search should be active
}

#[tokio::test]
async fn test_ctrl_b_moves_cursor_backward_one_char() {
    let mut app = create_test_app().await;
    app.current_input = "hello".to_string();
    app.cursor_position = 3;

    app.on_key_code(KeyCode::Char('b'), KeyModifiers::CONTROL);

    assert_eq!(app.cursor_position, 2);
    assert_eq!(app.current_input, "hello"); // Input unchanged
}

#[tokio::test]
async fn test_ctrl_b_at_beginning_does_nothing() {
    let mut app = create_test_app().await;
    app.current_input = "hello".to_string();
    app.cursor_position = 0; // At beginning

    app.on_key_code(KeyCode::Char('b'), KeyModifiers::CONTROL);

    assert_eq!(app.cursor_position, 0); // Unchanged
    assert_eq!(app.current_input, "hello"); // Input unchanged
}

#[tokio::test]
async fn test_ctrl_k_kills_to_end_of_line() {
    let mut app = create_test_app().await;
    app.current_input = "hello world".to_string();
    app.cursor_position = 5; // After "hello"

    app.on_key_code(KeyCode::Char('k'), KeyModifiers::CONTROL);

    assert_eq!(app.cursor_position, 5); // Cursor position unchanged
    assert_eq!(app.current_input, "hello"); // Text after cursor removed
}

#[tokio::test]
async fn test_ctrl_k_at_end_does_nothing() {
    let mut app = create_test_app().await;
    app.current_input = "hello".to_string();
    app.cursor_position = 5; // At end

    app.on_key_code(KeyCode::Char('k'), KeyModifiers::CONTROL);

    assert_eq!(app.cursor_position, 5); // Unchanged
    assert_eq!(app.current_input, "hello"); // Input unchanged
}

#[tokio::test]
async fn test_ctrl_k_at_beginning_clears_all() {
    let mut app = create_test_app().await;
    app.current_input = "hello world".to_string();
    app.cursor_position = 0; // At beginning

    app.on_key_code(KeyCode::Char('k'), KeyModifiers::CONTROL);

    assert_eq!(app.cursor_position, 0); // Unchanged
    assert_eq!(app.current_input, ""); // All text removed
}

#[tokio::test]
async fn test_ctrl_left_moves_backward_by_word() {
    let mut app = create_test_app().await;
    app.current_input = "hello world test".to_string();
    app.cursor_position = 16; // At end

    app.on_key_code(KeyCode::Left, KeyModifiers::CONTROL);

    assert_eq!(app.cursor_position, 12); // Beginning of "test"
    assert_eq!(app.current_input, "hello world test"); // Input unchanged
}

#[tokio::test]
async fn test_ctrl_left_with_spaces() {
    let mut app = create_test_app().await;
    app.current_input = "hello   world".to_string();
    app.cursor_position = 13; // At end

    app.on_key_code(KeyCode::Left, KeyModifiers::CONTROL);

    assert_eq!(app.cursor_position, 8); // Beginning of "world"
    assert_eq!(app.current_input, "hello   world"); // Input unchanged
}

#[tokio::test]
async fn test_ctrl_left_at_beginning_does_nothing() {
    let mut app = create_test_app().await;
    app.current_input = "hello world".to_string();
    app.cursor_position = 0; // At beginning

    app.on_key_code(KeyCode::Left, KeyModifiers::CONTROL);

    assert_eq!(app.cursor_position, 0); // Unchanged
    assert_eq!(app.current_input, "hello world"); // Input unchanged
}

#[tokio::test]
async fn test_ctrl_right_moves_forward_by_word() {
    let mut app = create_test_app().await;
    app.current_input = "hello world test".to_string();
    app.cursor_position = 0; // At beginning

    app.on_key_code(KeyCode::Right, KeyModifiers::CONTROL);

    assert_eq!(app.cursor_position, 6); // Beginning of "world"
    assert_eq!(app.current_input, "hello world test"); // Input unchanged
}

#[tokio::test]
async fn test_ctrl_right_with_spaces() {
    let mut app = create_test_app().await;
    app.current_input = "hello   world test".to_string();
    app.cursor_position = 0; // At beginning

    app.on_key_code(KeyCode::Right, KeyModifiers::CONTROL);

    assert_eq!(app.cursor_position, 8); // Beginning of "world"
    assert_eq!(app.current_input, "hello   world test"); // Input unchanged
}

#[tokio::test]
async fn test_ctrl_right_at_end_does_nothing() {
    let mut app = create_test_app().await;
    app.current_input = "hello world".to_string();
    app.cursor_position = 11; // At end

    app.on_key_code(KeyCode::Right, KeyModifiers::CONTROL);

    assert_eq!(app.cursor_position, 11); // Unchanged
    assert_eq!(app.current_input, "hello world"); // Input unchanged
}

#[tokio::test]
async fn test_word_movement_with_multiple_words() {
    let mut app = create_test_app().await;
    app.current_input = "one two three four".to_string();
    app.cursor_position = 0; // At beginning

    // Move forward by word: one -> two
    app.on_key_code(KeyCode::Right, KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 4); // Beginning of "two"

    // Move forward by word: two -> three
    app.on_key_code(KeyCode::Right, KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 8); // Beginning of "three"

    // Move backward by word: three -> two
    app.on_key_code(KeyCode::Left, KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 4); // Beginning of "two"

    // Move backward by word: two -> one
    app.on_key_code(KeyCode::Left, KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 0); // Beginning of "one"
}

#[tokio::test]
async fn test_cursor_movement_with_unicode() {
    let mut app = create_test_app().await;
    app.current_input = "héllo wörld 🚀".to_string();
    app.cursor_position = 0;

    // Test Ctrl+E (end)
    app.on_key_code(KeyCode::Char('e'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, app.current_input.chars().count());

    // Test Ctrl+A (beginning)
    app.on_key_code(KeyCode::Char('a'), KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 0);

    // Test word movement with unicode
    app.on_key_code(KeyCode::Right, KeyModifiers::CONTROL);
    assert_eq!(app.cursor_position, 6); // After "héllo "
}
