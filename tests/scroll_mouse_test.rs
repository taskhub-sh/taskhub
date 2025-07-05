use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use taskhub::db::init_db;
use taskhub::tui::app::App;
use taskhub::tui::views::terminal::CommandEntry;

#[tokio::test]
async fn test_mouse_with_scrolled_content() {
    let db_pool = init_db(None).await.expect("Failed to initialize database");
    let mut app = App::new(db_pool);

    // Add lots of content to force scrolling
    for i in 0..10 {
        app.command_history.push(CommandEntry {
            command: format!("command_{}", i),
            output: format!("output_{}_line1\noutput_{}_line2", i, i),
            success: true,
        });
    }

    // Set up terminal layout
    let terminal_height = 12; // Small terminal to force scrolling
    app.update_layout_areas(terminal_height, false, 0);

    println!("=== Scroll Mouse Test ===");
    println!("Terminal height: {}", terminal_height);
    println!(
        "History area: start={}, height={}",
        app.history_area_start, app.history_area_height
    );
    println!("Input area start: {}", app.input_area_start);

    // Build content structure
    let mut content_lines = Vec::new();
    for entry in &app.command_history {
        content_lines.push(format!("> {}", entry.command));
        if !entry.output.is_empty() {
            for line in entry.output.lines() {
                content_lines.push(line.to_string());
            }
        }
        content_lines.push(String::new()); // Empty line
    }

    println!("Total content lines: {}", content_lines.len());

    // Test with different scroll offsets
    let scroll_offsets = vec![0, 5, 10, 15];

    for scroll_offset in scroll_offsets {
        println!("\n=== Testing with scroll offset: {} ===", scroll_offset);
        app.scroll_offset = scroll_offset;

        // Calculate what should be visible
        let available_height = app.history_area_height.saturating_sub(2) as usize;
        let total_items = content_lines.len();

        println!("Available height: {}", available_height);
        println!("Total items: {}", total_items);

        if total_items > available_height {
            let visible_start = if scroll_offset >= total_items {
                0
            } else {
                total_items.saturating_sub(available_height + scroll_offset)
            };

            let visible_end = if scroll_offset == 0 {
                total_items
            } else {
                total_items.saturating_sub(scroll_offset)
            };

            println!("Visible range: {} to {}", visible_start, visible_end);

            // Show what should be visible
            for i in visible_start..visible_end.min(visible_start + available_height) {
                if i < content_lines.len() {
                    println!(
                        "  Visible line {}: '{}'",
                        i - visible_start,
                        content_lines[i]
                    );
                }
            }

            // Test clicking on the first visible line
            let click_row = 1u16; // First line after border
            let click_col = 5u16;

            println!("Clicking at row {}, col {}", click_row, click_col);

            app.clear_selection();

            let mouse_down = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: click_col,
                row: click_row,
                modifiers: crossterm::event::KeyModifiers::empty(),
            };

            app.on_mouse_event(mouse_down);

            if let Some((selected_line, _)) = app.selection_start {
                println!("Selected content line: {}", selected_line);
                if selected_line < content_lines.len() {
                    println!("Selected content: '{}'", content_lines[selected_line]);
                }

                // What line should be selected?
                let expected_line = visible_start;
                println!(
                    "Expected line: {} ('{}')",
                    expected_line,
                    if expected_line < content_lines.len() {
                        &content_lines[expected_line]
                    } else {
                        "out of bounds"
                    }
                );

                if selected_line == expected_line {
                    println!("✓ Selection matches expected line");
                } else {
                    println!(
                        "✗ Selection mismatch! Expected {}, got {}",
                        expected_line, selected_line
                    );
                }
            } else {
                println!("No selection made");
            }
        }
    }
}
