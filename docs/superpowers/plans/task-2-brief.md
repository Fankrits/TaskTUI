### Task 2: Reactive Event Loop & Dirty-Flag Rendering (Zero Idle CPU)

**Files:**
- Modify: `src/main.rs:36-55`
- Modify: `src/event.rs:5-20`
- Modify: `src/event.rs:279-517`

**Interfaces:**
- Consumes: `crossterm::event::poll`, `crossterm::event::read`
- Produces: `event::handle_events(app: &mut App) -> Result<bool, anyhow::Error>` returning whether an event occurred that modified state and requires a UI redraw.

- [ ] **Step 1: Update `event::handle_events` and `handle_mouse_event` in `src/event.rs`**
Update `event::handle_events` to return `Result<bool, anyhow::Error>`:
```rust
pub fn handle_events(app: &mut App) -> Result<bool, anyhow::Error> {
    let mut handled = false;
    while event::poll(Duration::from_millis(0))? {
        match event::read()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    handle_key_event(app, key);
                    handled = true;
                }
            }
            Event::Mouse(mouse) => {
                if handle_mouse_event(app, mouse) {
                    handled = true;
                }
            }
            Event::Resize(_, _) => {
                handled = true;
            }
            _ => {}
        }
    }
    Ok(handled)
}
```
Update `handle_mouse_event` to return `bool`:
- `true` when a scroll wheel action occurs
- `true` when a click is processed (on table rows, tabs, header buttons, modals)
- `false` for mouse move / hover events that do not alter state.

- [ ] **Step 2: Update Main Loop in `src/main.rs` for Reactive Rendering**
Modify `src/main.rs`:
```rust
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
        if crossterm::event::poll(timeout)? {
            if event::handle_events(&mut app)? {
                needs_redraw = true;
            }
        }

        // Tick update timer
        if last_tick.elapsed() >= app.tick_rate {
            app.on_tick();
            last_tick = Instant::now();
            needs_redraw = true;
        }
    }
```

- [ ] **Step 3: Verification**
Run: `cargo check` and `cargo test`
Ensure smooth handling of keypresses, mouse events, and resize events.

- [ ] **Step 4: Commit**
```bash
git add src/main.rs src/event.rs
git commit -m "perf: implement reactive event loop and dirty-flag rendering to eliminate idle CPU usage"
```
