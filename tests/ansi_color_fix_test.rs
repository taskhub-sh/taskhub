use taskhub::tui::ansi_parser::AnsiParser;

#[test]
fn test_ansi_color_parsing_fix() {
    // Test with the exact sequence from lolcat-2.txt that was failing
    let test_input = "\x1b[38;2;95;37;250mL\x1b[39m\x1b[38;2;91;40;251mo\x1b[39m\x1b[38;2;87;44;252mr\x1b[39m\x1b[38;2;83;47;253me\x1b[39m";

    let mut parser = AnsiParser::new(80, 1);

    // Test the public parse_line_with_vtparse method
    let parsed_line = parser.parse_line_with_vtparse(test_input);

    // The text content should be "Lore"
    assert_eq!(parsed_line.to_string(), "Lore");

    // Test with the full parse method as well
    let parsed_lines = parser.parse(test_input);
    assert_eq!(parsed_lines.len(), 1);
    assert_eq!(parsed_lines[0].to_string(), "Lore");
}

#[test]
fn test_simple_ansi_color() {
    let test_input = "\x1b[31mRed text\x1b[0m";

    let mut parser = AnsiParser::new(80, 1);
    let parsed_line = parser.parse_line_with_vtparse(test_input);

    // Should preserve the text content
    assert_eq!(parsed_line.to_string(), "Red text");
}
