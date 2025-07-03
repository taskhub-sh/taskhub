use crate::db::models::Task;
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Cell, Row, Table},
};

pub fn draw_task_list(f: &mut Frame<'_>, area: Rect, tasks: &[Task]) {
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
        ratatui::layout::Constraint::Percentage(15),
        ratatui::layout::Constraint::Percentage(40),
        ratatui::layout::Constraint::Percentage(15),
        ratatui::layout::Constraint::Percentage(15),
        ratatui::layout::Constraint::Percentage(15),
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
                .title("Tasks (Press 't' for terminal mode, 'q' to quit)")
                .borders(Borders::ALL),
        );

    f.render_widget(table, area);
}
