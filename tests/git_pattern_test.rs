use std::time::Duration;
use taskhub::db::init_db;
use taskhub::tui::app::App;

#[tokio::test]
async fn test_git_pattern_with_complex_processing() {
    let db_pool = init_db(Some(":memory:".into())).await.unwrap();
    let mut app = App::new(db_pool.clone());

    // Create a command that mimics git status pattern and triggers complex processing
    // This has enough escape sequences to trigger complex processing
    let command = "printf '\\t\\x1b[31mmodified:\\x1b[0m   test.txt\\n\\x1b[2J\\x1b[H'";

    println!("Testing git pattern with complex processing: {}", command);

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
    if !command_entry.output.starts_with('\t') {
        println!("ERROR: Output does not start with tab character");
        println!("Expected: tab + color codes");
        println!(
            "Actual: {:?}",
            command_entry.output.chars().take(5).collect::<String>()
        );
    }

    // The command should succeed
    assert!(command_entry.success, "Command should succeed");

    // For debugging, let's just check what we got
    println!(
        "First 10 bytes: {:?}",
        &command_entry.output.as_bytes()[..command_entry.output.len().min(10)]
    );
}
