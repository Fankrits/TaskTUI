# Performance and Lightweight Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform TaskTUI into a high-performance, lightweight TUI application with near-zero idle CPU usage, minimal memory allocations, efficient kernel polling, and compact binary size.

**Architecture:** Implement reactive dirty-flag rendering and adaptive event polling; eliminate per-frame cloning in top process rankings; replace sweeping `sysinfo::refresh_all()` with targeted metric refreshes; make network socket scanning tab-aware and throttled; optimize release profile with LTO and symbol stripping.

**Tech Stack:** Rust (Edition 2024), Ratatui 0.30.2, Crossterm 0.29.0, Sysinfo 0.39.6, Netstat2 0.11.2

## Global Constraints
- Preserve all existing keyboard shortcuts, mouse gestures, views, modals, filters, and features.
- Avoid introducing heavy external crates; use std library and existing dependencies.
- Ensure strict zero-regression compatibility on macOS, Linux, and Windows.
- Keep codebase clean, idiomatic, and maintainable.

---

### Task 1: Cargo Release Profile & Binary Optimization

**Files:**
- Modify: `Cargo.toml:1-13`

**Interfaces:**
- Produces: Optimized release profile configuration with LTO, codegen-units, panic abort, and stripping.

- [ ] **Step 1: Check existing release profile**

Run: `cargo build --release`
Expected: Default unoptimized release build (large binary size).

- [ ] **Step 2: Add release profile configuration to `Cargo.toml`**

Add the following to `Cargo.toml`:
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

- [ ] **Step 3: Verify optimized release compilation**

Run: `cargo build --release`
Expected: Build succeeds with smaller binary size.

- [ ] **Step 4: Commit release profile optimization**

```bash
git add Cargo.toml
git commit -m "perf: configure release profile with LTO, abort panic, and symbol stripping"
```

---

### Task 2: Reactive Event Loop & Dirty-Flag Rendering (Zero Idle CPU)

**Files:**
- Modify: `src/main.rs:36-55`
- Modify: `src/event.rs:5-20`
- Modify: `src/event.rs:279-311`

**Interfaces:**
- Consumes: `crossterm::event::poll`, `crossterm::event::read`
- Produces: `event::handle_events(app: &mut App) -> Result<bool, anyhow::Error>` returning whether state changed and redraw is required.

- [ ] **Step 1: Update `event::handle_events` to return a dirty flag**

Modify `src/event.rs`:
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

Update `handle_mouse_event` in `src/event.rs` to return `bool` (`true` on meaningful clicks/scrolls, `false` on ignored mouse motion/hover):
```rust
fn handle_mouse_event(app: &mut App, mouse: MouseEvent) -> bool {
    // Return true when a scroll or click action occurs, false on ignored motion
...
```

- [ ] **Step 2: Update Main Loop in `src/main.rs` for Reactive Rendering**

Modify `src/main.rs` main loop:
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

- [ ] **Step 3: Verify build and test event loop responsiveness**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit reactive rendering**

```bash
git add src/main.rs src/event.rs
git commit -m "perf: implement reactive event loop and dirty-flag rendering to eliminate idle CPU usage"
```

---

### Task 3: Zero-Allocation Top Process Rankings & Rendering Optimization

**Files:**
- Modify: `src/app.rs:581-592`
- Modify: `src/ui/graphs.rs:46-48, 260-383`

**Interfaces:**
- Consumes: `app.monitor.processes: Vec<ProcessInfo>`
- Produces: `app.get_top_cpu_processes(count: usize) -> Vec<&ProcessInfo>`, `app.get_top_memory_processes(count: usize) -> Vec<&ProcessInfo>` returning borrowed references without cloning the entire 500+ vector on every frame.

- [ ] **Step 1: Refactor `get_top_cpu_processes` and `get_top_memory_processes` in `src/app.rs`**

Modify `src/app.rs`:
```rust
    pub fn get_top_cpu_processes(&self, count: usize) -> Vec<&ProcessInfo> {
        let mut list: Vec<&ProcessInfo> = self.monitor.processes.iter().collect();
        if list.len() > count {
            list.select_nth_unstable_by(count, |a, b| {
                b.cpu_usage
                    .partial_cmp(&a.cpu_usage)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            list.truncate(count);
        }
        list.sort_by(|a, b| {
            b.cpu_usage
                .partial_cmp(&a.cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        list
    }

    pub fn get_top_memory_processes(&self, count: usize) -> Vec<&ProcessInfo> {
        let mut list: Vec<&ProcessInfo> = self.monitor.processes.iter().collect();
        if list.len() > count {
            list.select_nth_unstable_by(count, |a, b| b.memory_bytes.cmp(&a.memory_bytes));
            list.truncate(count);
        }
        list.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
        list
    }
```

- [ ] **Step 2: Update `src/ui/graphs.rs` to accept `&ProcessInfo` references**

Update `render_top_cpu_rank` and `render_top_memory_rank` signatures and usage in `src/ui/graphs.rs` to take `&[&ProcessInfo]` or work directly with references.

- [ ] **Step 3: Verify compilation and tests**

Run: `cargo check`
Expected: PASS with 0 errors.

- [ ] **Step 4: Commit zero-allocation rankings**

```bash
git add src/app.rs src/ui/graphs.rs
git commit -m "perf: eliminate vector cloning in top cpu and memory process rankings"
```

---

### Task 4: Targeted `sysinfo` Polling & Cached Metadata

**Files:**
- Modify: `src/system.rs:54-93`
- Modify: `src/system.rs:151-304`

**Interfaces:**
- Consumes: `sysinfo::System`, `sysinfo::ProcessRefreshKind`, `sysinfo::ProcessesToUpdate`
- Produces: Fine-grained process, CPU, and memory refreshes without full kernel scans.

- [ ] **Step 1: Update `SystemMonitor::refresh` in `src/system.rs`**

Replace broad `sys.refresh_all()` with targeted calls:
```rust
    pub fn refresh(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refresh).as_secs_f64().max(0.1);
        self.last_refresh = now;

        // Targeted CPU & Memory Refresh
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            sysinfo::ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_disk_usage()
                .with_user(sysinfo::UpdateKind::OnlyIfNotSet),
        );
        self.networks.refresh(true);
```

- [ ] **Step 2: Optimize disk and user list caching**

Only refresh `disks` every 15 seconds instead of every 1 second.
Cache user IDs in a persistent `HashMap<u32, String>` across ticks instead of allocating a fresh hash map on every tick.

- [ ] **Step 3: Verify build and runtime integrity**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit targeted sysinfo polling**

```bash
git add src/system.rs
git commit -m "perf: replace broad refresh_all with targeted sysinfo refresh and cached user mappings"
```

---

### Task 5: Tab-Aware and Throttled Network Socket Polling

**Files:**
- Modify: `src/system.rs:54-92, 151-215`
- Modify: `src/app.rs:194-205`

**Interfaces:**
- Consumes: `netstat2::get_sockets_info`
- Produces: Conditional/throttled socket scanning based on elapsed time and active tab.

- [ ] **Step 1: Add socket refresh timer in `SystemMonitor`**

Add `last_socket_refresh: Instant` to `SystemMonitor`.
In `refresh_sockets(force: bool)`: Only invoke `netstat2::get_sockets_info` if `force` or `last_socket_refresh.elapsed() >= Duration::from_secs(3)`.

- [ ] **Step 2: Update `App::on_tick` and tab switching**

In `app.rs`:
- When switching to `Tab::NetworkPorts`, trigger an immediate socket refresh.
- During regular ticks, pass tab awareness so sockets are refreshed every 3s instead of every 1s.

- [ ] **Step 3: Verify compilation and tests**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit throttled socket scanning**

```bash
git add src/system.rs src/app.rs
git commit -m "perf: throttle netstat socket queries and make collection tab-aware"
```

---

### Task 6: Comprehensive Verification & Benchmark Validation

**Files:**
- All modified files

- [ ] **Step 1: Run full debug and release builds**

Run: `cargo build` and `cargo build --release`
Expected: Both exit with code 0.

- [ ] **Step 2: Validate feature completeness**
Verify that all operations function seamlessly:
- Process table sorting (m/c/p/n/o), filtering (/), and scrolling (j/k, PgUp/PgDn, mouse wheel)
- Modal inspection (Enter / d) and process termination (k / K)
- Network ports tab filtering (L) and sorting (o/s/p)
- System details disk and interface rendering
- Help view and keyboard cheatsheet
- Responsive resizing

- [ ] **Step 3: Final Commit**

```bash
git commit -m "chore: complete performance and lightweight optimization overhaul"
```
