# Task T02 — Singular Extension

## Objective

Implement singular extension in the `negamax` function so that when a TT move is significantly better than all alternatives, its search depth is extended by one ply. This avoids missing critical moves that dominate the position.

## Deliverables

- Singular extension logic in `negamax` in `crates/search/src/lib.rs`
- Singular extension margin constant (initial value: 64 centipawns)
- Unit tests for activation and non-activation conditions

## Files to Modify

- `crates/search/src/lib.rs` — add singular extension logic within `negamax`

## Implementation Details

1. **Depth threshold**: Only attempt singular extension at nodes with remaining depth ≥ 6 to justify the cost of the verification search.
2. **Precondition**: A TT move must exist with sufficient depth information in the transposition table entry.
3. **Verification search**: Perform a reduced-depth search at `depth / 2`, excluding the TT move from the move list, with a zero-window centered at `tt_score - margin` (margin = 64 centipawns).
4. **Extension trigger**: If all alternatives score below `tt_score - margin`, extend the TT move's search depth by one ply.
5. **Non-trigger**: If any alternative scores at or above `tt_score - margin`, do not extend.
6. **Margin constant**: Define `SINGULAR_MARGIN: i32 = 64` (centipawns) as a named constant for tunability.

## Dependencies

- **T01 (Check Extension)** — check extension must be in place; singular extension is a separate extension that can stack with check extension
- **M05 (Transposition Table)** — TT entries with score and depth information are required to identify candidate TT moves and their scores
- **M06/P01 (Pruning Techniques)** — pruning logic must be functional for the verification search
- **M06/P02 (Move Ordering)** — move ordering must be in place for the verification search

## Verification Criteria

1. Singular extension triggers when the TT move is significantly better than all alternatives (unit test with a crafted position where one move clearly dominates)
2. Singular extension does NOT trigger when alternatives score comparably to the TT move (unit test)
3. Singular extension is only attempted at depth ≥ 6 (unit test verifying no activation at depth < 6)
4. Check and singular extensions together improve solve rate on the WAC tactical test suite at fixed depth compared to baseline without extensions
5. All existing tests continue to pass (`cargo test`)
6. `cargo clippy -- -D warnings` and `cargo fmt --check` pass