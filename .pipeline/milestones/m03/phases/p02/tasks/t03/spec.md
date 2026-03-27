# T03 — Iterative Deepening with Basic Time Management

## Objective

Implement iterative deepening that progressively searches at increasing depths, with basic time management that respects a fixed time budget per move. This provides the top-level search entry point for the engine.

## Deliverables

- Iterative deepening loop in `crates/search/src/lib.rs`
- Node counter with periodic time checks (every 1024 nodes)
- Top-level search entry point: `pub fn search(pos: &mut Position, time_budget: Duration) -> Option<Move>`
- Tests verifying deeper searches produce equal or better moves and that time limits are respected

## Implementation Details

### Iterative Deepening Loop

- Search at depth 1, then depth 2, then depth 3, incrementing by 1 each iteration
- Collect the best move at each completed depth
- Return the best move from the last **fully completed** depth when time runs out
- If time expires mid-search at a deeper depth, discard that incomplete result and return the result from the previous completed depth

### Time Management

- Accept a fixed time budget as `std::time::Duration`
- Record `Instant::now()` at search start
- Check elapsed time every **1024 nodes** during search (both negamax and quiescence)
- Abort cleanly when time is exhausted — the search must propagate the abort signal up the call stack
- Use `std::time::Instant` for timing

### Node Counting

- Maintain a node counter that increments for every node visited (in both negamax and quiescence search)
- The counter is used both for time-check intervals and can be reported for debugging/info

### Top-Level Entry Point

- `pub fn search(pos: &mut Position, time_budget: Duration) -> Option<Move>`
- Encapsulates iterative deepening and time management
- Returns `Some(best_move)` if any legal move exists, `None` only if the position has no legal moves

## Dependencies

- **T01 (Negamax Search)**: Iterative deepening calls the negamax search function at progressively increasing depths
- **T02 (Quiescence Search)**: Quiescence search must be integrated into negamax so that iterative deepening benefits from tactical resolution at leaf nodes
- **eval crate**: Used indirectly through negamax/quiescence
- **movegen crate**: Used indirectly through negamax/quiescence

## Verification Criteria

1. Iterative deepening searches at monotonically increasing depths (1, 2, 3, ...)
2. Deeper searches produce equal or better moves than shallower ones (verified on positions with known best moves)
3. The search respects the time budget — it does not significantly overrun the allocated time
4. The search returns a legal move from the last completed iteration
5. The top-level `search` function returns `Some(move)` for positions with legal moves and handles edge cases (checkmate/stalemate positions) gracefully
6. Node counter accurately tracks nodes visited
7. All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`