## T01 — Accumulator Incremental Updates

### Objective

Optimize the NNUE accumulator update path so that `make_move`/`unmake_move` operations only recompute affected features rather than performing a full accumulator recomputation from the position. This eliminates redundant work on every node of the search tree.

### Deliverables

1. **Dirty flag tracking mechanism** in `crates/nnue` — per-accumulator dirty flags that record which side's accumulator has been invalidated by a make/unmake operation
2. **Incremental add/subtract feature updates** — on `make_move`, mark the affected side's accumulator dirty and record the feature additions and removals (king-relative piece changes); on `unmake_move`, restore the previous accumulator state
3. **Lazy recomputation** — only recompute an accumulator when it is accessed and its dirty flag is set, applying the minimal delta (feature add/subtract) rather than a full recomputation
4. **Tests** verifying that incrementally updated accumulators produce identical results to full recomputation across a suite of positions

### Files to Create or Modify

- `crates/nnue/src/accumulator.rs` (or equivalent) — dirty flag tracking, incremental update logic
- `crates/nnue/src/lib.rs` — integrate incremental updates into the inference/evaluation path
- Integration points with `crates/board` (make/unmake hooks) to trigger accumulator updates
- Test files within `crates/nnue/src/` (colocated `#[cfg(test)]` modules)

### Key Implementation Details

- Maintain per-accumulator dirty flags recording which side's accumulator is invalidated
- On `make_move`: mark affected side's accumulator dirty, record feature deltas (additions and removals based on king-relative piece changes)
- On `unmake_move`: restore the previous accumulator state (not just re-derive it)
- When an accumulator is accessed and dirty, apply the minimal delta (add/subtract feature vectors) instead of full recomputation
- The accumulator structure and inference engine from Phase P01 must already be in place
- The trained NNUE network from Phase P02 must exist for integration testing

### Dependencies

- **Phase P01 (NNUE Architecture)** — accumulator structure, inference engine, and feature representation must exist
- **Phase P02 (Training & Integration)** — a trained, integrated NNUE network must be available for correctness verification
- No dependencies on other tasks within this phase (T02 and T03 may depend on this)

### Verification Criteria

1. Incrementally updated accumulators produce **identical** evaluation results to full recomputation for every position in the test suite
2. The dirty flag mechanism correctly tracks which accumulators need recomputation after make/unmake sequences
3. `unmake_move` correctly restores the previous accumulator state
4. All existing NNUE tests continue to pass
5. `cargo test` passes for `crates/nnue`
6. `cargo clippy -- -D warnings` and `cargo fmt --check` pass