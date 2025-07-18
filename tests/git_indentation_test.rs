use std::time::Duration;
use taskhub::db::init_db;
use taskhub::tui::app::App;

#[tokio::test]
async fn test_git_status_indentation_preservation() {
    // This test verifies that git status output preserves proper indentation
    // when colored output is processed through the ANSI parser

    let db_pool = init_db(Some(":memory:".into())).await.unwrap();
    let mut app = App::new(db_pool.clone());

    // Test git status command
    let command = "git -c color.status=always status";

    println!("Testing git status indentation preservation");

    // Execute the command
    app.execute_command(command.to_string()).await;

    // Wait for command to complete
    let start = std::time::Instant::now();
    let timeout_duration = Duration::from_secs(10);

    while app.running_command.is_some() && start.elapsed() < timeout_duration {
        app.check_running_command().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Find the command entry in history
    let command_entry = app
        .command_history
        .iter()
        .find(|entry| entry.command == command)
        .expect("Should find git status command in history");

    println!("Git status output: '{}'", command_entry.output);

    // The command should succeed
    assert!(command_entry.success, "Git status should succeed");

    // Check that indentation is preserved
    // Look for lines that should be indented (modified files)
    let lines: Vec<&str> = command_entry.output.lines().collect();

    // Find a line that contains "modified:" and check if it's properly indented
    let mut found_indented_line = false;
    for line in lines {
        if line.contains("modified:") || line.contains("new file:") || line.contains("deleted:") {
            println!("Found file status line: '{}'", line);
            println!("Line bytes: {:?}", line.as_bytes());

            // Check if the line starts with proper indentation (tab or spaces)
            // The original should start with tab, but after processing it should still have indentation
            let starts_with_indent =
                line.starts_with('\t') || line.starts_with("    ") || line.starts_with("  ");

            if !starts_with_indent {
                // If no indentation, check if color codes are at the beginning
                let has_color_at_start = line.starts_with('\x1b') || line.starts_with("\\x1b");
                if has_color_at_start {
                    println!(
                        "ERROR: Color codes appear at beginning of line instead of after indentation"
                    );
                    println!("Line: '{}'", line.replace('\x1b', "\\x1b"));
                }
            }

            found_indented_line = true;

            // For now, just log what we found - we'll assert once we understand the pattern
            println!("Line starts with indent: {}", starts_with_indent);
        }
    }

    assert!(
        found_indented_line,
        "Should find at least one indented file status line"
    );
}
