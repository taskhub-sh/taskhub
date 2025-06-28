use crossterm::event::{self, Event, KeyCode};
use ratatui::Terminal;
use std::io;
use taskhub::tui::app::App;
use taskhub::tui::{cleanup_terminal, setup_terminal};
use taskhub::tui::views::task_list::draw_task_list;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = setup_terminal()?;
    let mut app = App::new();
    run_app(&mut terminal, &mut app).await?;
    cleanup_terminal(&mut terminal)?;
    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            let size = f.size();
            draw_task_list(f, size);
        })?;

        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char(c) = key.code {
                    app.on_key(c);
                }
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

