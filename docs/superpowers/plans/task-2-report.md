# Task 2 Report: Reactive Event Loop & Dirty-Flag Rendering (Zero Idle CPU)

## Overview
Implemented a reactive event polling and dirty-flag rendering architecture for TaskTUI. Previously, the main loop rendered continuously without sleeping, causing unnecessary CPU cycles at idle. With this optimization, TaskTUI now sleeps for the remainder of its tick duration using `crossterm::event::poll(timeout)`, and only triggers terminal redraws (`terminal.draw(...)`) when a state-changing event occurs or when the periodic tick timer fires.

## Key Changes

### 1. `src/event.rs`
- **`handle_events(app: &mut App) -> Result<bool, anyhow::Error>`**:
  - Drains all pending events from the crossterm event queue using `while event::poll(Duration::from_millis(0))?`.
  - Tracks whether any processed event changed the app state (`handled = true`).
  - Handles `KeyEventKind::Press`, `Resize`, and meaningful `MouseEvent`s.
- **`handle_mouse_event(app: &mut App, mouse: MouseEvent) -> bool`**:
  - Returns `true` when user interactions alter application state:
    - Mouse scroll wheel navigation (up / down).
    - Right click process termination prompt.
    - Modal button clicks and modal dismissal.
    - Header navigation clicks (tabs, play/pause toggle, host info).
    - Dashboard graph/ranking view cycling.
    - Search bar activation.
    - Table column header sort toggle.
    - Table row selection and details modal opening.
    - Footer navigation and quit/help actions.
  - Returns `false` on non-state-changing events (such as mouse hover/motion).
- **Unit Tests**:
  - Added comprehensive unit tests in `src/event.rs` verifying scroll handling, mouse movement filtering, header/dashboard click interactions, tab navigation, and quit handling.

### 2. `src/main.rs`
- Introduced `needs_redraw: bool` initialized to `true`.
- Calculates exact remaining time until the next tick:
  ```rust
  let timeout = app.tick_rate.saturating_sub(last_tick.elapsed());
  if crossterm::event::poll(timeout)? {
      if event::handle_events(&mut app)? {
          needs_redraw = true;
      }
  }
  ```
- Redraws the UI only when `needs_redraw` is `true`.
- Advances `app.on_tick()` and sets `needs_redraw = true` when `last_tick.elapsed() >= app.tick_rate`.

## Verification & Test Results
- **`cargo check`**: Passed with 0 errors / warnings.
- **`cargo test`**: 5 unit tests executed and passed:
  ```
  running 5 tests
  test event::tests::test_mouse_click_header_and_dashboard ... ok
  test event::tests::test_key_tab_navigation ... ok
  test event::tests::test_mouse_scroll_triggers_redraw ... ok
  test event::tests::test_mouse_move_ignored ... ok
  test event::tests::test_key_quit ... ok

  test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
  ```

## Git Commit
- **Commit SHA**: `4e8a626bc0a4de89cc3eaf3c96d728ba11bd574b`
- **Commit Message**: `perf: implement reactive event loop and dirty-flag rendering to eliminate idle CPU usage`
