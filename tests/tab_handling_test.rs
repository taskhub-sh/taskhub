use taskhub::tui::ansi_parser::AnsiParser;

#[test]
fn test_tab_character_handling() {
    let mut parser = AnsiParser::new(80, 24);

    // Test simple tab character
    let tab_input = "\tHello";
    let lines = parser.parse(tab_input);
    assert_eq!(lines.len(), 1);

    let line_text = lines[0].to_string();
    println!("Tab input '{}' parsed as: '{}'", tab_input, line_text);

    // The line should start with spaces (representing the tab)
    assert!(
        line_text.starts_with("        "),
        "Line should start with 8 spaces for tab, got: '{}'",
        line_text
    );
    assert!(
        line_text.contains("Hello"),
        "Line should contain 'Hello', got: '{}'",
        line_text
    );
}

#[test]
fn test_tab_with_color() {
    let mut parser = AnsiParser::new(80, 24);

    // Test tab with colored text (like git status)
    let colored_tab = "\t\x1b[31mmodified: file.txt\x1b[0m";
    let lines = parser.parse(colored_tab);
    assert_eq!(lines.len(), 1);

    let line_text = lines[0].to_string();
    println!("Colored tab input parsed as: '{}'", line_text);

    // The line should start with spaces (representing the tab)
    assert!(
        line_text.starts_with("        "),
        "Line should start with 8 spaces for tab, got: '{}'",
        line_text
    );
    assert!(
        line_text.contains("modified: file.txt"),
        "Line should contain the text, got: '{}'",
        line_text
    );
}

#[test]
fn test_multiple_tabs() {
    let mut parser = AnsiParser::new(80, 24);

    // Test multiple tabs
    let multi_tab = "\t\tDouble tab";
    let lines = parser.parse(multi_tab);
    assert_eq!(lines.len(), 1);

    let line_text = lines[0].to_string();
    println!("Multi-tab input parsed as: '{}'", line_text);

    // Two tabs should result in 16 spaces
    assert!(
        line_text.starts_with("                "),
        "Line should start with 16 spaces for two tabs, got: '{}'",
        line_text
    );
    assert!(
        line_text.contains("Double tab"),
        "Line should contain the text, got: '{}'",
        line_text
    );
}
