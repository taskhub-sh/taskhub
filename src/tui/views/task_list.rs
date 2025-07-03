use crate::db::models::Task;
use crate::tui::views::terminal::TerminalDisplayState;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
};

pub fn draw_task_list(
    f: &mut Frame<'_>,
    area: Rect,
    tasks: &[Task],
    state: &TerminalDisplayState<'_>,
) {
    if state.show_command_list {
        // Split into three areas: tasks, command list, input
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(state.filtered_commands.len().min(8) as u16 + 2),
                Constraint::Length(3),
            ])
            .split(area);

        // Task list area
        draw_tasks_table(f, chunks[0], tasks);

        // Command list area
        draw_command_list_in_task_view(
            f,
            chunks[1],
            state.filtered_commands,
            state.selected_command_index,
        );

        // Input area
        draw_input_box_in_task_view(f, chunks[2], state.current_input, state.cursor_position);
    } else {
        // Normal two-area layout: tasks and input
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        // Task list area
        draw_tasks_table(f, chunks[0], tasks);

        // Input area
        draw_input_box_in_task_view(f, chunks[1], state.current_input, state.cursor_position);
    }
}

fn draw_tasks_table(f: &mut Frame<'_>, area: Rect, tasks: &[Task]) {
    let rows: Vec<Row> = tasks
        .iter()
        .map(|task| {
            Row::new(vec![
                Cell::from(task.id.to_string()),
                Cell::from(task.title.clone()),
                Cell::from(task.source.to_string()),
                Cell::from(task.status.to_string()),
                Cell::from(task.priority.to_string()),
            ])
        })
        .collect();

    let widths = &[
        Constraint::Percentage(15),
        Constraint::Percentage(40),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec![
                Cell::from("ID"),
                Cell::from("Title"),
                Cell::from("Source"),
                Cell::from("Status"),
                Cell::from("Priority"),
            ])
            .bottom_margin(1),
        )
        .block(
            Block::default()
                .title("Tasks (Use /task add <title> to add new tasks)")
                .borders(Borders::ALL),
        );

    f.render_widget(table, area);
}

fn draw_command_list_in_task_view(
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

fn draw_input_box_in_task_view(
    f: &mut Frame<'_>,
    area: Rect,
    current_input: &str,
    cursor_position: usize,
) {
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

    let title = if current_input.starts_with('/') {
        "Command Input (Type to filter commands)"
    } else {
        "Command Input (Type / for commands, 'q' to return to terminal)"
    };

    let input = Paragraph::new(input_text).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(input, area);
}
