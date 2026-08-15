# TaskTUI Runtime Performance — Round 2

Follow-up to the earlier lean-optimization pass. That round removed allocations
from filtering, sorting and sparkline storage; this round targets what was left:
the per-tick process scan, the render path, and two dependency-configuration
problems that were costing more than all the hand-optimized code combined.

## How this was measured

A temporary counting global allocator wrapped `System`, and each hot path was
run in a loop reporting allocations, bytes and wall time per operation. The
harness was removed before commit.

Numbers below are from the dev container, which had **75 processes and 23
sockets**. A typical desktop or server runs 400–600 processes, so every
per-process and per-row figure scales roughly 5–8× further in favour of the
optimized build.

## Results

| Operation | Before | After | Change |
|---|---|---|---|
| `monitor.refresh()` | 1,874 allocs / 4.14 MB / 1.82 ms | 1,441 allocs / 2.56 MB / 1.25 ms | −38% bytes, −31% time |
| filter+sort, sort by name | 1,058 allocs / 20 KB | **0 allocs / 0 bytes** | eliminated |
| filter+sort, active search | 182 allocs / 2.1 KB | **0 allocs / 0 bytes** | eliminated |
| `update_top_rankings()` | 2 allocs / 1.3 KB | **0 allocs / 0 bytes** | eliminated |
| Render Processes tab | 6,177 allocs / 792 KB / 1.34 ms | 1,400 allocs / 113 KB / 0.45 ms | −86% allocs, −86% bytes, −66% time |
| Render SystemDetails tab | 5,053 allocs / 827 KB / 1.17 ms | 498 allocs / 58 KB / 0.27 ms | −90% allocs, −93% bytes |
| `Layout::split` (one call) | 307 allocs / 50 KB | 2 allocs / 96 bytes | −99% |
| **Full tick + render (1 sec)** | **8,051 allocs / 4.93 MB / 3.16 ms** | **2,839 allocs / 2.67 MB / 1.86 ms** | **−65% allocs, −46% bytes, −41% time** |

Steady-state allocation churn dropped from ~4.9 MB/s to ~2.7 MB/s at 75
processes, and the remaining 2.56 MB sits inside `sysinfo`'s `/proc` reading —
TaskTUI's own per-tick code now allocates essentially nothing.

Release binary: 1,229,112 → 1,261,824 bytes (+32 KB), the cost of re-enabling
ratatui's layout cache. Worth it for a 99% cut in layout allocation.

## Changes

### 1. `layout-cache` was silently disabled (biggest single win)

`Cargo.toml` set `ratatui = { default-features = false, features = ["crossterm"] }`.
`layout-cache` is one of ratatui's *default* features, so turning defaults off to
trim the widget set also turned off layout memoisation. Every `Layout::split`
re-ran the constraint solver — 307 allocations and 50 KB each, ~20 times per
frame. Re-enabled explicitly.

### 2. Incremental process table (`src/system.rs`)

`refresh()` rebuilt the entire `Vec<ProcessInfo>` every tick, allocating a
`String` for name, user, cmd and exe path plus a `Vec` for ports, per process,
per second.

Identity fields never change for the lifetime of a PID, so they are now built
once when a process is first seen and reused. A `pid_index` map locates each
row; volatile metrics are overwritten in place; dead rows are compacted only on
ticks where something actually exited.

### 3. Data collected but never displayed

- `CpuRefreshKind::everything()` sampled per-core **frequency** every tick.
  Nothing in the UI renders frequency — switched to usage only.
- `with_disk_usage()` read `/proc/<pid>/io` for every process every tick, but
  disk I/O appears only in the details modal. Now sampled just for the process
  being inspected (measured: −1.2 MB and −15% refresh time on its own).
- Socket→process-name association re-ran every tick even though the socket list
  is only re-collected every 3s. Moved inside the throttle.
- `get_disks_info()` allocated a fresh `Vec<DiskInfo>` with three `String`s per
  disk on **every rendered frame**. Now cached and rebuilt on the 15s disk
  refresh.
- Dropped the `unicode-width` dependency — not referenced anywhere in `src/`.

### 4. Allocation-free filtering and sorting (`src/app.rs`)

Sorting by name or user called `to_lowercase()` *inside the comparator*, so an
O(n log n) sort performed O(n log n) heap allocations. `ProcessInfo` now carries
pre-computed `name_lower` / `user_lower` / `cmd_lower`, built once per PID.
Numeric matching uses a stack buffer instead of `to_string()`, socket protocol
and state are `&'static str`, and the ranking pass reuses one scratch buffer.

### 5. Virtualized tables (`src/ui/`)

Both tables built a `Row` — with every `Cell`, `Line` and `Span` inside it — for
*every* filtered entry, then drew only the ~40 that fit. They now materialise
just the visible window. At 75 processes this is a modest win; at 600 it is a
13× reduction in render work. `visible_window()` owns the scroll maths and is
unit-tested, with an end-to-end test asserting the selected row is really drawn
after scrolling to the end of a 500-row list.

### 6. Idle and input costs

- The Help tab renders nothing driven by live data, so ticks skip the whole
  sampling pass while it is open (and catch up on the way out).
- Process filtering/sorting runs only when the Processes tab is actually in
  front; the network filter only when its socket list changed.
- `handle_mouse_event` called `crossterm::terminal::size()` — an ioctl — before
  checking the event kind, so every mouse *motion* event paid for it. Motion,
  drag and button-release events now return before that call.

## Compatibility

No keyboard shortcut, mouse interaction, layout mode or displayed value changed.
All pre-existing tests pass unmodified; 5 tests were added covering the scroll
window, the allocation-free matching helpers, and renderer robustness against
stale indices. Clippy warning count is unchanged from the baseline (4, all
pre-existing). No `unsafe`.

## Note on toolchain

`sysinfo` 0.39.6 requires rustc ≥ 1.95; the container defaulted to 1.94.1 and
could not build the project at all until 1.95.0 was installed. Worth pinning via
`rust-version` in `Cargo.toml` or a `rust-toolchain.toml` so this surfaces as a
clear error rather than a dependency-resolution failure.

## Further opportunities (not done)

- The remaining 2.56 MB per refresh is inside `sysinfo` reading `/proc`. Cutting
  it means either refreshing processes less often than once a second, or reading
  `/proc` directly instead of through `sysinfo`.
- `per_core_cpu` is populated every tick but never rendered. It is now
  zero-allocation so the cost is negligible, and it was left in place as
  plausibly-intended API rather than removed.
