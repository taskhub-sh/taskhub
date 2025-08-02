use std::time::Duration;
use taskhub::db::init_db;
use taskhub::tui::app::App;
use tokio::time::timeout;

// Helper function to create a test app
async fn create_test_app() -> App {
    let pool = init_db(Some(":memory:".into())).await.unwrap();
    App::new(pool)
}

/// Test to demonstrate the bug where commands that should produce output show "(no output)"
/// This test should FAIL until the underlying issue is fixed.
#[tokio::test]
async fn test_simple_echo_command_produces_output() {
    let mut app = create_test_app().await;

    // Execute a simple echo command that should definitely produce output
    app.pending_command = Some("echo 'testing output'".to_string());
    app.handle_pending_commands().await;

    // Wait for command to complete with timeout
    let result = timeout(Duration::from_secs(5), async {
        while app.running_command.is_some() {
            app.check_running_command().await;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;

    assert!(result.is_ok(), "Command execution timed out");

    // Verify command was executed
    assert_eq!(app.command_history.len(), 1);
    let entry = &app.command_history[0];
    assert_eq!(entry.command, "echo 'testing output'");

    // This assertion should FAIL because the bug causes output to be "(no output)"
    // instead of the expected "testing output"
    assert!(
        entry.output.contains("testing output"),
        "Expected output to contain 'testing output', but got: '{}'",
        entry.output
    );

    // Additional check to ensure it's not showing "(no output)"
    assert_ne!(
        entry.output, "(no output)",
        "Command output should not be '(no output)' for a simple echo command"
    );
}

/// Test with a command that has newlines to verify line handling
#[tokio::test]
async fn test_echo_with_newlines_produces_output() {
    let mut app = create_test_app().await;

    // Execute echo command with explicit newlines using -e flag
    app.pending_command = Some("echo -e '123\\n345\\n678'".to_string());
    app.handle_pending_commands().await;

    // Wait for command to complete with timeout
    let result = timeout(Duration::from_secs(5), async {
        while app.running_command.is_some() {
            app.check_running_command().await;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;

    assert!(result.is_ok(), "Command execution timed out");

    // Verify command was executed
    assert_eq!(app.command_history.len(), 1);
    let entry = &app.command_history[0];
    assert_eq!(entry.command, "echo -e '123\\n345\\n678'");

    // This should show the multi-line output, not "(no output)"
    assert!(
        entry.output.contains("123")
            && entry.output.contains("345")
            && entry.output.contains("678"),
        "Expected output to contain '123', '345', and '678', but got: '{}'",
        entry.output
    );

    assert_ne!(
        entry.output, "(no output)",
        "Command output should not be '(no output)' for echo with content"
    );
}

/// Test with a command that produces stderr output
#[tokio::test]
async fn test_command_with_stderr_produces_output() {
    let mut app = create_test_app().await;

    // Use a command that writes to stderr
    app.pending_command = Some("echo 'error message' >&2".to_string());
    app.handle_pending_commands().await;

    // Wait for command to complete with timeout
    let result = timeout(Duration::from_secs(5), async {
        while app.running_command.is_some() {
            app.check_running_command().await;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;

    assert!(result.is_ok(), "Command execution timed out");

    // Verify command was executed
    assert_eq!(app.command_history.len(), 1);
    let entry = &app.command_history[0];
    assert_eq!(entry.command, "echo 'error message' >&2");

    // Should capture stderr output, not show "(no output)"
    assert!(
        entry.output.contains("error message"),
        "Expected output to contain 'error message', but got: '{}'",
        entry.output
    );

    assert_ne!(
        entry.output, "(no output)",
        "Command output should not be '(no output)' for command with stderr"
    );
}

/// Test to demonstrate the specific issue described by user
#[tokio::test]
async fn test_user_reported_echo_command() {
    let mut app = create_test_app().await;

    // Execute the exact command the user mentioned
    app.pending_command = Some("echo -e '123\\n345\\n678'".to_string());
    app.handle_pending_commands().await;

    // Wait for command to complete
    let result = timeout(Duration::from_secs(5), async {
        while app.running_command.is_some() {
            app.check_running_command().await;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;

    assert!(result.is_ok(), "Command execution timed out");

    // Debug: Print the actual output for investigation
    assert_eq!(app.command_history.len(), 1);
    let entry = &app.command_history[0];
    println!("Command: {}", entry.command);
    println!("Output: '{}'", entry.output);
    println!("Success: {}", entry.success);

    // The bug: this test should fail because the command produces "(no output)"
    // when it should produce the actual echo output
    assert!(
        !entry.output.is_empty() && entry.output != "(no output)",
        "BUG DEMONSTRATED: Command '{}' produced '{}' instead of expected output",
        entry.command,
        entry.output
    );
}
