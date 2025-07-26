use std::time::Duration;
use taskhub::db::init_db;
use taskhub::tui::app::App;

#[tokio::test]
async fn test_ansi_parser_preserves_colors_in_complex_processing() {
    // Test that when complex ANSI processing is triggered, colors are still preserved
    let db_pool = init_db(Some(":memory:".into())).await.unwrap();
    let mut app = App::new(db_pool.clone());

    // Create a command that triggers complex ANSI processing but also has colors
    let complex_command = "printf '\\x1b[31mred\\x1b[0m\\x1b[2J\\x1b[H\\x1b[32mgreen\\x1b[0m'";

    println!("Testing complex ANSI processing with colors");

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
        "Complex printf command should succeed"
    );

    // The output should contain color codes (red and green)
    // Even though this goes through complex ANSI processing, colors should be preserved
    let has_red_color =
        command_entry.output.contains("\x1b[31m") || command_entry.output.contains("\\x1b[31m");
    let has_green_color =
        command_entry.output.contains("\x1b[32m") || command_entry.output.contains("\\x1b[32m");

    assert!(
        has_red_color || has_green_color,
        "Complex ANSI processing should preserve color codes in output: '{}'",
        command_entry.output.replace('\x1b', "\\x1b")
    );
}
