# T02 — Book Move Selection

## Objective

Implement configurable move selection strategies (`bestmove` and `weighted`) over the book entries returned by the T01 parser, so the engine can choose how to pick a move when multiple book entries exist for a position.

## Deliverables

- **`bestmove` selection mode**: Given a list of `PolyglotEntry` records for a position, return the move with the highest `weight` field. Break ties deterministically (e.g., first encountered).
- **`weighted` selection mode**: Given a list of `PolyglotEntry` records for a position, select a move randomly with probability proportional to each entry's `weight`. Use a proper RNG (e.g., `rand` crate or engine's internal RNG).
- **Selection interface**: A function/method that accepts a selection mode parameter (enum with variants `BestMove` and `Weighted`) and a slice of book entries, delegating to the appropriate strategy. Returns `Option<Move>` (None if the entry list is empty).
- **BookMode enum**: Define an enum `BookMode { BestMove, Weighted }` (or similar) to represent the selection mode.
- **Unit tests**: Verify that `bestmove` returns the highest-weight move. Verify that `weighted` mode produces a distribution consistent with entry weights (statistical test over many samples, or seed the RNG for deterministic testing).

## Key Implementation Details

- The move returned should be the engine's internal `Move` type (using the translation from T01).
- The weighted selection should handle edge cases: single entry (always returns that entry), all entries with equal weight (uniform distribution), entries with weight 0 (should not be selected, or handle per Polyglot spec).

## Files to Create or Modify

- Modify: `crates/engine/src/book/mod.rs` (or a submodule like `crates/engine/src/book/selection.rs`) — add selection functions and `BookMode` enum
- Modify: `crates/engine/Cargo.toml` — add `rand` crate dependency if needed for weighted random selection

## Dependencies

- Depends on **T01** (Polyglot book format parser) — needs `PolyglotEntry` struct and the move translation function.

## Verification Criteria

1. `cargo test` passes all new unit tests.
2. `bestmove` mode correctly returns the entry with the highest weight from a set of test entries.
3. `weighted` mode selects moves with probability proportional to weight (verified by deterministic seeded RNG test or statistical sampling).
4. Empty entry list returns `None`.
5. Selection interface correctly delegates based on `BookMode` parameter.
6. `cargo clippy -- -D warnings` and `cargo fmt --check` pass.