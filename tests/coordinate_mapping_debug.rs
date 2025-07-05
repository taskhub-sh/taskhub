use taskhub::db::init_db;
use taskhub::tui::app::App;
use taskhub::tui::views::terminal::CommandEntry;

#[tokio::test]
async fn test_coordinate_mapping_with_content() {
    let db_pool = init_db(None).await.expect("Failed to initialize database");
    let mut app = App::new(db_pool);

    // Set up realistic terminal content
    let entries = vec![
        CommandEntry {
            command: "echo hello".to_string(),
            output: "hello".to_string(),
            success: true,
        },
        CommandEntry {
            command: "ls -la".to_string(),
            output: "file1.txt\nfile2.txt\nfile3.txt".to_string(),
            success: true,
        },
        CommandEntry {
            command: "pwd".to_string(),
            output: "/home/user".to_string(),
            success: true,
        },
    ];

    for entry in entries {
        app.command_history.push(entry);
    }

    // Set up terminal layout (typical size)
    app.update_layout_areas(24, false, 0);

    println!("=== Terminal Content Structure ===");

    // Calculate content structure (same as in map_mouse_to_content_line)
    let mut all_items_count = 0;
    let mut content_map = Vec::new();

    for (_entry_idx, entry) in app.command_history.iter().enumerate() {
        // Command line
        content_map.push(format!("Line {}: > {}", all_items_count, entry.command));
        all_items_count += 1;

        // Output lines
        if !entry.output.is_empty() {
            for (_line_idx, line) in entry.output.lines().enumerate() {
                content_map.push(format!("Line {}: {}", all_items_count, line));
                all_items_count += 1;
            }
        }

        // Empty line for spacing
        content_map.push(format!("Line {}: (empty)", all_items_count));
        all_items_count += 1;
    }

    println!("Total content lines: {}", all_items_count);
    for (_i, line) in content_map.iter().enumerate() {
        println!("  {}", line);
    }

    // Test coordinate mapping with no scroll
    println!("\n=== Testing Coordinate Mapping (No Scroll) ===");
    app.scroll_offset = 0;

    let test_clicks = vec![
        (1, 5),   // First line after border
        (2, 10),  // Second line
        (5, 0),   // Middle area
        (10, 15), // Lower area
    ];

    for (row, col) in &test_clicks {
        let mouse_row = app.history_area_start + row;
        if let Some((content_line, content_col)) = app.map_mouse_to_content_line(mouse_row, *col) {
            println!(
                "Mouse ({}, {}) -> Content line {}, col {}",
                row, col, content_line, content_col
            );
            if content_line < content_map.len() {
                println!("  Maps to: {}", content_map[content_line]);
            }
        } else {
            println!(
                "Mouse ({}, {}) -> No mapping (border or out of bounds)",
                row, col
            );
        }
    }

    // Test with scroll offset
    println!("\n=== Testing Coordinate Mapping (With Scroll) ===");
    app.scroll_offset = 3;

    for (row, col) in &test_clicks {
        let mouse_row = app.history_area_start + row;
        if let Some((content_line, content_col)) = app.map_mouse_to_content_line(mouse_row, *col) {
            println!(
                "Mouse ({}, {}) -> Content line {}, col {} (scroll={})",
                row, col, content_line, content_col, app.scroll_offset
            );
            if content_line < content_map.len() {
                println!("  Maps to: {}", content_map[content_line]);
            }
        } else {
            println!(
                "Mouse ({}, {}) -> No mapping (border or out of bounds)",
                row, col
            );
        }
    }
}

#[tokio::test]
async fn test_coordinate_mapping_edge_cases() {
    let db_pool = init_db(None).await.expect("Failed to initialize database");
    let mut app = App::new(db_pool);

    // Add minimal content
    app.command_history.push(CommandEntry {
        command: "test".to_string(),
        output: "output".to_string(),
        success: true,
    });

    app.update_layout_areas(24, false, 0);

    println!("\n=== Edge Case Testing ===");

    // Test border clicks
    let border_click = app.map_mouse_to_content_line(app.history_area_start, 5);
    println!("Border click result: {:?}", border_click);
    assert!(border_click.is_none(), "Border clicks should return None");

    // Test out of bounds
    let out_of_bounds = app.map_mouse_to_content_line(app.history_area_start + 100, 5);
    println!("Out of bounds click result: {:?}", out_of_bounds);

    // Test first valid line
    let first_line = app.map_mouse_to_content_line(app.history_area_start + 1, 5);
    println!("First line click result: {:?}", first_line);
    assert!(
        first_line.is_some(),
        "First content line should be clickable"
    );
}
