### Task 5: Tab-Aware and Throttled Network Socket Polling

**Files:**
- Modify: `src/system.rs:54-92, 151-240`
- Modify: `src/app.rs:194-205`

**Interfaces:**
- Consumes: `netstat2::get_sockets_info`
- Produces: Throttled socket scanning (every 3 seconds or on tab switch/force) to avoid expensive OS socket scans every single second.

- [ ] **Step 1: Add socket refresh timer in `SystemMonitor`**
In `src/system.rs`:
- Add `pub last_socket_refresh: Instant` and `pub cached_pid_ports: HashMap<u32, Vec<u16>>` to `SystemMonitor`.
- In `refresh()`: Only scan sockets if `last_socket_refresh.elapsed() >= Duration::from_secs(3)` or if `force_socket_refresh` is true. Otherwise reuse `cached_pid_ports` and cached `sockets`.
- Add a method `pub fn refresh_sockets(&mut self)` to allow immediate socket refresh on demand.

- [ ] **Step 2: Update `App` tab switching and `on_tick`**
In `src/app.rs`:
- When switching tabs to `Tab::NetworkPorts`, call `self.monitor.refresh_sockets()` and `self.apply_network_filter_and_sort()`.
- During `on_tick()`, avoid unnecessary re-filtering of network sockets if sockets were not updated.

- [ ] **Step 3: Add unit tests in `src/system.rs` or `src/app.rs`**
Add unit test verifying socket collection throttling.

- [ ] **Step 4: Verification and Commit**
Run: `cargo check` and `cargo test`
Commit:
```bash
git add src/system.rs src/app.rs
git commit -m "perf: throttle netstat socket queries and make collection tab-aware"
```
