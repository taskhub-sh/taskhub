use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use taskhub::db::init_db;
use taskhub::tui::app::App;
use taskhub::tui::views::terminal::CommandEntry;

#[tokio::test]
async fn test_mouse_selection_behavior() {
    let db_pool = init_db(None).await.expect("Failed to initialize database");
    let mut app = App::new(db_pool);

    // Add some content to work with
    app.command_history.push(CommandEntry {
        command: "echo hello world".to_string(),
        output: "hello world".to_string(),
        success: true,
    });
    app.command_history.push(CommandEntry {
        command: "ls -la".to_string(),
        output: "file1.txt\nfile2.txt\ndirectory/".to_string(),
        success: true,
    });

    // Set up terminal layout
    let terminal_height = 24;
    app.update_layout_areas(terminal_height, false, 0);

    println!("=== Mouse Selection Test ===");
    println!("Terminal height: {}", terminal_height);
    println!(
        "History area: start={}, height={}",
        app.history_area_start, app.history_area_height
    );
    println!("Input area start: {}", app.input_area_start);
    println!();

    // Print the content structure so we can see what should be selected
    println!("=== Content Structure ===");
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

    for (i, line) in content_lines.iter().enumerate() {
        println!("Content line {}: '{}'", i, line);
    }
    println!();

    // Test mouse selection at specific coordinates
    println!("=== Testing Mouse Selection ===");

    // Test clicking on the first command line (should be at visual row 1)
    let click_row = 1u16;
    let click_col = 5u16;

    println!(
        "User clicks at visual row {}, col {} (expecting to select first command)",
        click_row, click_col
    );

    // Simulate mouse down event
    let mouse_down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: click_col,
        row: click_row,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };

    app.on_mouse_event(mouse_down);

    // Check what was selected
    println!("Selection start: {:?}", app.selection_start);
    println!("Selection end: {:?}", app.selection_end);
    println!("Is selecting: {}", app.is_selecting);

    if let Some((start_line, _start_col)) = app.selection_start {
        if start_line < content_lines.len() {
            println!("Selected line content: '{}'", content_lines[start_line]);
            println!("Expected: '> echo hello world'");

            // Check if it matches expectation
            if content_lines[start_line] == "> echo hello world" {
                println!("✓ Selection matches expectation!");
            } else {
                println!("✗ Selection mismatch!");
                println!("  Expected line 0: '> echo hello world'");
                println!("  Got line {}: '{}'", start_line, content_lines[start_line]);
            }
        } else {
            println!("Selection line {} is out of bounds", start_line);
        }
    }

    // Test clicking on the second line (output "hello world")
    let click_row = 2u16;
    let click_col = 3u16;

    println!(
        "\nUser clicks at visual row {}, col {} (expecting to select output 'hello world')",
        click_row, click_col
    );

    // Reset selection
    app.clear_selection();

    let mouse_down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: click_col,
        row: click_row,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };

    app.on_mouse_event(mouse_down);

    // Check what was selected
    println!("Selection start: {:?}", app.selection_start);

    if let Some((start_line, _start_col)) = app.selection_start {
        if start_line < content_lines.len() {
            println!("Selected line content: '{}'", content_lines[start_line]);
            println!("Expected: 'hello world'");

            // Check if it matches expectation
            if content_lines[start_line] == "hello world" {
                println!("✓ Selection matches expectation!");
            } else {
                println!("✗ Selection mismatch!");
                println!("  Expected line 1: 'hello world'");
                println!("  Got line {}: '{}'", start_line, content_lines[start_line]);
            }
        } else {
            println!("Selection line {} is out of bounds", start_line);
        }
    }
}
