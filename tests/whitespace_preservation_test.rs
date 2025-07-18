use std::time::Duration;
use taskhub::db::init_db;
use taskhub::tui::app::App;

#[tokio::test]
async fn test_whitespace_preservation_simple() {
    let db_pool = init_db(Some(":memory:".into())).await.unwrap();
    let mut app = App::new(db_pool.clone());

    // Test a simple command that should preserve leading whitespace
    let command = "printf '\\t\\x1b[31mred\\x1b[0m text\\n'";

    println!("Testing whitespace preservation with command: {}", command);

    // Execute the command
    app.execute_command(command.to_string()).await;

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
        .find(|entry| entry.command == command)
        .expect("Should find command in history");

    println!("Command output: '{}'", command_entry.output);
    println!(
        "Command output bytes: {:?}",
        command_entry.output.as_bytes()
    );

    // Check if the output starts with a tab character
    assert!(
        command_entry.output.starts_with('\t'),
        "Output should start with tab character. Got: {:?}",
        command_entry.output.as_bytes()
    );

    // Check that we have the color codes
    assert!(
        command_entry.output.contains("\x1b[31m") || command_entry.output.contains("\\x1b[31m"),
        "Output should contain red color code"
    );
}
