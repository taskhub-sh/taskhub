use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

#[derive(Debug, Clone)]
pub struct CommandEntry {
    pub command: String,
    pub output: String,
    pub timestamp: String,
    pub success: bool,
}

pub fn draw_terminal(
    f: &mut Frame<'_>,
    area: Rect,
    command_history: &[CommandEntry],
    current_input: &str,
    cursor_position: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    // Command history area
    draw_command_history(f, chunks[0], command_history);

    // Input area
    draw_input_box(f, chunks[1], current_input, cursor_position);
}

fn draw_command_history(f: &mut Frame<'_>, area: Rect, command_history: &[CommandEntry]) {
    let mut items = Vec::new();

    for entry in command_history.iter().rev().take(100) {
        // Show last 100 commands
        // Add command line
        let command_style = if entry.success {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Red)
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("{}> ", entry.timestamp),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(&entry.command, command_style),
        ])));

        // Add output lines
        if !entry.output.is_empty() {
            for line in entry.output.lines() {
                items.push(ListItem::new(Line::from(Span::styled(
                    line,
                    Style::default().fg(Color::White),
                ))));
            }
        }

        // Add empty line for spacing
        items.push(ListItem::new(Line::from("")));
    }

    let list = List::new(items).block(
        Block::default()
            .title("Terminal Output")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );

    f.render_widget(list, area);
}

fn draw_input_box(f: &mut Frame<'_>, area: Rect, current_input: &str, cursor_position: usize) {
    let chars: Vec<char> = current_input.chars().collect();
    let cursor_pos = cursor_position.min(chars.len());

    let input_text = if cursor_pos < chars.len() {
        let before_cursor: String = chars[..cursor_pos].iter().collect();
        let cursor_char = chars[cursor_pos];
        let after_cursor: String = chars[(cursor_pos + 1)..].iter().collect();

        Line::from(vec![
            Span::raw(before_cursor),
            Span::styled(
                cursor_char.to_string(),
                Style::default().bg(Color::White).fg(Color::Black),
            ),
            Span::raw(after_cursor),
        ])
    } else {
        Line::from(vec![
            Span::raw(current_input),
            Span::styled(" ", Style::default().bg(Color::White)),
        ])
    };

    let input = Paragraph::new(input_text).block(
        Block::default()
            .title("Command Input (Press Esc to exit terminal mode)")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(input, area);
}
