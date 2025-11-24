mod app;
mod config;
mod ssh;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

fn main() -> Result<()> {
    let mut app = App::new()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend + std::io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                if app.is_form_active() {
                    handle_form_input(app, key.code, key.modifiers)?;
                } else if handle_normal_input(terminal, app, key.code)? {
                    return Ok(());
                }
            }
        }
    }
}

fn handle_normal_input<B: ratatui::backend::Backend + std::io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    code: KeyCode,
) -> Result<bool> {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => {
            return Ok(true);
        }
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        KeyCode::Char('i') => {
            app.enter_edit_mode();
        }
        KeyCode::Char('n') => {
            app.enter_new_mode();
        }
        KeyCode::Enter => {
            if let Some(host) = app.selected_host_name() {
                let host = host.to_string();
                disable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                )?;
                terminal.show_cursor()?;

                let result = app.connect_to_host(&host);

                enable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    EnterAlternateScreen,
                    EnableMouseCapture
                )?;
                terminal.clear()?;

                app.set_status(result);
            }
        }
        _ => {}
    }
    Ok(false)
}

fn handle_form_input(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Result<()> {
    match code {
        KeyCode::Esc => {
            app.cancel_form();
        }
        KeyCode::Enter => {
            app.save_form();
        }
        KeyCode::Tab => {
            if modifiers.contains(KeyModifiers::SHIFT) {
                app.focus_previous_field();
            } else {
                app.focus_next_field();
            }
        }
        KeyCode::BackTab => {
            app.focus_previous_field();
        }
        KeyCode::Down => app.focus_next_field(),
        KeyCode::Up => app.focus_previous_field(),
        KeyCode::Backspace | KeyCode::Delete => {
            app.handle_form_backspace();
        }
        KeyCode::Char(c) => {
            if modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(());
            }
            app.handle_form_input(c);
        }
        _ => {}
    }
    Ok(())
}
