use crossterm::event::{self, Event, KeyCode};
use ratatui::Terminal;
use std::io;
use std::path::PathBuf;
use taskhub::config::settings::Settings;
use taskhub::db::init_db;
use taskhub::tui::app::{App, AppMode};
use taskhub::tui::views::task_list::draw_task_list;
use taskhub::tui::views::terminal::draw_terminal;
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
        // Handle any pending commands
        app.handle_pending_commands().await;

        terminal.draw(|f| {
            let size = f.area();
            match app.mode {
                AppMode::TaskList => {
                    draw_task_list(f, size, &app.tasks);
                }
                AppMode::Terminal => {
                    draw_terminal(
                        f,
                        size,
                        &app.command_history,
                        &app.current_input,
                        app.cursor_position,
                    );
                }
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char(c) => {
                        app.on_key(c);
                    }
                    other_key => {
                        app.on_key_code(other_key);
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
