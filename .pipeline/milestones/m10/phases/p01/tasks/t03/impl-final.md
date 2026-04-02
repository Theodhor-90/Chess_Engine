All verification commands pass. Here's the summary:

- **`crates/nnue/src/loader.rs`**:
  - **Line 416-418**: Replaced `result.unwrap_err()` pattern (which requires `Network: Debug`) with `let Err(err) = load(&path) else { panic!("expected error") };` in `load_unsupported_version` test, matching the pattern used by other error tests
  - **Lines 305-316**: Fixed `load_dimension_mismatch` test — removed incorrect arch_hash recomputation that produced a hash mismatching the compiled architecture, causing `ArchitectureMismatch` to fire before reaching the `DimensionMismatch` check. The compiled arch_hash is now left intact so the loader reaches the individual dimension checks.