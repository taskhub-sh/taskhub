use color_eyre::Result;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    text::Line,
    widgets::Paragraph,
};
use std::env;
use std::fs;

// Add import for the ANSI parser from the tui module
use taskhub::tui::ansi_parser::AnsiParser;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Get command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        std::process::exit(1);
    }

    // Read file content
    let file_path = &args[1];
    let file_content = fs::read_to_string(file_path)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to read file '{}': {}", file_path, e))?;

    let terminal = ratatui::init();
    let app_result = run(terminal, file_content);
    ratatui::restore();
    app_result
}

fn run(mut terminal: DefaultTerminal, ansi_content: String) -> Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, &ansi_content))?;
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(());
            }
        }
    }
}

fn draw(frame: &mut Frame, ansi_content: &str) {
    // Use ANSI parser to parse the file content
    let mut parser = AnsiParser::new_with_terminal_size();
    let lines = parser.parse(ansi_content);

    // Use the parsed lines, or create a fallback
    let display_lines = if lines.is_empty() {
        vec![Line::from("No content to display")]
    } else {
        lines
    };

    // For now, just display the first line
    let line = display_lines
        .into_iter()
        .next()
        .unwrap_or_else(|| Line::from("Empty"));
    let paragraph = Paragraph::new(line);

    frame.render_widget(paragraph, frame.area());
}
