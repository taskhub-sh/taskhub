use taskhub::db::init_db;
use taskhub::tui::app::App;

#[tokio::test]
async fn test_coordinate_mapping_accuracy() {
    let db_pool = init_db(None).await.expect("Failed to initialize database");
    let mut app = App::new(db_pool);

    // Set up terminal dimensions (typical terminal size)
    let terminal_height = 24;
    let _terminal_width = 80;

    // Test normal two-area layout (no command list)
    app.update_layout_areas(terminal_height, false, 0);

    println!("=== Two-area layout ===");
    println!("Terminal height: {}", terminal_height);
    println!(
        "History area: start={}, height={}",
        app.history_area_start, app.history_area_height
    );
    println!("Input area start: {}", app.input_area_start);

    // Verify layout calculations
    assert_eq!(app.history_area_start, 0);
    assert_eq!(app.history_area_height, 21); // 24 - 3 for input area
    assert_eq!(app.input_area_start, 21);

    // Test three-area layout (with command list)
    let command_list_size = 5; // 3 commands + 2 for borders
    app.update_layout_areas(terminal_height, true, command_list_size);

    println!("\n=== Three-area layout ===");
    println!("Terminal height: {}", terminal_height);
    println!("Command list size: {}", command_list_size);
    println!(
        "History area: start={}, height={}",
        app.history_area_start, app.history_area_height
    );
    println!("Input area start: {}", app.input_area_start);

    // Verify layout calculations with command list
    assert_eq!(app.history_area_start, 0);
    assert_eq!(app.history_area_height, 16); // 24 - 5 (command list) - 3 (input)
    assert_eq!(app.input_area_start, 21);

    // Test input position mapping
    app.current_input = "test command input".to_string();

    // Test coordinate mapping with different mouse positions
    let test_cases = vec![
        (0, 0),   // Top-left corner
        (5, 10),  // Middle of history area
        (21, 5),  // Input area
        (23, 40), // Bottom area
    ];

    for (row, col) in test_cases {
        println!("\nTesting mouse at ({}, {})", row, col);

        if row >= app.input_area_start as usize {
            let input_pos = app.mouse_col_to_input_pos(col);
            println!("  Maps to input position: {}", input_pos);
            assert!(input_pos <= app.current_input.chars().count());
        } else {
            println!("  Maps to history area");
            // Test that coordinates are properly adjusted for content
            let history_relative_row = row as u16 - app.history_area_start;
            let content_row = if history_relative_row > 0 {
                (history_relative_row - 1) as usize + app.scroll_offset
            } else {
                app.scroll_offset
            };
            println!("  Content row: {}", content_row);
        }
    }
}

#[tokio::test]
async fn test_scroll_offset_coordinate_mapping() {
    let db_pool = init_db(None).await.expect("Failed to initialize database");
    let mut app = App::new(db_pool);

    // Set up terminal and add scroll offset
    app.update_layout_areas(24, false, 0);
    app.scroll_offset = 10; // Scrolled up 10 lines

    println!("=== Scroll offset test ===");
    println!("Scroll offset: {}", app.scroll_offset);

    // Test that coordinates account for scroll offset
    let mouse_row = 5u16; // Click on row 5
    let history_relative_row = mouse_row - app.history_area_start;
    let content_row = if history_relative_row > 0 {
        (history_relative_row - 1) as usize + app.scroll_offset
    } else {
        app.scroll_offset
    };

    println!("Mouse row: {}", mouse_row);
    println!("History relative row: {}", history_relative_row);
    println!("Content row (with scroll): {}", content_row);

    // With scroll offset of 10, clicking on row 5 should map to content row 14
    assert_eq!(content_row, 14);
}
