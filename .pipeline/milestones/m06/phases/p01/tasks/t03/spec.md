# T03 — Futility Pruning

## Objective

Implement futility pruning and reverse futility pruning to skip quiet moves near leaf nodes when the static evaluation indicates they cannot improve the position enough to affect the search result. Validate that the WAC tactical test suite solve rate is not reduced.

## Deliverables

1. Futility pruning logic in `negamax`
2. Reverse futility pruning logic in `negamax`
3. Margin constants for both techniques
4. WAC solve-rate validation test

## Files to Modify

- `crates/search/src/lib.rs` — add futility pruning and reverse futility pruning conditional blocks within `negamax`

## Implementation Details

### Static Eval Caching
- Compute static eval once at the top of `negamax` and reuse it for both futility and reverse futility checks
- Use the existing eval function from `crates/eval/`

### Futility Pruning (at frontier nodes)
- Apply at depth <= 1 (can extend to depth <= 3 for quiet moves)
- If `static_eval + futility_margin < alpha`, skip quiet moves
- Margin table:
  - Depth 1: 200 cp
  - Depth 2: 400 cp
  - Depth 3: 600 cp

### Reverse Futility Pruning (at pre-frontier nodes)
- Apply at depth <= 3
- If `static_eval - reverse_futility_margin >= beta`, return static eval (cut off entire node)
- Margin table:
  - Depth 1: 100 cp
  - Depth 2: 300 cp
  - Depth 3: 500 cp

### Do NOT Apply When
- The side to move is in check
- The move gives check
- The move is a capture or promotion (tactical moves)
- Note: reverse futility pruning prunes at the node level before the move loop; futility pruning prunes individual moves within the move loop

### Integration Points
- Must interact correctly with null move pruning (T01) and LMR (T02) already in `negamax`
- Must interact correctly with TT probing, killer moves, PV ordering, and quiescence search

## Dependencies

- **T01 (Null Move Pruning)** and **T02 (LMR)** — all three techniques coexist in `negamax`
- Requires: M05 TT, M03 search/eval framework, M01-M02 movegen

## Verification Criteria

1. Futility pruning correctly skips quiet moves when static eval + margin < alpha at shallow depths
2. Reverse futility pruning correctly returns static eval when static eval - margin >= beta at shallow depths
3. Neither technique is applied when in check, for checks, captures, or promotions
4. WAC tactical test suite solve rate is NOT reduced at fixed depth compared to baseline without futility pruning
5. Margin constants match the specification (200/400/600 for futility, 100/300/500 for reverse futility)
6. Static eval is computed once per node and reused
7. All three pruning techniques (null move, LMR, futility) interact correctly within `negamax`
8. `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` all pass