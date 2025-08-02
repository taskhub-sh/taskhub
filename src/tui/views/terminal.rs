use crate::tui::ansi_parser::AnsiParser;
use crossterm;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

struct HistoryRenderState<'a> {
    scroll_offset: usize,
    selection_start: Option<(usize, usize)>,
    selection_end: Option<(usize, usize)>,
    search_matches: &'a [(usize, usize, usize)],
    current_search_match: usize,
}

/// Create a ListItem with vtparse ANSI parsing
fn create_vtparse_parsed_line(text: &str, fallback_style: Style) -> ListItem<'static> {
    // Expand tab characters to spaces (using 8-space tab stops)
    let expanded_text = expand_tabs(text, 8);

    // Use vtparse to parse ANSI sequences with actual terminal width
    let (term_width, _) = crossterm::terminal::size().unwrap_or((120, 24));
    let mut parser = AnsiParser::new(term_width as usize, 1); // Single line parser
    let parsed_lines = parser.parse(&expanded_text);

    if let Some(parsed_line) = parsed_lines.first() {
        // Use the parsed line with proper ANSI handling
        ListItem::new(parsed_line.clone())
    } else {
        // Fallback to styled text
        ListItem::new(Line::from(Span::styled(expanded_text, fallback_style)))
    }
}

/// Expand tab characters to spaces using the specified tab width
fn expand_tabs(text: &str, tab_width: usize) -> String {
    let mut result = String::new();
    let mut column = 0;

    for ch in text.chars() {
        if ch == '\t' {
            // Calculate how many spaces to add to reach the next tab stop
            let spaces_to_add = tab_width - (column % tab_width);
            result.push_str(&" ".repeat(spaces_to_add));
            column += spaces_to_add;
        } else {
            result.push(ch);
            column += 1;
        }
    }

    result
}

/// Process output text to handle carriage returns properly for progress bars
/// Carriage returns (\r) should overwrite the current line, not create new lines
pub fn process_output_with_carriage_returns(output: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for ch in output.chars() {
        match ch {
            '\n' => {
                // Newline: commit current line and start a new one
                lines.push(current_line.clone());
                current_line.clear();
            }
            '\r' => {
                // Carriage return: reset current line (overwrite mode)
                current_line.clear();
            }
            _ => {
                // Regular character: add to current line
                current_line.push(ch);
            }
        }
    }

    // Add the final line if it's not empty
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    // If we have no lines but the input wasn't empty, add at least one empty line
    if lines.is_empty() && !output.is_empty() {
        lines.push(String::new());
    }

    lines
}

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
    pub reverse_search_active: bool,
    pub reverse_search_prompt: &'a str,
    pub current_search_result: Option<&'a str>,
    pub output_search_active: bool,
    pub output_search_query: &'a str,
    pub output_search_matches: &'a [(usize, usize, usize)],
    pub output_search_current_match: usize,
    pub output_search_status: &'a str,
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
        let history_state = HistoryRenderState {
            scroll_offset: state.scroll_offset,
            selection_start: state.selection_start,
            selection_end: state.selection_end,
            search_matches: state.output_search_matches,
            current_search_match: state.output_search_current_match,
        };
        draw_command_history(f, chunks[0], state.command_history, &history_state);

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
        let history_state = HistoryRenderState {
            scroll_offset: state.scroll_offset,
            selection_start: state.selection_start,
            selection_end: state.selection_end,
            search_matches: state.output_search_matches,
            current_search_match: state.output_search_current_match,
        };
        draw_command_history(f, chunks[0], state.command_history, &history_state);

        // Input area
        draw_input_box(f, chunks[1], state);
    }
}

fn draw_command_history(
    f: &mut Frame<'_>,
    area: Rect,
    command_history: &[CommandEntry],
    render_state: &HistoryRenderState<'_>,
) {
    // Create all history items first
    let mut all_items = Vec::new();
    let mut line_index = 0;

    // Determine selection bounds
    let selection_bounds = if let (Some(start), Some(end)) =
        (render_state.selection_start, render_state.selection_end)
    {
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

        // Check if this line has search matches
        let search_matches_for_line: Vec<(usize, (usize, usize, usize))> = render_state
            .search_matches
            .iter()
            .enumerate()
            .filter(|(_, (match_line, _, _))| *match_line == line_index)
            .map(|(idx, (line, start, end))| (idx, (*line, *start, *end)))
            .collect();

        // Check if this line is selected
        let line_item = if !search_matches_for_line.is_empty() {
            // This line has search matches, create with highlighting
            let command_text = format!("> {}", entry.command);
            create_line_with_search_highlights(
                command_text,
                &search_matches_for_line,
                render_state.current_search_match,
                command_style,
                selection_bounds,
                line_index,
            )
        } else if let Some(((start_line, start_col), (end_line, end_col))) = selection_bounds {
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

            // Split output handling carriage returns for progress bars
            let processed_lines = process_output_with_carriage_returns(&entry.output);
            for line in processed_lines {
                // Check if this line has search matches
                let search_matches_for_line: Vec<(usize, (usize, usize, usize))> = render_state
                    .search_matches
                    .iter()
                    .enumerate()
                    .filter(|(_, (match_line, _, _))| *match_line == line_index)
                    .map(|(idx, (line, start, end))| (idx, (*line, *start, *end)))
                    .collect();

                // Check if this line is selected
                let line_item = if !search_matches_for_line.is_empty() {
                    // This line has search matches, create with highlighting
                    create_line_with_search_highlights(
                        line.to_string(),
                        &search_matches_for_line,
                        render_state.current_search_match,
                        output_style,
                        selection_bounds,
                        line_index,
                    )
                } else if let Some(((start_line, start_col), (end_line, end_col))) =
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
                        // Not selected - use vtparse ANSI parsing
                        create_vtparse_parsed_line(&line, output_style)
                    }
                } else {
                    // No selection - use vtparse ANSI parsing
                    create_vtparse_parsed_line(&line, output_style)
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
        let start_index = if render_state.scroll_offset >= total_items {
            0
        } else {
            total_items.saturating_sub(available_height + render_state.scroll_offset)
        };

        let end_index = if render_state.scroll_offset == 0 {
            total_items
        } else {
            total_items.saturating_sub(render_state.scroll_offset)
        };

        all_items[start_index..end_index].to_vec()
    };

    // Create scroll indicator text
    let scroll_info = if render_state.scroll_offset > 0 {
        format!(
            "Terminal Output (↑{} lines scrolled)",
            render_state.scroll_offset
        )
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

    let input_text = if state.output_search_active {
        create_output_search_line(state.output_search_query, state.output_search_status)
    } else if state.reverse_search_active {
        create_reverse_search_line(state.reverse_search_prompt, state.current_search_result)
    } else {
        create_input_line_with_selection(
            state.prompt,
            prompt_style,
            &chars,
            cursor_pos,
            state.input_selection_start,
            state.input_selection_end,
            state.auto_suggestion,
        )
    };

    let title = if state.output_search_active {
        "Output Search (Type to search, ↑↓ to navigate, Tab for mode, Enter/Esc to exit)"
    } else if state.reverse_search_active {
        "Reverse Search (Enter to accept, Esc to cancel, ↑↓ to navigate)"
    } else if state.is_command_running {
        "Command Input (Command running... Press Ctrl-C to stop)"
    } else if state.current_input.starts_with('/') {
        "Command Input (Type to filter commands)"
    } else if state.auto_suggestion.is_some() {
        "Command Input (Tab to accept all, Right arrow for next char, / for commands)"
    } else {
        "Command Input (Type / for commands, /quit to exit, Ctrl-R for search, Ctrl-F for output search)"
    };

    let border_style = if state.output_search_active {
        Style::default().fg(Color::Cyan)
    } else if state.reverse_search_active {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let input = Paragraph::new(input_text).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style),
    );

    f.render_widget(input, area);
}

fn create_selected_line(
    text: String,
    start_col: usize,
    end_col: usize,
    base_style: Style,
) -> ListItem<'static> {
    // Expand tab characters to spaces first
    let expanded_text = expand_tabs(&text, 8);
    let chars: Vec<char> = expanded_text.chars().collect();
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

fn create_reverse_search_line<'a>(
    search_prompt: &'a str,
    current_result: Option<&'a str>,
) -> Line<'a> {
    let mut spans = vec![Span::styled(
        search_prompt,
        Style::default().fg(Color::Magenta),
    )];

    if let Some(result) = current_result {
        spans.push(Span::styled(
            format!(" {result}"),
            Style::default().fg(Color::Green),
        ));
    }

    Line::from(spans)
}

fn create_output_search_line<'a>(search_query: &'a str, search_status: &'a str) -> Line<'a> {
    let prompt = if search_status.is_empty() {
        format!("Search: {search_query}")
    } else {
        search_status.to_string()
    };
    Line::from(vec![Span::styled(prompt, Style::default().fg(Color::Cyan))])
}

fn create_line_with_search_highlights(
    text: String,
    search_matches: &[(usize, (usize, usize, usize))],
    current_search_match: usize,
    base_style: Style,
    _selection_bounds: Option<((usize, usize), (usize, usize))>,
    _line_index: usize,
) -> ListItem<'static> {
    // Expand tab characters to spaces first
    let expanded_text = expand_tabs(&text, 8);
    let chars: Vec<char> = expanded_text.chars().collect();
    let mut spans = Vec::new();
    let mut pos = 0;

    // Collect all match ranges for this line
    let mut match_ranges: Vec<(usize, usize, bool)> = search_matches
        .iter()
        .map(|(match_idx, (_, start_col, end_col))| {
            (*start_col, *end_col, *match_idx == current_search_match)
        })
        .collect();

    // Sort by start position
    match_ranges.sort_by_key(|(start, _, _)| *start);

    for (start_col, end_col, is_current) in match_ranges {
        // Add text before match
        if pos < start_col && start_col <= chars.len() {
            let before_text: String = chars[pos..start_col].iter().collect();
            spans.push(Span::styled(before_text, base_style));
        }

        // Add highlighted match
        let match_start = start_col.min(chars.len());
        let match_end = end_col.min(chars.len());
        if match_start < match_end {
            let match_text: String = chars[match_start..match_end].iter().collect();
            let highlight_style = if is_current {
                // Current match: bright yellow background
                base_style.bg(Color::Yellow).fg(Color::Black)
            } else {
                // Other matches: cyan background
                base_style.bg(Color::Cyan).fg(Color::Black)
            };
            spans.push(Span::styled(match_text, highlight_style));
        }

        pos = match_end;
    }

    // Add remaining text after last match
    if pos < chars.len() {
        let remaining_text: String = chars[pos..].iter().collect();
        spans.push(Span::styled(remaining_text, base_style));
    }

    // Handle selection highlighting over search highlights if needed
    // For simplicity, we'll prioritize search highlighting over selection
    // A more sophisticated implementation could layer both

    ListItem::new(Line::from(spans))
}
