# Task 4 Implementation Report: Targeted `sysinfo` Polling & Cached Metadata

## Overview
Replaced full OS kernel refresh calls (`sys.refresh_all()`) with fine-grained targeted subsystem refresh calls, added persistent UID-to-username caching to prevent redundant map rebuilds, and throttled disk queries from every 1 second tick to every 15 seconds.

## Changes Implemented

### 1. Targeted Subsystem Refresh
- In `src/system.rs`, replaced `self.sys.refresh_all()` with:
  - `self.sys.refresh_cpu_all()`
  - `self.sys.refresh_memory()`
  - `self.sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu().with_memory().with_disk_usage().with_user(UpdateKind::OnlyIfNotSet))`
- This drastically reduces kernel scan overhead and unnecessary telemetry collection on each tick.

### 2. Disk Refresh Throttling
- Added `last_disk_refresh: Instant` field to `SystemMonitor`.
- Throttled `self.disks.refresh(true)` so it only queries filesystem status if `last_disk_refresh.elapsed() >= 15s`.

### 3. Persistent User Metadata Cache
- Added `user_cache: HashMap<sysinfo::Uid, String>` to `SystemMonitor`.
- Eliminated temporary per-tick `pid_user_map: HashMap<u32, String>` construction. Process iterations now look up UIDs directly in `user_cache`, querying `Users` and populating the cache only on cache miss.

### 4. Unit Tests Added
Added 5 comprehensive unit tests to `src/system.rs`:
- `test_system_monitor_initialization`: Verifies system metadata and ring buffer initialization.
- `test_system_monitor_refresh_updates_metrics`: Verifies `refresh()` updates timestamps and metric histories.
- `test_user_cache_populated_and_persisted`: Verifies UID-to-user mappings are cached and persist across refreshes.
- `test_disk_refresh_throttling`: Verifies disk refresh is throttled within the 15-second window and executes when threshold is exceeded.
- `test_get_disks_info`: Verifies disk metadata extraction.

## Verification & Test Results
- `cargo check`: Passed with 0 errors.
- `cargo test`: 13 passed (5 new tests in `system::tests`, 8 existing tests in `app` and `event`).

## Git Commit
- **Commit SHA**: `17a79bb3919e3b24f8f5130711804174a37e9d19`
- **Commit Message**: `perf: replace broad refresh_all with targeted sysinfo refresh and cached user mappings`
