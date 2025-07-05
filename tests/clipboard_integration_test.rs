use taskhub::db::init_db;
use taskhub::tui::app::App;

#[tokio::test]
async fn test_input_selection_functionality() {
    let db_pool = init_db(None).await.expect("Failed to initialize database");
    let mut app = App::new(db_pool);

    // Set up some input text
    app.current_input = "hello world".to_string();
    app.cursor_position = 11;

    // Test input selection
    app.start_input_selection(0);
    app.update_input_selection(5);
    app.end_input_selection();

    // Test getting selected input text
    let selected = app.get_selected_input_text();
    assert_eq!(selected, Some("hello".to_string()));

    // Test clearing selection
    app.clear_input_selection();
    assert_eq!(app.get_selected_input_text(), None);
}

#[tokio::test]
async fn test_mouse_coordinate_mapping() {
    let db_pool = init_db(None).await.expect("Failed to initialize database");
    let mut app = App::new(db_pool);

    // Set up terminal height
    app.set_terminal_area_height(24);

    // Test mouse position conversion for input area
    app.current_input = "test command".to_string();
    let pos = app.mouse_col_to_input_pos(10);

    // Should account for prompt length
    assert!(pos <= app.current_input.chars().count());
}

#[tokio::test]
async fn test_copy_paste_workflow() {
    let db_pool = init_db(None).await.expect("Failed to initialize database");
    let mut app = App::new(db_pool);

    // Set up input with selection
    app.current_input = "copy this text".to_string();
    app.start_input_selection(0);
    app.update_input_selection(4); // Select "copy"
    app.end_input_selection();

    // Verify selection exists
    assert_eq!(app.get_selected_input_text(), Some("copy".to_string()));

    // Note: Actual clipboard testing requires a windowing system
    // so we'll test the logic but not the actual clipboard operations
}
