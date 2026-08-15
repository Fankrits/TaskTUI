### Task 6: Comprehensive Verification & Benchmark Validation

**Files:**
- All files in `src/` and `Cargo.toml`

**Interfaces:**
- Validates: Zero compilation errors, full test suite pass rate, release build size reduction, feature correctness across all UI tabs, views, and keyboard/mouse interactions.

- [ ] **Step 1: Run full debug and release builds**
Run: `cargo build` and `cargo build --release`
Expected: Both exit with code 0.

- [ ] **Step 2: Run complete unit test suite**
Run: `cargo test`
Expected: All tests pass with 0 failures.

- [ ] **Step 3: Benchmark binary size and verify optimizations**
Measure release binary size vs unoptimized build.

- [ ] **Step 4: Final verification and report**
Write final verification report to `docs/superpowers/plans/task-6-report.md`.
