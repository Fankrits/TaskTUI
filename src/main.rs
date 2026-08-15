mod app;
mod cli;
mod event;
mod system;
mod theme;
mod ui;

use app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::stdout;
use std::panic;
use std::time::Instant;

fn main() -> Result<(), anyhow::Error> {
    // Process CLI arguments (help, version, update)
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "-h" | "--help" | "help" => {
                cli::print_help();
                return Ok(());
            }
            "-v" | "-V" | "--version" | "version" => {
                cli::print_version();
                return Ok(());
            }
            "update" | "upgrade" => {
                return cli::run_update();
            }
            "uninstall" | "--uninstall" => {
                let skip_confirm = args.iter().any(|a| a == "-y" || a == "--yes");
                return cli::run_uninstall(skip_confirm);
            }
            unknown => {
                eprintln!("\x1b[1;31mError:\x1b[0m Unknown argument: {unknown}");
                eprintln!("Run `tasktui --help` for usage instructions.");
                std::process::exit(1);
            }
        }
    }

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
