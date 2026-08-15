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
Update `render_top_cpu_rank` and `render_top_memory_rank` in `src/ui/graphs.rs`:
The helpers receive `top_cpu: Vec<&ProcessInfo>` / `top_mem: Vec<&ProcessInfo>` and iterate over them cleanly.

- [ ] **Step 3: Add unit tests in `src/app.rs`**
Add unit test verifying `get_top_cpu_processes` and `get_top_memory_processes` return correct ordered top items without cloning.

- [ ] **Step 4: Verification and Commit**
Run: `cargo check` and `cargo test`
Commit:
```bash
git add src/app.rs src/ui/graphs.rs
git commit -m "perf: eliminate vector cloning in top cpu and memory process rankings"
```
