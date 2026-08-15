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
