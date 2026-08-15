# Task 5 Implementation Report: Tab-Aware and Throttled Network Socket Polling

## Overview
Optimized network socket scanning by throttling expensive `netstat2` queries from running every 1-second tick to every 3 seconds (or immediately on tab switch to Network Ports / manual F5 refresh), caching port-to-PID mappings and socket entries across ticks.

## Changes Implemented

### 1. Socket Refresh Throttling in `SystemMonitor`
- Added `pub last_socket_refresh: Instant` and `pub cached_pid_ports: HashMap<u32, Vec<u16>>` to [`SystemMonitor`](file:///Users/fankrits/dev/TaskTUI/src/system.rs#L54-L64).
- In [`refresh()`](file:///Users/fankrits/dev/TaskTUI/src/system.rs#L161-L245), socket collection is only invoked when `last_socket_refresh.elapsed() >= Duration::from_secs(3)` or if `self.sockets` is empty. Otherwise, existing `cached_pid_ports` and `sockets` are reused across ticks.
- Added [`pub fn refresh_sockets(&mut self)`](file:///Users/fankrits/dev/TaskTUI/src/system.rs#L338-L365) to force immediate socket scanning, updating `cached_pid_ports`, `sockets`, and mapping socket process names & process port lists.

### 2. Tab-Aware Socket Polling & Filtering in `App` and `Event`
- Added [`pub fn switch_tab(&mut self, tab: Tab)`](file:///Users/fankrits/dev/TaskTUI/src/app.rs#L194-L201) to [`App`](file:///Users/fankrits/dev/TaskTUI/src/app.rs#L117-L148) which immediately triggers `self.monitor.refresh_sockets()` and `self.apply_network_filter_and_sort()` whenever switching to `Tab::NetworkPorts`.
- Updated [`on_tick()`](file:///Users/fankrits/dev/TaskTUI/src/app.rs#L203-L215) in [`App`](file:///Users/fankrits/dev/TaskTUI/src/app.rs#L117-L148) to avoid redundant network socket filtering unless `last_socket_refresh` has changed.
- Updated keyboard and mouse tab navigation in [`src/event.rs`](file:///Users/fankrits/dev/TaskTUI/src/event.rs) to use `app.switch_tab(...)`.
- Added immediate `app.monitor.refresh_sockets()` trigger on `F5` manual refresh.

### 3. Unit Tests Added
- Added unit tests in `src/system.rs`:
  - `test_socket_refresh_throttling`: Verifies that rapid `refresh()` calls do not re-scan sockets, but elapsed time >= 3s triggers a rescan.
  - `test_force_refresh_sockets`: Verifies `refresh_sockets()` forces immediate socket query and timestamp update.
- Added unit tests in `src/app.rs`:
  - `test_switch_tab_to_network_ports_triggers_socket_refresh`: Verifies tab switch to `Tab::NetworkPorts` triggers immediate socket refresh.
  - `test_switch_tab_to_other_tabs_does_not_force_socket_refresh`: Verifies switching to other tabs does not unnecessarily query sockets.
  - `test_on_tick_throttles_socket_refresh`: Verifies regular tick execution respects socket throttling.

## Verification & Test Results
- `cargo check`: Passed with 0 warnings/errors.
- `cargo test`: 18 passed (0 failed, 0 ignored).

## Git Commit
- **Commit SHA**: `afc81e6a874e07776ad7c2c3ba71c9e850b17a90`
- **Commit Message**: `perf: throttle netstat socket queries and make collection tab-aware`
