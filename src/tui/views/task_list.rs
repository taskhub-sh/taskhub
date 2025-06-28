use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Widget},
    Frame,
};

pub fn draw_task_list(f: &mut Frame<'_>, area: Rect) {
    let block = Block::default().title("Tasks").borders(Borders::ALL);
    f.render_widget(block, area);
}
