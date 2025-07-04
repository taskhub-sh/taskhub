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

pub struct TerminalDisplayState<'a> {
    pub command_history: &'a [CommandEntry],
    pub current_input: &'a str,
    pub cursor_position: usize,
    pub scroll_offset: usize,
    pub show_command_list: bool,
    pub filtered_commands: &'a [String],
    pub selected_command_index: usize,
    pub is_command_running: bool,
    pub prompt: &'a str,
}

pub fn draw_terminal(f: &mut Frame<'_>, area: Rect, state: &TerminalDisplayState<'_>) {
    if state.show_command_list {
        // Split into three areas: history, command list, input
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(state.filtered_commands.len().min(8) as u16 + 2),
                Constraint::Length(3),
            ])
            .split(area);

        // Command history area
        draw_command_history(f, chunks[0], state.command_history, state.scroll_offset);

        // Command list area
        draw_command_list(
            f,
            chunks[1],
            state.filtered_commands,
            state.selected_command_index,
        );

        // Input area
        draw_input_box(
            f,
            chunks[2],
            state.current_input,
            state.cursor_position,
            state.is_command_running,
            state.prompt,
        );
    } else {
        // Normal two-area layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        // Command history area
        draw_command_history(f, chunks[0], state.command_history, state.scroll_offset);

        // Input area
        draw_input_box(
            f,
            chunks[1],
            state.current_input,
            state.cursor_position,
            state.is_command_running,
            state.prompt,
        );
    }
}

fn draw_command_history(
    f: &mut Frame<'_>,
    area: Rect,
    command_history: &[CommandEntry],
    scroll_offset: usize,
) {
    // Create all history items first
    let mut all_items = Vec::new();

    for entry in command_history.iter() {
        // Add command line
        let command_style = if entry.success {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Red)
        };

        all_items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("{}> ", entry.timestamp),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(&entry.command, command_style),
        ])));

        // Add output lines
        if !entry.output.is_empty() {
            let output_style = if entry.output == "Running..." {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };

            for line in entry.output.lines() {
                all_items.push(ListItem::new(Line::from(Span::styled(line, output_style))));
            }
        }

        // Add empty line for spacing
        all_items.push(ListItem::new(Line::from("")));
    }

    // Calculate available height (subtract 2 for borders)
    let available_height = area.height.saturating_sub(2) as usize;

    // Calculate which items to show based on scroll offset
    let total_items = all_items.len();
    let visible_items = if total_items <= available_height {
        // All items fit, show them all
        all_items
    } else {
        // Need to scroll - show from bottom up with offset
        let start_index = if scroll_offset >= total_items {
            0
        } else {
            total_items.saturating_sub(available_height + scroll_offset)
        };

        let end_index = if scroll_offset == 0 {
            total_items
        } else {
            total_items.saturating_sub(scroll_offset)
        };

        all_items[start_index..end_index].to_vec()
    };

    // Create scroll indicator text
    let scroll_info = if scroll_offset > 0 {
        format!("Terminal Output (↑{scroll_offset} lines scrolled)")
    } else if total_items > available_height {
        "Terminal Output (Use ↑↓ to scroll, Home/End for top/bottom)".to_string()
    } else {
        "Terminal Output".to_string()
    };

    let list = List::new(visible_items).block(
        Block::default()
            .title(scroll_info)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );

    f.render_widget(list, area);
}

fn draw_command_list(
    f: &mut Frame<'_>,
    area: Rect,
    filtered_commands: &[String],
    selected_index: usize,
) {
    let items: Vec<ListItem> = filtered_commands
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let style = if i == selected_index {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default().fg(Color::Green)
            };
            ListItem::new(Line::from(Span::styled(cmd, style)))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("Commands (↑↓ to navigate, Enter to select, Esc to cancel)")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    );

    f.render_widget(list, area);
}

fn draw_input_box(
    f: &mut Frame<'_>,
    area: Rect,
    current_input: &str,
    cursor_position: usize,
    is_command_running: bool,
    prompt: &str,
) {
    let chars: Vec<char> = current_input.chars().collect();
    let cursor_pos = cursor_position.min(chars.len());

    let prompt_style = if is_command_running {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };

    let input_text = if cursor_pos < chars.len() {
        let before_cursor: String = chars[..cursor_pos].iter().collect();
        let cursor_char = chars[cursor_pos];
        let after_cursor: String = chars[(cursor_pos + 1)..].iter().collect();

        Line::from(vec![
            Span::styled(format!("{prompt} "), prompt_style),
            Span::raw(before_cursor),
            Span::styled(
                cursor_char.to_string(),
                Style::default().bg(Color::White).fg(Color::Black),
            ),
            Span::raw(after_cursor),
        ])
    } else {
        Line::from(vec![
            Span::styled(format!("{prompt} "), prompt_style),
            Span::raw(current_input),
            Span::styled(" ", Style::default().bg(Color::White)),
        ])
    };

    let title = if is_command_running {
        "Command Input (Command running... Press Ctrl-C to stop)"
    } else if current_input.starts_with('/') {
        "Command Input (Type to filter commands)"
    } else {
        "Command Input (Type / for commands, /quit to exit)"
    };

    let input = Paragraph::new(input_text).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(input, area);
}
