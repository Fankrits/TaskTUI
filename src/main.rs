mod app;
mod event;
mod system;
mod theme;
mod ui;

use app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::panic;
use std::time::Instant;

fn main() -> Result<(), anyhow::Error> {
    // Setup panic hook to ensure terminal is restored properly
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Create App instance
    let mut app = App::new();

    let mut last_tick = Instant::now();
    let mut needs_redraw = true;

    // Main App Loop
    while !app.should_quit {
        if needs_redraw {
            terminal.draw(|f| ui::render(f, &mut app))?;
            needs_redraw = false;
        }

        // Calculate exact remaining time until next tick
        let timeout = app.tick_rate.saturating_sub(last_tick.elapsed());
        if crossterm::event::poll(timeout)? && event::handle_events(&mut app)? {
            needs_redraw = true;
        }

        // Tick update timer
        if last_tick.elapsed() >= app.tick_rate {
            app.on_tick();
            last_tick = Instant::now();
            needs_redraw = true;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    println!("Task TUI exited successfully.");
    Ok(())
}
