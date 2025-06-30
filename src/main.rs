use crossterm::event::{self, Event, KeyCode};
use ratatui::Terminal;
use std::io;
use std::path::PathBuf;
use taskhub::config::settings::Settings;
use taskhub::db::init_db;
use taskhub::tui::app::App;
use taskhub::tui::views::task_list::draw_task_list;
use taskhub::tui::{cleanup_terminal, setup_terminal};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::new()?;
    let db_path = settings.database_path.map(PathBuf::from);
    let db_pool = init_db(db_path).await?;
    let mut terminal = setup_terminal()?;
    let mut app = App::new(db_pool);
    app.load_tasks().await?;
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
            let size = f.area();
            draw_task_list(f, size, &app.tasks);
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
