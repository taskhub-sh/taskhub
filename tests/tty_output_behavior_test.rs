use sqlx::SqlitePool;
use std::fs;
use std::path::PathBuf;
use taskhub::db::init_db;
use taskhub::tui::app::App;
use tokio::process::Command;

/// Creates a test database with proper migrations
async fn create_test_db() -> SqlitePool {
    init_db(Some(":memory:".into())).await.unwrap()
}

/// Test that demonstrates the TTY vs pipe behavior issue in command execution.
///
/// This test shows that the current implementation using `cmd.output().await`
/// produces piped output (one file per line) instead of TTY-formatted output
/// (multiple columns) for commands like `ls`.
///
/// **Expected Behavior:**
/// - Commands executed in a terminal should preserve TTY formatting
/// - `ls` command should show files in columns when there are multiple files
/// - Output should be formatted as if running in an interactive terminal
///
/// **Current Behavior (Bug):**
/// - Commands behave as if their output is piped (like `ls | cat`)
/// - `ls` shows one file per line instead of columnar format
/// - TTY context is lost due to using `cmd.output().await`
///
/// **Root Cause:**
/// The `execute_command_with_pipes` method in `src/tui/app.rs` uses
/// `cmd.output().await` which doesn't allocate a PTY, causing commands
/// to behave as if their output is being piped.
///
/// **Fix Required:**
/// Replace the current pipe-based approach with PTY (pseudo-terminal)
/// allocation using libraries like `portable-pty` to maintain TTY context.
#[tokio::test]
async fn test_commands_should_preserve_tty_formatting() {
    // Arrange: Create a temporary directory with multiple files
    let temp_dir = std::env::temp_dir().join(format!("tty_test_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let temp_path = &temp_dir;

    // Create several files to make the column vs single-line difference obvious
    let files = vec![
        "file01.txt",
        "file02.txt",
        "file03.txt",
        "file04.txt",
        "file05.txt",
        "file06.txt",
        "file07.txt",
        "file08.txt",
        "file09.txt",
        "file10.txt",
        "document_a.md",
        "document_b.md",
        "script.sh",
        "config.toml",
        "readme.rst",
    ];

    for file in &files {
        let file_path = temp_path.join(file);
        fs::write(&file_path, "test content").unwrap();
    }

    // Create test app with database
    let db_pool = create_test_db().await;
    let mut app = App::new(db_pool);

    // Change to the test directory
    std::env::set_current_dir(temp_path).unwrap();

    // Act: Execute ls command through TaskHub's command execution system
    let ls_command = "ls".to_string();
    app.execute_command(ls_command).await;

    // Wait for command to complete by polling the running command state
    let mut retries = 0;
    const MAX_RETRIES: u32 = 50; // 5 seconds max wait

    while app.running_command.is_some() && retries < MAX_RETRIES {
        app.check_running_command().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        retries += 1;
    }

    // Ensure we have a completed command entry
    assert!(!app.command_history.is_empty(), "No command was executed");

    // Get the output from the command history
    let command_entry = app.command_history.last().unwrap();
    let taskhub_output = &command_entry.output;

    // Get the expected TTY output by running ls directly with proper TTY allocation
    let expected_tty_output = get_expected_tty_ls_output(temp_path).await;

    // Assert: The current implementation will show single-column output (like ls | cat)
    // This assertion should FAIL, demonstrating the bug

    // Count lines in TaskHub output (current pipe behavior will have many lines)
    let taskhub_lines: Vec<&str> = taskhub_output.lines().collect();
    let taskhub_line_count = taskhub_lines.len();

    // Count lines in expected TTY output (should be fewer due to column formatting)
    let expected_lines: Vec<&str> = expected_tty_output.lines().collect();
    let expected_line_count = expected_lines.len();

    println!("=== DEBUGGING TTY vs PIPE BEHAVIOR ===");
    println!("TaskHub output (current implementation):");
    println!("{}", taskhub_output);
    println!("Lines in TaskHub output: {}", taskhub_line_count);
    println!("\nExpected TTY output:");
    println!("{}", expected_tty_output);
    println!("Lines in expected TTY output: {}", expected_line_count);

    // This assertion will FAIL with current implementation
    // TaskHub will show more lines (one file per line) than proper TTY output (columns)
    assert!(
        taskhub_line_count <= expected_line_count,
        "TaskHub output shows {} lines but TTY should show {} lines or fewer. \
         Current implementation behaves like 'ls | cat' instead of proper TTY formatting. \
         TaskHub output:\n{}\n\nExpected TTY output:\n{}",
        taskhub_line_count,
        expected_line_count,
        taskhub_output,
        expected_tty_output
    );

    // Additional check: ensure files are displayed in columns in TTY mode
    // Current implementation will fail this check
    let _has_column_formatting = check_for_column_formatting(&expected_tty_output, &files);
    let taskhub_has_columns = check_for_column_formatting(taskhub_output, &files);

    assert!(
        taskhub_has_columns,
        "TaskHub should preserve TTY column formatting like proper terminal output, \
         but current implementation shows single-column output like piped commands"
    );

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}

/// Get expected TTY-formatted ls output by using a PTY
async fn get_expected_tty_ls_output(directory: &PathBuf) -> String {
    // Use script command to simulate TTY environment
    // This is a workaround to get what the output SHOULD look like with proper TTY
    let output = if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        Command::new("script")
            .args(["-qec", "ls", "/dev/null"])
            .current_dir(directory)
            .output()
            .await
            .unwrap()
    } else {
        // Fallback for other systems - just use regular ls for comparison
        Command::new("ls")
            .current_dir(directory)
            .output()
            .await
            .unwrap()
    };

    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Check if output contains column formatting (multiple files on same line)
fn check_for_column_formatting(output: &str, files: &[&str]) -> bool {
    for line in output.lines() {
        let files_in_line = files.iter().filter(|&&file| line.contains(file)).count();

        // If any line contains multiple files, it has column formatting
        if files_in_line > 1 {
            return true;
        }
    }
    false
}

/// Additional test to demonstrate the specific difference between piped and TTY output
#[tokio::test]
async fn test_demonstrate_pipe_vs_tty_difference() {
    // Create temp directory with files
    let temp_dir = std::env::temp_dir().join(format!("tty_test_demo_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let temp_path = &temp_dir;

    let files = vec!["a.txt", "b.txt", "c.txt", "d.txt", "e.txt"];
    for file in &files {
        fs::write(temp_path.join(file), "test").unwrap();
    }

    // Get piped output (like current TaskHub implementation)
    let piped_output = Command::new("sh")
        .args(["-c", "ls"])
        .current_dir(temp_path)
        .output()
        .await
        .unwrap();
    let piped_text = String::from_utf8_lossy(&piped_output.stdout);

    // Get TTY output using script command
    let tty_output = if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        Command::new("script")
            .args(["-qec", "ls", "/dev/null"])
            .current_dir(temp_path)
            .output()
            .await
            .unwrap()
    } else {
        piped_output.clone() // Fallback
    };
    let tty_text = String::from_utf8_lossy(&tty_output.stdout);

    println!("=== PIPE vs TTY COMPARISON ===");
    println!("Piped output (current TaskHub behavior):");
    println!("'{}'", piped_text);
    println!("TTY output (expected behavior):");
    println!("'{}'", tty_text);

    // Count lines - piped will typically have more lines
    let piped_lines = piped_text.lines().count();
    let tty_lines = tty_text.lines().count();

    println!("Piped lines: {}, TTY lines: {}", piped_lines, tty_lines);

    // This test documents the expected difference but doesn't assert
    // It's mainly for demonstration and debugging purposes
    assert!(piped_lines >= 1, "Should have at least some output");

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}
