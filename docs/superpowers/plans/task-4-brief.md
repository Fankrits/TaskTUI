### Task 4: Targeted `sysinfo` Polling & Cached Metadata

**Files:**
- Modify: `src/system.rs:54-93`
- Modify: `src/system.rs:151-304`

**Interfaces:**
- Consumes: `sysinfo::System`, `sysinfo::ProcessRefreshKind`, `sysinfo::ProcessesToUpdate`
- Produces: Fine-grained process, CPU, and memory refreshes without full kernel scans.

- [ ] **Step 1: Update `SystemMonitor` struct and fields**
In `src/system.rs`:
- Add `last_disk_refresh: Instant` to `SystemMonitor`.
- Cache user mapping `user_cache: HashMap<sysinfo::Uid, String>` so users aren't resolved and allocated from scratch on every tick.

- [ ] **Step 2: Update `SystemMonitor::refresh` in `src/system.rs`**
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

        // Only refresh disks every 15 seconds or when needed
        if self.last_disk_refresh.elapsed() >= std::time::Duration::from_secs(15) {
            self.disks.refresh(true);
            self.last_disk_refresh = now;
        }
...
```

- [ ] **Step 3: Add unit tests in `src/system.rs`**
Add unit tests verifying `SystemMonitor::refresh` updates metrics correctly.

- [ ] **Step 4: Verification and Commit**
Run: `cargo check` and `cargo test`
Commit:
```bash
git add src/system.rs
git commit -m "perf: replace broad refresh_all with targeted sysinfo refresh and cached user mappings"
```
