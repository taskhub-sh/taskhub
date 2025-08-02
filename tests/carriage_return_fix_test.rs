use taskhub::tui::views::terminal::process_output_with_carriage_returns;

#[test]
fn test_crlf_line_endings_preserved() {
    // Test that \r\n (Windows line endings) are treated as normal line endings
    // and don't get stripped by carriage return processing
    let input = "Hello World\r\n";
    let result = process_output_with_carriage_returns(input);

    assert_eq!(result, vec!["Hello World"]);
    assert!(!result.is_empty());
    assert_ne!(result[0], "");
}

#[test]
fn test_multi_line_crlf_preserved() {
    // Test multiple lines with \r\n endings
    let input = "Line 1\r\nLine 2\r\nLine 3\r\n";
    let result = process_output_with_carriage_returns(input);

    assert_eq!(result, vec!["Line 1", "Line 2", "Line 3"]);
}

#[test]
fn test_standalone_cr_still_overwrites() {
    // Test that standalone \r still works for progress bars
    let input = "Progress: 50%\rProgress: 100%";
    let result = process_output_with_carriage_returns(input);

    // Should only show the final state after carriage return
    assert_eq!(result, vec!["Progress: 100%"]);
}

#[test]
fn test_mixed_line_endings() {
    // Test mix of \r\n and standalone \r
    let input = "Line 1\r\nProgress: 0%\rProgress: 50%\rProgress: 100%\nLine 2\r\n";
    let result = process_output_with_carriage_returns(input);

    assert_eq!(result, vec!["Line 1", "Progress: 100%", "Line 2"]);
}

#[test]
fn test_echo_output_like_user_scenario() {
    // Test output similar to what the user is experiencing
    // This simulates the actual output from an echo command with \r\n
    let input = "Hello World\r\n";
    let result = process_output_with_carriage_returns(input);

    // Before the fix, this would return [""] (empty string)
    // After the fix, this should return ["Hello World"]
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], "Hello World");
    assert!(!result[0].is_empty());
}

#[test]
fn test_ls_output_like_user_scenario() {
    // Test ls-like output that might have \r\n line endings
    let input = "file1.txt\r\nfile2.txt\r\ndirectory/\r\n";
    let result = process_output_with_carriage_returns(input);

    assert_eq!(result, vec!["file1.txt", "file2.txt", "directory/"]);
    assert!(!result.iter().any(|line| line.is_empty()));
}
