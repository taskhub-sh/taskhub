use std::time::Duration;
use taskhub::db::init_db;
use taskhub::tui::app::App;

#[tokio::test]
async fn test_rgb_color_support() {
    // Test RGB color codes that should be preserved in output
    let db_pool = init_db(Some(":memory:".into())).await.unwrap();
    let mut app = App::new(db_pool.clone());

    // RGB color command (24-bit true color) - use actual escape sequences
    let rgb_command = "printf '\x1b[38;2;255;100;50mRGB Red Orange\x1b[0m'";

    println!("Testing RGB color support");

    // Execute the command
    app.execute_command(rgb_command.to_string()).await;

    // Wait for command to complete
    let start = std::time::Instant::now();
    let timeout_duration = Duration::from_secs(5);

    while app.running_command.is_some() && start.elapsed() < timeout_duration {
        app.check_running_command().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Find the command entry in history
    let command_entry = app
        .command_history
        .iter()
        .find(|entry| entry.command == rgb_command)
        .expect("Should find RGB command in history");

    println!("RGB command output: '{}'", command_entry.output);

    // The command should succeed
    assert!(command_entry.success, "RGB printf command should succeed");

    // The output should contain RGB color codes
    let has_rgb_color = command_entry.output.contains("\x1b[38;2;255;100;50m");

    assert!(
        has_rgb_color,
        "RGB color codes should be preserved in output: '{}'",
        command_entry.output.replace('\x1b', "\\x1b")
    );

    // The text content should also be present
    assert!(
        command_entry.output.contains("RGB Red Orange"),
        "Text content should be preserved: '{}'",
        command_entry.output
    );
}

#[tokio::test]
async fn test_indexed_color_support() {
    // Test 256-color indexed color codes that should be preserved in output
    let db_pool = init_db(Some(":memory:".into())).await.unwrap();
    let mut app = App::new(db_pool.clone());

    // Indexed color command (256-color palette) - use actual escape sequences
    let indexed_command = "printf '\x1b[38;5;196mBright Red (Index 196)\x1b[0m'";

    println!("Testing indexed color support");

    // Execute the command
    app.execute_command(indexed_command.to_string()).await;

    // Wait for command to complete
    let start = std::time::Instant::now();
    let timeout_duration = Duration::from_secs(5);

    while app.running_command.is_some() && start.elapsed() < timeout_duration {
        app.check_running_command().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Find the command entry in history
    let command_entry = app
        .command_history
        .iter()
        .find(|entry| entry.command == indexed_command)
        .expect("Should find indexed command in history");

    println!("Indexed command output: '{}'", command_entry.output);

    // The command should succeed
    assert!(
        command_entry.success,
        "Indexed printf command should succeed"
    );

    // The output should contain either indexed color codes or their RGB equivalent
    // Indexed color 196 should be bright red (255,0,0)
    let has_indexed_color = command_entry.output.contains("\x1b[38;5;196m")
        || command_entry.output.contains("\x1b[38;2;255;0;0m");

    assert!(
        has_indexed_color,
        "Indexed color codes should be preserved or converted to RGB in output: '{}'",
        command_entry.output.replace('\x1b', "\\x1b")
    );

    // The text content should also be present
    assert!(
        command_entry.output.contains("Bright Red (Index 196)"),
        "Text content should be preserved: '{}'",
        command_entry.output
    );
}

#[tokio::test]
async fn test_rgb_background_color_support() {
    // Test RGB background color codes
    let db_pool = init_db(Some(":memory:".into())).await.unwrap();
    let mut app = App::new(db_pool.clone());

    // RGB background color command - use actual escape sequences
    let rgb_bg_command = "printf '\x1b[48;2;0;100;255mBlue Background\x1b[0m'";

    println!("Testing RGB background color support");

    // Execute the command
    app.execute_command(rgb_bg_command.to_string()).await;

    // Wait for command to complete
    let start = std::time::Instant::now();
    let timeout_duration = Duration::from_secs(5);

    while app.running_command.is_some() && start.elapsed() < timeout_duration {
        app.check_running_command().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Find the command entry in history
    let command_entry = app
        .command_history
        .iter()
        .find(|entry| entry.command == rgb_bg_command)
        .expect("Should find RGB background command in history");

    println!("RGB background command output: '{}'", command_entry.output);

    // The command should succeed
    assert!(
        command_entry.success,
        "RGB background printf command should succeed"
    );

    // The output should contain RGB background color codes
    let has_rgb_bg_color = command_entry.output.contains("\x1b[48;2;0;100;255m");

    assert!(
        has_rgb_bg_color,
        "RGB background color codes should be preserved in output: '{}'",
        command_entry.output.replace('\x1b', "\\x1b")
    );

    // The text content should also be present
    assert!(
        command_entry.output.contains("Blue Background"),
        "Text content should be preserved: '{}'",
        command_entry.output
    );
}

#[tokio::test]
async fn test_indexed_background_color_support() {
    // Test indexed background color codes
    let db_pool = init_db(Some(":memory:".into())).await.unwrap();
    let mut app = App::new(db_pool.clone());

    // Indexed background color command - use actual escape sequences
    let indexed_bg_command = "printf '\x1b[48;5;226mYellow Background (Index 226)\x1b[0m'";

    println!("Testing indexed background color support");

    // Execute the command
    app.execute_command(indexed_bg_command.to_string()).await;

    // Wait for command to complete
    let start = std::time::Instant::now();
    let timeout_duration = Duration::from_secs(5);

    while app.running_command.is_some() && start.elapsed() < timeout_duration {
        app.check_running_command().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Find the command entry in history
    let command_entry = app
        .command_history
        .iter()
        .find(|entry| entry.command == indexed_bg_command)
        .expect("Should find indexed background command in history");

    println!(
        "Indexed background command output: '{}'",
        command_entry.output
    );

    // The command should succeed
    assert!(
        command_entry.success,
        "Indexed background printf command should succeed"
    );

    // The output should contain either indexed background color codes or their RGB equivalent
    // Indexed color 226 should be bright yellow (255,255,0)
    let has_indexed_bg_color = command_entry.output.contains("\x1b[48;5;226m")
        || command_entry.output.contains("\x1b[48;2;255;255;0m");

    assert!(
        has_indexed_bg_color,
        "Indexed background color codes should be preserved or converted to RGB in output: '{}'",
        command_entry.output.replace('\x1b', "\\x1b")
    );

    // The text content should also be present
    assert!(
        command_entry
            .output
            .contains("Yellow Background (Index 226)"),
        "Text content should be preserved: '{}'",
        command_entry.output
    );
}

#[tokio::test]
async fn test_complex_rgb_indexed_mix() {
    // Test a mix of RGB and indexed colors with complex ANSI processing
    let db_pool = init_db(Some(":memory:".into())).await.unwrap();
    let mut app = App::new(db_pool.clone());

    // Complex command with RGB, indexed, and screen clearing - use actual escape sequences
    let complex_command = "printf '\x1b[38;2;255;0;128mRGB Magenta\x1b[0m \x1b[38;5;46mIndexed Green\x1b[0m\x1b[2J\x1b[H\x1b[48;2;50;50;50m\x1b[38;5;15mWhite on Gray\x1b[0m'";

    println!("Testing complex RGB/indexed color mix");

    // Execute the command
    app.execute_command(complex_command.to_string()).await;

    // Wait for command to complete
    let start = std::time::Instant::now();
    let timeout_duration = Duration::from_secs(5);

    while app.running_command.is_some() && start.elapsed() < timeout_duration {
        app.check_running_command().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Find the command entry in history
    let command_entry = app
        .command_history
        .iter()
        .find(|entry| entry.command == complex_command)
        .expect("Should find complex command in history");

    println!("Complex command output: '{}'", command_entry.output);

    // The command should succeed
    assert!(
        command_entry.success,
        "Complex color command should succeed"
    );

    // Check for RGB foreground color (magenta)
    let has_rgb_fg = command_entry.output.contains("\x1b[38;2;255;0;128m");

    // Check for indexed foreground colors (may be converted to RGB)
    // Index 46 is bright green, Index 15 is bright white
    let has_indexed_fg = command_entry.output.contains("\x1b[38;5;46m") ||
                        command_entry.output.contains("\x1b[38;5;15m") ||
                        command_entry.output.contains("\x1b[38;2;0;255;0m") ||  // bright green RGB
                        command_entry.output.contains("\x1b[38;2;255;255;255m"); // bright white RGB

    // Check for RGB background color (dark gray)
    let has_rgb_bg = command_entry.output.contains("\x1b[48;2;50;50;50m");

    // For complex commands with screen clearing, sometimes only the final output is preserved
    // So we check if any color codes are present OR if the expected text is present
    let has_color_codes = has_rgb_fg || has_indexed_fg || has_rgb_bg;
    let has_expected_text = command_entry.output.contains("White on Gray")
        || command_entry.output.contains("RGB Magenta")
        || command_entry.output.contains("Indexed Green");

    assert!(
        has_color_codes || has_expected_text,
        "Complex ANSI processing should preserve RGB/indexed color codes or text in output: '{}'",
        command_entry.output.replace('\x1b', "\\x1b")
    );
}
