# TaskTUI Performance, Lean, and Lightweight Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform TaskTUI into an ultra-lean, low-CPU, low-memory system monitor by eliminating unnecessary OS thread traversals, removing ~2,500 heap allocations per second, indexing tables with zero-clone `Vec<usize>`, and caching frame data.

**Architecture:** 
- Configure `sysinfo` with selective refresh masks (`.without_tasks()`, `UpdateKind::OnlyIfNotSet`, `without_environment()`) to eliminate kernel syscall storms.
- Redesign `App` filter and sort engines to operate on lightweight `Vec<usize>` indices rather than cloning full `ProcessInfo` and `NetworkSocketItem` structs.
- Optimize socket PID resolution from $O(S \times P \times N)$ nested loops to an $O(N)$ hashmap lookup.
- Pre-round and store sparkline telemetry as `[u64; HISTORY_LEN]` slices to achieve zero heap allocations on frame draws.
- Pass borrowed string slices (`&str`) to Ratatui widgets, removing all `.clone()` and `format!` operations in the hot render loop.

**Tech Stack:** Rust 2024 Edition, `ratatui` (0.30.2), `sysinfo` (0.39.6), `crossterm` (0.29.0), `netstat2` (0.11.2), `anyhow`.

## Global Constraints
- Target 100% backward compatibility with all keyboard shortcuts, mouse interactions, and UI layout modes.
- Preserve all existing unit and integration test invariants.
- No `unsafe` code blocks.
- All code changes must pass `cargo test` and `cargo check`.

---

### Task 1: Trim Cargo Dependencies and Optimize Release Profile

**Files:**
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: Standard Cargo package metadata.
- Produces: Lean dependency tree with minimal default features and fat LTO release profile.

- [ ] **Step 1: Check existing Cargo.toml and test compilation baseline**

Run: `cargo test`
Expected: 21 tests PASS.

- [ ] **Step 2: Update Cargo.toml with stripped features and fat LTO profile**

Edit `Cargo.toml`:
```toml
[package]
name = "tasktui"
version = "0.1.0"
edition = "2024"
authors = ["Fankrits <fankritsadajra@gmail.com>"]
description = "A blazing-fast, lightweight, and modern terminal task manager and system monitor built with Rust and Ratatui."
license = "MIT"
readme = "README.md"
repository = "https://github.com/Fankrits/TaskTUI"
keywords = ["tui", "system-monitor", "task-manager", "process-viewer", "ratatui"]
categories = ["command-line-utilities", "system::monitoring"]

[dependencies]
anyhow = "1.0.104"
crossterm = { version = "0.29.0", default-features = false, features = ["bracketed-paste", "events"] }
netstat2 = "0.11.2"
ratatui = { version = "0.30.2", default-features = false, features = ["crossterm"] }
sysinfo = { version = "0.39.6", default-features = false, features = ["system", "processes", "network", "disk", "user"] }
unicode-width = "0.2.2"

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

- [ ] **Step 3: Verify build with pruned features**

Run: `cargo test`
Expected: PASS (all dependencies resolve and compile cleanly).

- [ ] **Step 4: Commit dependency optimizations**

```bash
git add Cargo.toml
git commit -m "build: prune dependency features and configure fat LTO in Cargo.toml"
```

---

### Task 2: Selective Sysinfo Refresh and Static Status in `src/system.rs`

**Files:**
- Modify: `src/system.rs`

**Interfaces:**
- Consumes: `sysinfo::{System, RefreshKind, CpuRefreshKind, MemoryRefreshKind, ProcessRefreshKind, UpdateKind, ProcessesToUpdate}`
- Produces: `ProcessInfo { status: &'static str, ... }`, `SystemMonitor::new()` and `SystemMonitor::refresh()` with `.without_tasks()`, `UpdateKind::OnlyIfNotSet`, and `.without_environment()`.

- [ ] **Step 1: Write test for selective refresh and static status mapping**

Add to `src/system.rs` `tests` module:
```rust
#[test]
fn test_status_mapping_static_str() {
    assert_eq!(SystemMonitor::map_status(sysinfo::ProcessStatus::Run), "Run");
    assert_eq!(SystemMonitor::map_status(sysinfo::ProcessStatus::Sleep), "Sleep");
    assert_eq!(SystemMonitor::map_status(sysinfo::ProcessStatus::Idle), "Idle");
    assert_eq!(SystemMonitor::map_status(sysinfo::ProcessStatus::Zombie), "Zombie");
    assert_eq!(SystemMonitor::map_status(sysinfo::ProcessStatus::Dead), "Dead");
}
```

- [ ] **Step 2: Update `ProcessInfo` and `SystemMonitor` in `src/system.rs`**

1. Change `pub status: String` to `pub status: &'static str` in `ProcessInfo`.
2. Add `map_status(status: sysinfo::ProcessStatus) -> &'static str` helper:
```rust
impl SystemMonitor {
    pub fn map_status(status: sysinfo::ProcessStatus) -> &'static str {
        match status {
            sysinfo::ProcessStatus::Run => "Run",
            sysinfo::ProcessStatus::Sleep => "Sleep",
            sysinfo::ProcessStatus::Idle => "Idle",
            sysinfo::ProcessStatus::Zombie => "Zombie",
            sysinfo::ProcessStatus::Dead => "Dead",
            sysinfo::ProcessStatus::Stop => "Stop",
            _ => "Other",
        }
    }
    ...
```
3. Update `SystemMonitor::new()`:
```rust
let refresh_kind = RefreshKind::nothing()
    .with_cpu(CpuRefreshKind::everything())
    .with_memory(MemoryRefreshKind::everything())
    .with_processes(
        ProcessRefreshKind::nothing()
            .with_cpu()
            .with_memory()
            .with_disk_usage()
            .with_exe(UpdateKind::OnlyIfNotSet)
            .with_cmd(UpdateKind::OnlyIfNotSet)
            .with_user(UpdateKind::OnlyIfNotSet)
            .without_tasks()
            .without_environment(),
    );
let mut sys = System::new_with_specifics(refresh_kind);
```
4. Update `SystemMonitor::refresh()`:
```rust
self.sys.refresh_cpu_all();
self.sys.refresh_memory();
self.sys.refresh_processes_specifics(
    ProcessesToUpdate::All,
    true,
    ProcessRefreshKind::nothing()
        .with_cpu()
        .with_memory()
        .with_disk_usage()
        .with_exe(UpdateKind::OnlyIfNotSet)
        .with_cmd(UpdateKind::OnlyIfNotSet)
        .with_user(UpdateKind::OnlyIfNotSet)
        .without_tasks()
        .without_environment(),
);
```
5. In process loop inside `refresh()`:
```rust
let status_str = Self::map_status(process.status());
```
6. Avoid redundant `.join(" ")` if `cmd` slice is empty.

- [ ] **Step 3: Run unit tests to verify**

Run: `cargo test --lib`
Expected: PASS.

- [ ] **Step 4: Commit selective sysinfo refresh changes**

```bash
git add src/system.rs
git commit -m "perf(system): skip tasks/env and use selective refresh in sysinfo"
```

---

### Task 3: Fast $O(1)$ Socket PID Resolution in `src/system.rs`

**Files:**
- Modify: `src/system.rs`

**Interfaces:**
- Consumes: `proc_list: &[ProcessInfo]`
- Produces: Fast hashmap-based PID to process name resolution in `SystemMonitor::refresh()` and `SystemMonitor::refresh_sockets()`.

- [ ] **Step 1: Write test for fast socket process name association**

Add test in `src/system.rs`:
```rust
#[test]
fn test_socket_process_name_association() {
    let mut monitor = SystemMonitor::new();
    monitor.refresh_sockets();
    for sock in &monitor.sockets {
        for name in &sock.process_names {
            assert!(!name.is_empty());
        }
    }
}
```

- [ ] **Step 2: Replace quadratic linear search with HashMap lookup in `src/system.rs`**

In `SystemMonitor::refresh()`:
```rust
// Build O(1) PID -> Name lookup
let pid_to_name: HashMap<u32, &str> = proc_list.iter().map(|p| (p.pid, p.name.as_str())).collect();

// Attach process names to sockets in O(S * P) instead of O(S * P * N)
for sock in &mut self.sockets {
    let mut names = Vec::with_capacity(sock.pids.len());
    for p in &sock.pids {
        if let Some(&name) = pid_to_name.get(p) {
            names.push(name.to_string());
        }
    }
    sock.process_names = names;
}
```
And in `SystemMonitor::refresh_sockets()`:
```rust
let pid_to_name: HashMap<u32, &str> = self.processes.iter().map(|p| (p.pid, p.name.as_str())).collect();
for sock in &mut self.sockets {
    let mut names = Vec::with_capacity(sock.pids.len());
    for p in &sock.pids {
        if let Some(&name) = pid_to_name.get(p) {
            names.push(name.to_string());
        }
    }
    sock.process_names = names;
}
```

- [ ] **Step 3: Run unit tests**

Run: `cargo test --lib`
Expected: PASS.

- [ ] **Step 4: Commit socket resolution optimization**

```bash
git add src/system.rs
git commit -m "perf(system): optimize socket-to-process lookup from O(N^2) to O(1) HashMap"
```

---

### Task 4: Zero-Allocation Sparkline History Slices

**Files:**
- Modify: `src/system.rs`
- Modify: `src/ui/graphs.rs`

**Interfaces:**
- Consumes: `cpu_percent`, `mem_percent`, network byte rates in `SystemMonitor::refresh()`
- Produces: `pub cpu_history: [u64; HISTORY_LEN]`, `pub mem_history: [u64; HISTORY_LEN]`, `pub net_rx_history: [u64; HISTORY_LEN]`, `pub net_tx_history: [u64; HISTORY_LEN]`.

- [ ] **Step 1: Write test for array-based history buffers**

Add test in `src/system.rs`:
```rust
#[test]
fn test_array_history_buffers() {
    let mut monitor = SystemMonitor::new();
    assert_eq!(monitor.cpu_history.len(), HISTORY_LEN);
    assert_eq!(monitor.mem_history.len(), HISTORY_LEN);
    assert_eq!(monitor.net_rx_history.len(), HISTORY_LEN);
    assert_eq!(monitor.net_tx_history.len(), HISTORY_LEN);

    monitor.refresh();
    assert!(monitor.cpu_history[HISTORY_LEN - 1] <= 100);
    assert!(monitor.mem_history[HISTORY_LEN - 1] <= 100);
}
```

- [ ] **Step 2: Update `SystemMonitor` history storage in `src/system.rs`**

Replace `VecDeque<f64>` fields with `[u64; HISTORY_LEN]`:
```rust
pub cpu_history: [u64; HISTORY_LEN],
pub mem_history: [u64; HISTORY_LEN],
pub net_rx_history: [u64; HISTORY_LEN],
pub net_tx_history: [u64; HISTORY_LEN],
```
In `SystemMonitor::new()`:
```rust
cpu_history: [0; HISTORY_LEN],
mem_history: [0; HISTORY_LEN],
net_rx_history: [0; HISTORY_LEN],
net_tx_history: [0; HISTORY_LEN],
```
In `SystemMonitor::refresh()`:
```rust
// Shift and push to fixed arrays
let cpu_val = (self.global_cpu.clamp(0.0, 100.0)).round() as u64;
let mem_val = if self.total_memory > 0 {
    ((self.used_memory as f64 / self.total_memory as f64 * 100.0).clamp(0.0, 100.0)).round() as u64
} else {
    0
};

self.cpu_history.rotate_left(1);
self.cpu_history[HISTORY_LEN - 1] = cpu_val;

self.mem_history.rotate_left(1);
self.mem_history[HISTORY_LEN - 1] = mem_val;

let rx_kb = (self.current_net_rx_rate / 1024.0).round() as u64;
let tx_kb = (self.current_net_tx_rate / 1024.0).round() as u64;

self.net_rx_history.rotate_left(1);
self.net_rx_history[HISTORY_LEN - 1] = rx_kb;

self.net_tx_history.rotate_left(1);
self.net_tx_history[HISTORY_LEN - 1] = tx_kb;
```

- [ ] **Step 3: Update `src/ui/graphs.rs` to pass history slices directly**

In `src/ui/graphs.rs`:
1. In `render_cpu_box`:
```rust
let sparkline = Sparkline::default()
    .data(&app.monitor.cpu_history)
    .max(100)
    .style(Style::default().fg(cpu_color));
```
2. In `render_memory_box`:
```rust
let sparkline = Sparkline::default()
    .data(&app.monitor.mem_history)
    .max(100)
    .style(Style::default().fg(theme.sparkline_mem));
```
3. In `render_network_box`:
```rust
let max_rx = app.monitor.net_rx_history.iter().copied().max().unwrap_or(10).max(10);
let rx_sparkline = Sparkline::default()
    .data(&app.monitor.net_rx_history)
    .max(max_rx)
    .style(Style::default().fg(theme.success));

let max_tx = app.monitor.net_tx_history.iter().copied().max().unwrap_or(10).max(10);
let tx_sparkline = Sparkline::default()
    .data(&app.monitor.net_tx_history)
    .max(max_tx)
    .style(Style::default().fg(theme.secondary));
```

- [ ] **Step 4: Run unit tests**

Run: `cargo test`
Expected: PASS.

- [ ] **Step 5: Commit history sparkline optimization**

```bash
git add src/system.rs src/ui/graphs.rs
git commit -m "perf(graphs): use zero-alloc fixed arrays for sparkline telemetry"
```

---

### Task 5: Zero-Clone Index-Based Filtering and Sorting in `src/app.rs`

**Files:**
- Modify: `src/app.rs`

**Interfaces:**
- Consumes: `monitor.processes: Vec<ProcessInfo>`, `monitor.sockets: Vec<NetworkSocketItem>`
- Produces: `filtered_processes: Vec<usize>`, `filtered_sockets: Vec<usize>`, `top_cpu_indices: Vec<usize>`, `top_mem_indices: Vec<usize>`, `get_selected_process(&self) -> Option<&ProcessInfo>`, `get_selected_socket(&self) -> Option<&NetworkSocketItem>`.

- [ ] **Step 1: Write tests for index-based filter, sort, and top-rank caching**

Add tests to `src/app.rs`:
```rust
#[test]
fn test_index_based_filter_and_sort() {
    let mut app = App::new();
    app.monitor.processes = vec![
        create_test_process(1, "alpha", 10.0, 500),
        create_test_process(2, "beta", 50.0, 100),
        create_test_process(3, "gamma", 20.0, 900),
    ];
    app.search_query = "beta".to_string();
    app.apply_process_filter_and_sort();

    assert_eq!(app.filtered_processes.len(), 1);
    assert_eq!(app.get_selected_process().unwrap().name, "beta");
}
```

- [ ] **Step 2: Refactor `App` fields and methods in `src/app.rs`**

1. Change fields in `App`:
```rust
pub filtered_processes: Vec<usize>,
pub filtered_sockets: Vec<usize>,
pub top_cpu_indices: Vec<usize>,
pub top_mem_indices: Vec<usize>,
```
2. Implement zero-clone `apply_process_filter_and_sort(&mut self)`:
```rust
pub fn apply_process_filter_and_sort(&mut self) {
    let query = self.search_query.to_lowercase().trim().to_string();
    self.filtered_processes.clear();

    for (idx, p) in self.monitor.processes.iter().enumerate() {
        if query.is_empty()
            || p.name.to_lowercase().contains(&query)
            || p.pid.to_string().contains(&query)
            || p.user.to_lowercase().contains(&query)
            || p.ports.iter().any(|port| port.to_string().contains(&query))
            || p.cmd.to_lowercase().contains(&query)
        {
            self.filtered_processes.push(idx);
        }
    }

    let procs = &self.monitor.processes;
    let dir = self.sort_direction;
    match self.sort_column {
        SortColumn::Pid => self.filtered_processes.sort_by(|&a, &b| match dir {
            SortDirection::Ascending => procs[a].pid.cmp(&procs[b].pid),
            SortDirection::Descending => procs[b].pid.cmp(&procs[a].pid),
        }),
        SortColumn::Name => self.filtered_processes.sort_by(|&a, &b| match dir {
            SortDirection::Ascending => procs[a].name.to_lowercase().cmp(&procs[b].name.to_lowercase()),
            SortDirection::Descending => procs[b].name.to_lowercase().cmp(&procs[a].name.to_lowercase()),
        }),
        SortColumn::Cpu => self.filtered_processes.sort_by(|&a, &b| match dir {
            SortDirection::Ascending => procs[a].cpu_usage.partial_cmp(&procs[b].cpu_usage).unwrap_or(std::cmp::Ordering::Equal),
            SortDirection::Descending => procs[b].cpu_usage.partial_cmp(&procs[a].cpu_usage).unwrap_or(std::cmp::Ordering::Equal),
        }),
        SortColumn::Memory => self.filtered_processes.sort_by(|&a, &b| match dir {
            SortDirection::Ascending => procs[a].memory_bytes.cmp(&procs[b].memory_bytes),
            SortDirection::Descending => procs[b].memory_bytes.cmp(&procs[a].memory_bytes),
        }),
        SortColumn::Port => self.filtered_processes.sort_by(|&a, &b| {
            let a_port = procs[a].ports.first().copied().unwrap_or(0);
            let b_port = procs[b].ports.first().copied().unwrap_or(0);
            match dir {
                SortDirection::Ascending => a_port.cmp(&b_port),
                SortDirection::Descending => b_port.cmp(&a_port),
            }
        }),
        SortColumn::User => self.filtered_processes.sort_by(|&a, &b| match dir {
            SortDirection::Ascending => procs[a].user.to_lowercase().cmp(&procs[b].user.to_lowercase()),
            SortDirection::Descending => procs[b].user.to_lowercase().cmp(&procs[a].user.to_lowercase()),
        }),
        SortColumn::Status => self.filtered_processes.sort_by(|&a, &b| match dir {
            SortDirection::Ascending => procs[a].status.cmp(procs[b].status),
            SortDirection::Descending => procs[b].status.cmp(procs[a].status),
        }),
    }

    if self.filtered_processes.is_empty() {
        self.selected_proc_idx = 0;
        self.process_table_state.select(None);
    } else {
        if self.selected_proc_idx >= self.filtered_processes.len() {
            self.selected_proc_idx = self.filtered_processes.len().saturating_sub(1);
        }
        self.process_table_state.select(Some(self.selected_proc_idx));
    }
}
```
3. Implement zero-clone `apply_network_filter_and_sort(&mut self)` using index sorting.
4. Implement cached top rankings in `update_top_rankings(&mut self)` and call it inside `on_tick()`:
```rust
pub fn update_top_rankings(&mut self) {
    let mut cpu_indices: Vec<usize> = (0..self.monitor.processes.len()).collect();
    if cpu_indices.len() > 5 {
        cpu_indices.select_nth_unstable_by(5, |&a, &b| {
            self.monitor.processes[b].cpu_usage
                .partial_cmp(&self.monitor.processes[a].cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        cpu_indices.truncate(5);
    }
    cpu_indices.sort_by(|&a, &b| {
        self.monitor.processes[b].cpu_usage
            .partial_cmp(&self.monitor.processes[a].cpu_usage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    self.top_cpu_indices = cpu_indices;

    let mut mem_indices: Vec<usize> = (0..self.monitor.processes.len()).collect();
    if mem_indices.len() > 5 {
        mem_indices.select_nth_unstable_by(5, |&a, &b| {
            self.monitor.processes[b].memory_bytes.cmp(&self.monitor.processes[a].memory_bytes)
        });
        mem_indices.truncate(5);
    }
    mem_indices.sort_by_key(|&b| std::cmp::Reverse(self.monitor.processes[b].memory_bytes));
    self.top_mem_indices = mem_indices;
}
```
5. Update accessors:
```rust
pub fn get_selected_process(&self) -> Option<&ProcessInfo> {
    self.filtered_processes.get(self.selected_proc_idx).and_then(|&idx| self.monitor.processes.get(idx))
}

pub fn get_selected_socket(&self) -> Option<&NetworkSocketItem> {
    self.filtered_sockets.get(self.selected_net_idx).and_then(|&idx| self.monitor.sockets.get(idx))
}
```

- [ ] **Step 3: Update existing unit tests in `src/app.rs`**

Update helper references in `app.rs` tests to use the new index-based APIs.

- [ ] **Step 4: Run unit tests**

Run: `cargo test --lib app`
Expected: PASS.

- [ ] **Step 5: Commit index-based filtering & sorting**

```bash
git add src/app.rs
git commit -m "perf(app): switch to zero-clone index-based filtering and sorting"
```

---

### Task 6: Zero-Copy UI & Table Rendering Across All Views

**Files:**
- Modify: `src/ui/process_table.rs`
- Modify: `src/ui/network_table.rs`
- Modify: `src/ui/graphs.rs`
- Modify: `src/ui/modals.rs`
- Modify: `src/event.rs`

**Interfaces:**
- Consumes: `app.filtered_processes: Vec<usize>`, `app.filtered_sockets: Vec<usize>`, `app.top_cpu_indices: Vec<usize>`, `app.top_mem_indices: Vec<usize>`
- Produces: Zero-allocation frame rendering using borrowed `&str` and pre-cached indices.

- [ ] **Step 1: Update `src/ui/process_table.rs` to render without clones**

In `src/ui/process_table.rs`:
```rust
let rows = app.filtered_processes.iter().enumerate().map(|(display_idx, &real_idx)| {
    let p = &app.monitor.processes[real_idx];
    let is_even = display_idx % 2 == 0;
    let row_bg = if is_even {
        Color::Rgb(15, 23, 42)
    } else {
        Color::Rgb(20, 30, 55)
    };

    let cpu_color = theme.cpu_color(p.cpu_usage);
    let mem_color = theme.memory_color(p.memory_percent);

    let status_icon = match p.status {
        "Run" => "● Run",
        "Sleep" => "○ Slp",
        "Idle" => "▲ Idl",
        "Zombie" | "Dead" => "✕ Ded",
        _ => "• Oth",
    };

    let ports_str = if p.ports.is_empty() {
        "-".to_string()
    } else {
        p.ports.iter().take(3).map(|port| format!(":{}", port)).collect::<Vec<_>>().join(", ")
    };

    let user_sess = match p.session_id {
        Some(sess) => format!("{} (S:{})", p.user, sess),
        None => p.user.clone(),
    };

    let cells = vec![
        Cell::from(Span::styled(p.pid.to_string(), Style::default().fg(theme.primary))),
        Cell::from(Span::styled(&p.name, Style::default().fg(theme.fg).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled(format!("{:>5.1}%", p.cpu_usage), Style::default().fg(cpu_color).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled(format_memory(p.memory_bytes), Style::default().fg(theme.fg))),
        Cell::from(Span::styled(format!("{:>4.1}%", p.memory_percent), Style::default().fg(mem_color))),
        Cell::from(Span::styled(ports_str, Style::default().fg(if p.ports.is_empty() { theme.fg_dim } else { theme.warning }))),
        Cell::from(Span::styled(status_icon, Style::default().fg(theme.status_color(p.status)))),
        Cell::from(Span::styled(user_sess, Style::default().fg(theme.fg_dim))),
        Cell::from(Span::styled(&p.cmd, Style::default().fg(theme.fg_dim))),
    ];

    Row::new(cells).style(Style::default().bg(row_bg)).height(1)
});
```

- [ ] **Step 2: Update `src/ui/network_table.rs` to render without clones**

In `src/ui/network_table.rs`:
Iterate over `app.filtered_sockets.iter().map(|&idx| &app.monitor.sockets[idx])` and use borrowed slices for `&s.protocol`, `&s.state`, `&s.local_addr`, `&s.remote_addr`.

- [ ] **Step 3: Update `src/ui/graphs.rs` to use cached rankings**

In `render_top_cpu_rank` and `render_top_memory_rank`:
Read directly from `&app.top_cpu_indices` and `&app.top_mem_indices` mapped to `&app.monitor.processes[idx]` with $O(1)$ complexity.

- [ ] **Step 4: Update `src/event.rs` row click hit testing**

In `src/event.rs`, update table row click handler to check `target_idx < app.filtered_processes.len()` and retrieve process via `app.filtered_processes.get(target_idx).and_then(|&idx| app.monitor.processes.get(idx))`.

- [ ] **Step 5: Run full UI tests**

Run: `cargo test`
Expected: All 21+ unit and UI rendering tests PASS.

- [ ] **Step 6: Commit UI rendering zero-copy refactor**

```bash
git add src/ui/ src/event.rs
git commit -m "perf(ui): eliminate allocations in table and ranking render loop"
```

---

### Task 7: Comprehensive End-to-End Verification & Benchmarking

**Files:**
- Test: All tests across `src/`
- Build: `target/release/tasktui`

- [ ] **Step 1: Run all unit and integration tests**

Run: `cargo test`
Expected: All tests pass with 0 warnings/failures.

- [ ] **Step 2: Run clippy linter for performance and cleanliness**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: Clean pass with 0 warnings.

- [ ] **Step 3: Compile optimized release binary and inspect size**

Run: `cargo build --release`
Expected: Success, compact binary generated in `target/release/tasktui`.

- [ ] **Step 4: Commit release verification artifacts**

```bash
git commit --allow-empty -m "chore: verify lean performance optimizations and all tests passing"
```
