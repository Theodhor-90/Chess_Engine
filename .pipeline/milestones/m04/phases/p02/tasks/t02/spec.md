# T02 — Search Integration

## Objective

Modify the search to use the time allocation from T01 as its time budget, so the engine respects clock constraints during play. When the budget is exceeded, the engine aborts search and returns the best move from the last fully completed iteration.

## Deliverables

- The existing `SearchContext::check_time` mechanism (every 1024 nodes) enforces the time budget by setting `aborted = true` when elapsed time exceeds the allocation.
- When aborted mid-iteration during iterative deepening, the engine returns the best move from the **last fully completed iteration**, not a partially searched one. This ensures the returned move has a complete evaluation behind it.
- The engine main loop in `crates/engine/src/main.rs` is updated to call the T01 time allocation function and pass the computed budget to `search()`.

## Files to Modify

- **`crates/engine/src/main.rs`** — update the `Go` handler to call the time allocation function from T01 and pass the computed `Duration` budget to `search()`
- **`crates/search/src/lib.rs`** — ensure `SearchContext::check_time` uses the passed budget to abort when time is exceeded; ensure iterative deepening returns the best move from the last fully completed iteration

## Key Constraints

- Reuse the existing `Arc<AtomicBool>` stop flag from Phase P01 — it serves dual duty (set by `stop` command and by time management when budget expires). No additional signaling mechanism.
- Time is checked every **1024 nodes** to minimize performance overhead of `Instant::elapsed()` system calls.
- Return best move from the last **fully completed** iteration when time expires mid-iteration.
- Replace the hardcoded `Duration::from_secs(5)` fallback with the T01 allocation function.

## Dependencies

- **T01** (Time Allocation) — the time budget computation function
- Phase P01 (UCI Core) — the engine main loop, `GoParams` struct, `Arc<AtomicBool>` stop flag
- M03 (Basic Search & Evaluation) — `SearchContext`, `check_time`, iterative deepening

## Verification Criteria

1. Search aborts correctly when the computed time budget expires
2. Engine returns a valid best move (from the last fully completed iteration) even when aborted
3. The hardcoded 5-second fallback in `main.rs` is replaced by the T01 allocation function
4. `cargo test` passes
5. `cargo clippy -- -D warnings` passes
6. `cargo fmt --check` passes