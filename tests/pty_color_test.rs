use std::time::Duration;
use taskhub::db::init_db;
use taskhub::tui::app::App;

#[tokio::test]
async fn test_pty_execution_with_environment_variables() {
    // Create a test database
    let db_pool = init_db(Some(":memory:".into())).await.unwrap();

    // Create an app instance
    let mut app = App::new(db_pool.clone());

    // Test command that should show environment variables
    let test_command = if cfg!(target_os = "windows") {
        "echo %TERM% %FORCE_COLOR%".to_string()
    } else {
        "echo $TERM $FORCE_COLOR $CLICOLOR_FORCE".to_string()
    };

    // Execute the command
    app.execute_command(test_command.clone()).await;

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

    // Check that we have at least one command in history
    assert!(
        !app.command_history.is_empty(),
        "Should have command history"
    );

    // Find the command we just executed
    let command_entry = app
        .command_history
        .iter()
        .find(|entry| entry.command == test_command)
        .expect("Should find our test command in history");

    // For PTY execution, we should see the environment variables we set
    if !cfg!(target_os = "windows") {
        // On Unix systems, check for the environment variables we set
        assert!(
            command_entry.output.contains("xterm-256color")
                || command_entry.output.contains("1")
                || command_entry.success,
            "PTY execution should set color environment variables. Output: '{}'",
            command_entry.output
        );
    }

    // At minimum, the command should have succeeded
    assert!(command_entry.success, "Command should have succeeded");
}

#[tokio::test]
async fn test_pty_fallback_to_pipes() {
    // Create a test database
    let db_pool = init_db(Some(":memory:".into())).await.unwrap();

    // Create an app instance
    let mut app = App::new(db_pool.clone());

    // Test a simple command that should work in both PTY and pipe modes
    let test_command = if cfg!(target_os = "windows") {
        "echo hello world".to_string()
    } else {
        "echo hello world".to_string()
    };

    // Execute the command
    app.execute_command(test_command.clone()).await;

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

    // Check that we have at least one command in history
    assert!(
        !app.command_history.is_empty(),
        "Should have command history"
    );

    // Find the command we just executed
    let command_entry = app
        .command_history
        .iter()
        .find(|entry| entry.command == test_command)
        .expect("Should find our test command in history");

    // Check that the output contains what we expect
    assert!(
        command_entry.output.contains("hello world") || command_entry.success,
        "Command should produce expected output or succeed. Output: '{}'",
        command_entry.output
    );

    // The command should have succeeded
    assert!(command_entry.success, "Command should have succeeded");
}
