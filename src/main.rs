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

    // Main App Loop
    while !app.should_quit {
        terminal.draw(|f| ui::render(f, &mut app))?;

        // Handle key and mouse events
        event::handle_events(&mut app)?;

        // Tick update timer
        if last_tick.elapsed() >= app.tick_rate {
            app.on_tick();
            last_tick = Instant::now();
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
