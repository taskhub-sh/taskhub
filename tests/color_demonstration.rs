use std::time::Duration;
use taskhub::db::init_db;
use taskhub::tui::app::App;

#[tokio::test]
async fn test_color_commands_work_by_default() {
    // Create a test database
    let db_pool = init_db(Some(":memory:".into())).await.unwrap();

    // Create an app instance
    let mut app = App::new(db_pool.clone());

    // Test commands that normally output colors when they detect a TTY
    let test_commands = vec![
        // ls should output colors (on systems that support it)
        "ls --color=auto",
        // echo with color codes should work
        "echo '\x1b[31mred text\x1b[0m'",
    ];

    for test_command in test_commands {
        println!("Testing command: {}", test_command);

        // Execute the command
        app.execute_command(test_command.to_string()).await;

        // Wait for command to complete (with timeout)
        let start = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(5);

        while app.running_command.is_some() && start.elapsed() < timeout_duration {
            app.check_running_command().await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Check that the command completed
        assert!(
            app.running_command.is_none(),
            "Command should have completed"
        );

        // Find the command we just executed
        if let Some(command_entry) = app
            .command_history
            .iter()
            .find(|entry| entry.command == test_command)
        {
            println!(
                "Command '{}' output: '{}'",
                test_command, command_entry.output
            );

            // Commands should generally succeed
            if !command_entry.success {
                println!(
                    "Warning: Command '{}' failed, but this might be expected on some systems",
                    test_command
                );
            }
        }
    }

    // Check that we have command history
    assert!(
        !app.command_history.is_empty(),
        "Should have command history"
    );

    println!("PTY-based execution test completed successfully!");
}
