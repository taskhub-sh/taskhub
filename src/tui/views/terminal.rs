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
    pub selection_start: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
    pub input_selection_start: Option<usize>,
    pub input_selection_end: Option<usize>,
    pub auto_suggestion: Option<&'a str>,
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
        draw_command_history(
            f,
            chunks[0],
            state.command_history,
            state.scroll_offset,
            state.selection_start,
            state.selection_end,
        );

        // Command list area
        draw_command_list(
            f,
            chunks[1],
            state.filtered_commands,
            state.selected_command_index,
        );

        // Input area
        draw_input_box(f, chunks[2], state);
    } else {
        // Normal two-area layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        // Command history area
        draw_command_history(
            f,
            chunks[0],
            state.command_history,
            state.scroll_offset,
            state.selection_start,
            state.selection_end,
        );

        // Input area
        draw_input_box(f, chunks[1], state);
    }
}

fn draw_command_history(
    f: &mut Frame<'_>,
    area: Rect,
    command_history: &[CommandEntry],
    scroll_offset: usize,
    selection_start: Option<(usize, usize)>,
    selection_end: Option<(usize, usize)>,
) {
    // Create all history items first
    let mut all_items = Vec::new();
    let mut line_index = 0;

    // Determine selection bounds
    let selection_bounds = if let (Some(start), Some(end)) = (selection_start, selection_end) {
        // Ensure start is before end
        if start.0 <= end.0 || (start.0 == end.0 && start.1 <= end.1) {
            Some((start, end))
        } else {
            Some((end, start))
        }
    } else {
        None
    };

    for entry in command_history.iter() {
        // Add command line
        let command_style = if entry.success {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Red)
        };

        // Check if this line is selected
        let line_item =
            if let Some(((start_line, start_col), (end_line, end_col))) = selection_bounds {
                if line_index >= start_line && line_index <= end_line {
                    // This line is within selection
                    let command_text = format!("> {}", entry.command);
                    if line_index == start_line && line_index == end_line {
                        // Single line selection
                        create_selected_line(command_text, start_col, end_col, command_style)
                    } else if line_index == start_line {
                        // Start of multi-line selection
                        let len = command_text.len();
                        create_selected_line(command_text, start_col, len, command_style)
                    } else if line_index == end_line {
                        // End of multi-line selection
                        create_selected_line(command_text, 0, end_col, command_style)
                    } else {
                        // Fully selected line
                        let len = command_text.len();
                        create_selected_line(command_text, 0, len, command_style)
                    }
                } else {
                    // Not selected
                    ListItem::new(Line::from(vec![
                        Span::styled("> ", Style::default().fg(Color::Cyan)),
                        Span::styled(&entry.command, command_style),
                    ]))
                }
            } else {
                // No selection
                ListItem::new(Line::from(vec![
                    Span::styled("> ", Style::default().fg(Color::Cyan)),
                    Span::styled(&entry.command, command_style),
                ]))
            };

        all_items.push(line_item);
        line_index += 1;

        // Add output lines
        if !entry.output.is_empty() {
            let output_style = if entry.output == "Running..." {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };

            for line in entry.output.lines() {
                // Check if this line is selected
                let line_item = if let Some(((start_line, start_col), (end_line, end_col))) =
                    selection_bounds
                {
                    if line_index >= start_line && line_index <= end_line {
                        // This line is within selection
                        if line_index == start_line && line_index == end_line {
                            // Single line selection
                            create_selected_line(line.to_string(), start_col, end_col, output_style)
                        } else if line_index == start_line {
                            // Start of multi-line selection
                            let len = line.len();
                            create_selected_line(line.to_string(), start_col, len, output_style)
                        } else if line_index == end_line {
                            // End of multi-line selection
                            create_selected_line(line.to_string(), 0, end_col, output_style)
                        } else {
                            // Fully selected line
                            let len = line.len();
                            create_selected_line(line.to_string(), 0, len, output_style)
                        }
                    } else {
                        // Not selected
                        ListItem::new(Line::from(Span::styled(line, output_style)))
                    }
                } else {
                    // No selection
                    ListItem::new(Line::from(Span::styled(line, output_style)))
                };

                all_items.push(line_item);
                line_index += 1;
            }
        }

        // Add empty line for spacing
        all_items.push(ListItem::new(Line::from("")));
        line_index += 1;
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
        "Terminal Output (Use Shift+↑↓ to scroll, ↑↓ for history, Home/End for top/bottom)"
            .to_string()
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

fn draw_input_box(f: &mut Frame<'_>, area: Rect, state: &TerminalDisplayState<'_>) {
    let chars: Vec<char> = state.current_input.chars().collect();
    let cursor_pos = state.cursor_position.min(chars.len());

    let prompt_style = if state.is_command_running {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };

    let input_text = create_input_line_with_selection(
        state.prompt,
        prompt_style,
        &chars,
        cursor_pos,
        state.input_selection_start,
        state.input_selection_end,
        state.auto_suggestion,
    );

    let title = if state.is_command_running {
        "Command Input (Command running... Press Ctrl-C to stop)"
    } else if state.current_input.starts_with('/') {
        "Command Input (Type to filter commands)"
    } else if state.auto_suggestion.is_some() {
        "Command Input (Tab to accept all, Right arrow for next char, / for commands)"
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

fn create_selected_line(
    text: String,
    start_col: usize,
    end_col: usize,
    base_style: Style,
) -> ListItem<'static> {
    let chars: Vec<char> = text.chars().collect();
    let mut spans = Vec::new();

    // Add text before selection
    if start_col > 0 && start_col <= chars.len() {
        let before_text: String = chars[..start_col].iter().collect();
        spans.push(Span::styled(before_text, base_style));
    }

    // Add selected text with highlight
    let selection_start = start_col.min(chars.len());
    let selection_end = end_col.min(chars.len());
    if selection_start < selection_end {
        let selected_text: String = chars[selection_start..selection_end].iter().collect();
        spans.push(Span::styled(
            selected_text,
            base_style.bg(Color::Blue).fg(Color::White),
        ));
    }

    // Add text after selection
    if end_col < chars.len() {
        let after_text: String = chars[end_col..].iter().collect();
        spans.push(Span::styled(after_text, base_style));
    }

    ListItem::new(Line::from(spans))
}

fn create_input_line_with_selection<'a>(
    prompt: &str,
    prompt_style: Style,
    chars: &[char],
    cursor_pos: usize,
    selection_start: Option<usize>,
    selection_end: Option<usize>,
    auto_suggestion: Option<&'a str>,
) -> Line<'a> {
    let mut spans = vec![Span::styled(format!("{prompt} "), prompt_style)];

    // Check if there's a selection
    if let (Some(start), Some(end)) = (selection_start, selection_end) {
        let sel_start = start.min(end).min(chars.len());
        let sel_end = start.max(end).min(chars.len());

        // Add text before selection
        if sel_start > 0 {
            let before_text: String = chars[..sel_start].iter().collect();
            spans.push(Span::raw(before_text));
        }

        // Add selected text with highlight
        if sel_start < sel_end {
            let selected_text: String = chars[sel_start..sel_end].iter().collect();
            spans.push(Span::styled(
                selected_text,
                Style::default().bg(Color::Blue).fg(Color::White),
            ));
        }

        // Add text after selection
        if sel_end < chars.len() {
            let after_text: String = chars[sel_end..].iter().collect();
            spans.push(Span::raw(after_text));
        }

        // Add cursor if not within selection
        if cursor_pos < sel_start || cursor_pos >= sel_end {
            if cursor_pos < chars.len() {
                // We need to insert cursor character - this is complex with selections
                // For now, just add a simple cursor at the end
                spans.push(Span::styled(" ", Style::default().bg(Color::White)));
            } else {
                spans.push(Span::styled(" ", Style::default().bg(Color::White)));
            }
        }
    } else {
        // No selection, show normal cursor
        if cursor_pos < chars.len() {
            let before_cursor: String = chars[..cursor_pos].iter().collect();
            let cursor_char = chars[cursor_pos];
            let after_cursor: String = chars[(cursor_pos + 1)..].iter().collect();

            spans.push(Span::raw(before_cursor));
            spans.push(Span::styled(
                cursor_char.to_string(),
                Style::default().bg(Color::White).fg(Color::Black),
            ));
            spans.push(Span::raw(after_cursor));
        } else {
            let input_text: String = chars.iter().collect();
            spans.push(Span::raw(input_text.clone()));

            // Show auto-suggestion as grayed-out text if available
            if let Some(suggestion) = auto_suggestion {
                if cursor_pos == chars.len()
                    && !input_text.is_empty()
                    && suggestion.starts_with(&input_text)
                {
                    // Show the remaining part of the suggestion
                    let remaining = &suggestion[input_text.len()..];
                    if !remaining.is_empty() {
                        let remaining_chars: Vec<char> = remaining.chars().collect();
                        if !remaining_chars.is_empty() {
                            // Show first character with subtle cursor (gray background)
                            spans.push(Span::styled(
                                remaining_chars[0].to_string(),
                                Style::default().bg(Color::Gray).fg(Color::Black),
                            ));

                            // Show rest of suggestion in dark gray
                            if remaining_chars.len() > 1 {
                                let rest: String = remaining_chars[1..].iter().collect();
                                spans
                                    .push(Span::styled(rest, Style::default().fg(Color::DarkGray)));
                            }
                        }
                    } else {
                        // No remaining suggestion, show normal block cursor
                        spans.push(Span::styled(" ", Style::default().bg(Color::White)));
                    }
                } else {
                    // No suggestion, show normal block cursor
                    spans.push(Span::styled(" ", Style::default().bg(Color::White)));
                }
            } else {
                // No suggestion, show normal block cursor
                spans.push(Span::styled(" ", Style::default().bg(Color::White)));
            }
        }
    }

    Line::from(spans)
}
