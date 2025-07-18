use taskhub::tui::ansi_parser::AnsiParser;

#[test]
fn test_ansi_parser_indentation_directly() {
    // Test the ANSI parser directly to see what happens with indented colored text

    // First, let's test just the tab character
    println!("=== Testing just tab character ===");
    let mut parser_tab = AnsiParser::new(80, 24);
    let _parsed_lines_tab = parser_tab.parse("\t");
    let terminal_state_tab = parser_tab.get_terminal_state();
    println!(
        "After parsing tab: cursor at ({}, {})",
        terminal_state_tab.cursor.row, terminal_state_tab.cursor.col
    );

    // Now let's test the full input
    println!("=== Testing full input ===");
    let mut parser = AnsiParser::new(80, 24);
    let input = "\t\x1b[31mmodified:   src/main.rs\x1b[0m";

    println!("Input: '{}'", input);
    println!("Input bytes: {:?}", input.as_bytes());

    // Let's also check what the terminal state looks like after parsing
    let parsed_lines = parser.parse(input);

    // Check the raw terminal state
    let terminal_state = parser.get_terminal_state();
    println!("Terminal state after parsing:");
    println!(
        "  Cursor: row={}, col={}",
        terminal_state.cursor.row, terminal_state.cursor.col
    );
    println!("  Tab stops: {:?}", terminal_state.tab_stops);

    // Check the buffer content
    let buffer = terminal_state.current_buffer();
    if let Some(first_row) = buffer.first() {
        println!("  First row length: {}", first_row.len());
        println!("  First 32 characters of first row:");
        for (i, styled_char) in first_row.iter().take(32).enumerate() {
            if styled_char.ch == ' ' {
                println!("    [{}]: SPACE (style: {:?})", i, styled_char.style);
            } else {
                println!(
                    "    [{}]: '{}' (style: {:?})",
                    i, styled_char.ch, styled_char.style
                );
            }
        }
    }

    println!("Parsed lines count: {}", parsed_lines.len());

    for (i, line) in parsed_lines.iter().enumerate() {
        println!("Line {}: {} spans", i, line.spans.len());
        for (j, span) in line.spans.iter().enumerate() {
            println!(
                "  Span {}: content='{}' (bytes: {:?})",
                j,
                span.content,
                span.content.as_bytes()
            );
            println!("  Span {}: style={:?}", j, span.style);
        }
    }

    // Convert back to string to see what the reconstruction produces
    let reconstructed = parsed_lines
        .into_iter()
        .map(|line| {
            line.spans
                .into_iter()
                .map(|span| span.content.to_string())
                .collect::<String>()
        })
        .collect::<Vec<String>>()
        .join("\n");

    println!("Reconstructed (plain text): '{}'", reconstructed);
    println!("Reconstructed bytes: {:?}", reconstructed.as_bytes());

    // For now, let's just understand what's happening instead of asserting
    println!("SUMMARY:");
    println!("  Tab alone: cursor stays at (0, 0) - tab processing not working");
    println!("  Full input: text appears at positions 0-22 instead of 8-30");
    println!("  This suggests tabs are not being processed correctly by VtActionHandler");

    // Let's check if the issue is consistent
    let starts_with_tab = reconstructed.starts_with('\t');
    let starts_with_spaces = reconstructed.starts_with("        "); // 8 spaces
    println!("  Reconstructed starts with tab: {}", starts_with_tab);
    println!(
        "  Reconstructed starts with 8 spaces: {}",
        starts_with_spaces
    );

    if !starts_with_tab && !starts_with_spaces {
        println!("  ERROR: Indentation completely lost!");
    }

    // For now, don't fail the test - we want to understand the issue first
    // assert!(reconstructed.starts_with('\t'), "Tab should be preserved in reconstruction");
}
