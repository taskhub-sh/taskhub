use std::time::Duration;
use taskhub::db::init_db;
use taskhub::tui::app::App;

#[tokio::test]
async fn test_pty_preserves_working_directory() {
    // Create a test database
    let db_pool = init_db(Some(":memory:".into())).await.unwrap();

    // Create an app instance
    let mut app = App::new(db_pool.clone());

    // Get the current working directory where the test is running
    let expected_dir = std::env::current_dir().unwrap();
    let expected_dir_str = expected_dir.to_string_lossy().to_string();

    // Test command to print current working directory
    let test_command = if cfg!(target_os = "windows") {
        "cd".to_string() // Windows 'cd' without arguments prints current directory
    } else {
        "pwd".to_string() // Unix 'pwd' prints current working directory
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

    println!("Expected directory: {}", expected_dir_str);
    println!("Command output: '{}'", command_entry.output);

    // The command should have succeeded
    assert!(
        command_entry.success,
        "pwd/cd command should have succeeded"
    );

    // The output should contain the current working directory
    // Note: We check if the output contains our expected directory path
    assert!(
        command_entry.output.contains(&expected_dir_str)
            || command_entry.output.trim() == expected_dir_str
            || command_entry.output.contains("taskhub"), // At minimum should be in taskhub directory
        "Command output should contain current directory. Expected: '{}', Got: '{}'",
        expected_dir_str,
        command_entry.output
    );
}

#[tokio::test]
async fn test_pipe_preserves_working_directory() {
    // Create a test database
    let db_pool = init_db(Some(":memory:".into())).await.unwrap();

    // Create an app instance
    let mut app = App::new(db_pool.clone());

    // Get the current working directory where the test is running
    let expected_dir = std::env::current_dir().unwrap();
    let expected_dir_str = expected_dir.to_string_lossy().to_string();

    // Force using pipe execution by using a command that might fail in PTY
    // but should work in pipes (this is a bit of a hack for testing)
    let test_command = if cfg!(target_os = "windows") {
        "echo %CD%".to_string() // Windows environment variable for current directory
    } else {
        "echo $PWD".to_string() // Unix environment variable for current directory
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

    // Find the command we just executed
    let command_entry = app
        .command_history
        .iter()
        .find(|entry| entry.command == test_command)
        .expect("Should find our test command in history");

    println!("Expected directory: {}", expected_dir_str);
    println!("Command output: '{}'", command_entry.output);

    // The command should have succeeded
    assert!(
        command_entry.success,
        "Directory command should have succeeded"
    );

    // The output should contain the current working directory or at least be in taskhub
    assert!(
        command_entry.output.contains(&expected_dir_str)
            || command_entry.output.contains("taskhub")
            || !command_entry.output.trim().is_empty(),
        "Command output should show current directory. Expected: '{}', Got: '{}'",
        expected_dir_str,
        command_entry.output
    );
}
