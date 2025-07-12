use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::Terminal;
use std::io;
use std::path::PathBuf;
use taskhub::config::settings::Settings;
use taskhub::db::init_db;
use taskhub::tui::app::{App, AppMode};
use taskhub::tui::views::task_list::draw_task_list;
use taskhub::tui::views::terminal::{TerminalDisplayState, draw_terminal};
use taskhub::tui::{cleanup_terminal, setup_terminal};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::new().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let db_path = settings.database_path.map(PathBuf::from);
    let db_pool = init_db(db_path)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let mut terminal = setup_terminal()?;

    // Create app with history manager if persistence is enabled
    let mut app = if settings.history.persist {
        App::new(db_pool.clone()).with_history_manager(Some(settings.history.max_entries))
    } else {
        App::new(db_pool)
    };

    // Load persistent history if enabled
    app.load_persistent_history().await;
    app.load_tasks()
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let result = run_app(&mut terminal, &mut app).await;

    // Save history before exiting
    app.save_persistent_history().await;

    cleanup_terminal(&mut terminal)?;
    result?;
    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        // Handle any pending commands
        app.handle_pending_commands().await;

        // Check if any running command has finished
        app.check_running_command().await;

        // Update spinner animation if command is running
        app.update_spinner();

        terminal.draw(|f| {
            let size = f.area();
            app.set_terminal_area_height(size.height);

            // Update layout areas for accurate mouse coordinate mapping
            let command_list_size = if app.show_command_list {
                app.get_filtered_commands().len().min(8) as u16 + 2
            } else {
                0
            };
            app.update_layout_areas(size.height, app.show_command_list, command_list_size);

            match app.mode {
                AppMode::TaskList => {
                    let filtered_commands = app.get_filtered_commands();
                    let reverse_search_prompt = app.get_reverse_search_prompt();
                    let state = TerminalDisplayState {
                        command_history: &app.command_history,
                        current_input: &app.current_input,
                        cursor_position: app.cursor_position,
                        scroll_offset: app.scroll_offset,
                        show_command_list: app.show_command_list,
                        filtered_commands: &filtered_commands,
                        selected_command_index: app.selected_command_index,
                        is_command_running: app.running_command.is_some(),
                        prompt: app.get_prompt(),
                        selection_start: app.selection_start,
                        selection_end: app.selection_end,
                        input_selection_start: app.input_selection_start,
                        input_selection_end: app.input_selection_end,
                        auto_suggestion: app.auto_suggestion.as_deref(),
                        reverse_search_active: app.reverse_search_active,
                        reverse_search_prompt: &reverse_search_prompt,
                        current_search_result: app.get_current_search_result().map(|x| x.as_str()),
                        output_search_active: app.output_search_active,
                        output_search_query: &app.output_search_query,
                        output_search_matches: app.get_output_search_matches(),
                        output_search_current_match: app.get_current_search_match(),
                        output_search_status: &app.get_output_search_status(),
                    };
                    draw_task_list(f, size, &app.tasks, &state);
                }
                AppMode::Terminal => {
                    let filtered_commands = app.get_filtered_commands();
                    let reverse_search_prompt = app.get_reverse_search_prompt();
                    let state = TerminalDisplayState {
                        command_history: &app.command_history,
                        current_input: &app.current_input,
                        cursor_position: app.cursor_position,
                        scroll_offset: app.scroll_offset,
                        show_command_list: app.show_command_list,
                        filtered_commands: &filtered_commands,
                        selected_command_index: app.selected_command_index,
                        is_command_running: app.running_command.is_some(),
                        prompt: app.get_prompt(),
                        selection_start: app.selection_start,
                        selection_end: app.selection_end,
                        input_selection_start: app.input_selection_start,
                        input_selection_end: app.input_selection_end,
                        auto_suggestion: app.auto_suggestion.as_deref(),
                        reverse_search_active: app.reverse_search_active,
                        reverse_search_prompt: &reverse_search_prompt,
                        current_search_result: app.get_current_search_result().map(|x| x.as_str()),
                        output_search_active: app.output_search_active,
                        output_search_query: &app.output_search_query,
                        output_search_matches: app.get_output_search_matches(),
                        output_search_current_match: app.get_current_search_match(),
                        output_search_status: &app.get_output_search_status(),
                    };
                    draw_terminal(f, size, &state);
                }
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    // Handle Ctrl-C to kill running commands
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        // Check if there's any text selection first
                        if (app.selection_start.is_some() && app.selection_end.is_some())
                            || (app.input_selection_start.is_some()
                                && app.input_selection_end.is_some())
                        {
                            let _ = app.copy_selected_text();
                        } else {
                            app.kill_running_command().await;
                        }
                    } else if key.code == KeyCode::Char('v')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        // Handle Ctrl-V for paste
                        let _ = app.paste_from_clipboard();
                    } else if key.code == KeyCode::Char('r')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        // Handle Ctrl-R for reverse search
                        app.on_key_code(key.code, key.modifiers);
                    } else {
                        match key.code {
                            KeyCode::Char(c) => {
                                // For character input, we should handle Shift as normal since
                                // crossterm already provides the correct uppercase/lowercase char.
                                // Only treat as special modifier key if it's something other than Shift.
                                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT
                                {
                                    app.on_key(c);
                                } else {
                                    app.on_key_code(key.code, key.modifiers);
                                }
                            }
                            other_key => {
                                app.on_key_code(other_key, key.modifiers);
                            }
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    app.on_mouse_event(mouse);
                }
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
