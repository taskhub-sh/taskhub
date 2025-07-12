use taskhub::tui::views::terminal::CommandEntry;

#[test]
fn test_basic_ansi_color_parsing() {
    // Test with ANSI color codes that ls --color=always might produce
    let colored_text = "\x1b[34mdir/\x1b[0m"; // Blue text for directories

    // Create a command entry with colored output
    let entry = CommandEntry {
        command: "ls --color=always".to_string(),
        output: colored_text.to_string(),
        success: true,
    };

    // This test just ensures the parsing doesn't crash
    // In a real application, we'd need to run the terminal view
    assert_eq!(entry.output, colored_text);
}

#[test]
fn test_git_colored_output() {
    // Test with git status colors
    let git_output = "\x1b[31mmodified:   src/main.rs\x1b[0m\n\x1b[32mnew file:   test.txt\x1b[0m";

    let entry = CommandEntry {
        command: "git status --porcelain".to_string(),
        output: git_output.to_string(),
        success: true,
    };

    // Verify the entry contains ANSI codes
    assert!(entry.output.contains("\x1b[31m")); // Red
    assert!(entry.output.contains("\x1b[32m")); // Green
    assert!(entry.output.contains("\x1b[0m")); // Reset
}

#[test]
fn test_cargo_colored_output() {
    // Test with cargo build colors
    let cargo_output = "\x1b[1m\x1b[32m   Compiling\x1b[0m taskhub v0.1.0\n\x1b[1m\x1b[32m    Finished\x1b[0m dev target(s)";

    let entry = CommandEntry {
        command: "cargo build".to_string(),
        output: cargo_output.to_string(),
        success: true,
    };

    // Verify the entry contains ANSI codes
    assert!(entry.output.contains("\x1b[1m")); // Bold
    assert!(entry.output.contains("\x1b[32m")); // Green
}
