use taskhub::db::init_db;
use taskhub::tui::app::App;
use taskhub::tui::views::terminal::CommandEntry;

#[tokio::test]
async fn test_realistic_coordinate_mapping() {
    let db_pool = init_db(None).await.expect("Failed to initialize database");
    let mut app = App::new(db_pool);

    // Add some actual content like in a real session
    app.command_history.push(CommandEntry {
        command: "echo hello".to_string(),
        output: "hello".to_string(),
        success: true,
    });
    app.command_history.push(CommandEntry {
        command: "ls".to_string(),
        output: "file1.txt\nfile2.txt".to_string(),
        success: true,
    });

    // Set up realistic terminal size and layout
    let terminal_height = 24;
    app.update_layout_areas(terminal_height, false, 0);

    println!("=== Realistic Terminal Layout ===");
    println!("Terminal height: {}", terminal_height);
    println!(
        "History area: start={}, height={}",
        app.history_area_start, app.history_area_height
    );
    println!("Input area start: {}", app.input_area_start);
    println!();

    // Test clicking on different rows as a user would
    println!("=== User Click Tests ===");

    // The Terminal Output area has a border, so:
    // - Row 0 is the top border with title "Terminal Output"
    // - Row 1 is the first content line ("> echo hello")
    // - Row 2 is the second content line ("hello")
    // - Row 3 is the empty line
    // - Row 4 is the next command ("> ls")
    // - etc.

    let user_clicks = vec![
        (0, 5, "Border click (should be ignored)"),
        (1, 5, "First content line: > echo hello"),
        (2, 3, "Second content line: hello"),
        (3, 0, "Empty spacing line"),
        (4, 2, "Third content line: > ls"),
        (5, 0, "Fourth content line: file1.txt"),
        (6, 0, "Fifth content line: file2.txt"),
    ];

    for (row, col, description) in user_clicks {
        println!(
            "Testing click at row {}, col {} ({})",
            row, col, description
        );

        if let Some((content_line, content_col)) = app.map_mouse_to_content_line(row, col) {
            println!(
                "  -> Maps to content line {}, col {}",
                content_line, content_col
            );

            // Let's also show what the actual content structure looks like
            let mut all_items = Vec::new();
            for entry in &app.command_history {
                all_items.push(format!("> {}", entry.command));
                if !entry.output.is_empty() {
                    for line in entry.output.lines() {
                        all_items.push(line.to_string());
                    }
                }
                all_items.push(String::new()); // Empty line
            }

            if content_line < all_items.len() {
                println!("  -> Content: '{}'", all_items[content_line]);
            } else {
                println!(
                    "  -> Content line {} is out of bounds (max: {})",
                    content_line,
                    all_items.len()
                );
            }
        } else {
            println!("  -> No mapping (border or invalid)");
        }
        println!();
    }
}
