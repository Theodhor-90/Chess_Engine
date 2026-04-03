# T02 — Late Move Reductions (LMR)

## Objective

Implement late move reductions to reduce search depth for moves ordered late in the move list (likely bad moves), significantly reducing the effective branching factor. Achieve >=30% node count reduction at depth 10 on standard benchmark positions compared to baseline without LMR.

## Deliverables

1. Pre-computed LMR reduction table
2. LMR logic in `negamax` with re-search on fail-high
3. Benchmark test confirming >=30% node reduction at depth 10

## Files to Modify

- `crates/search/src/lib.rs` — add LMR reduction table initialization and LMR logic within `negamax`

## Implementation Details

### LMR Reduction Table
- Pre-compute a 2D table indexed by `(depth, move_index)`
- Formula: `floor(ln(depth) * ln(move_index) / 1.75)` where the divisor constant C=1.75 is the initial tuning value
- Table should be computed once at engine initialization (or as a static/lazy initialization)

### LMR Logic in negamax
- After searching the first few moves at full depth, reduce search depth for later quiet moves
- Do NOT reduce: TT moves, PV moves, killer moves, moves that give check, captures, promotions
- Only reduce quiet, non-special moves
- Reduced search: `negamax(pos, depth - 1 - reduction, -alpha - 1, -alpha, ...)`
- If the reduced search returns a score above alpha, re-search at full depth to confirm

### Integration Points
- Must interact correctly with existing move ordering pipeline (TT move, killers, MVV-LVA, PV)
- Must interact correctly with TT probing and quiescence search
- Must interact correctly with null move pruning from T01

## Dependencies

- **T01 (Null Move Pruning)** — both techniques coexist in `negamax`; LMR should be applied after null move pruning check
- Requires: M05 TT, M03 search framework, M01-M02 movegen

## Verification Criteria

1. LMR reduction table is correctly pre-computed with expected values for sample (depth, move_index) pairs
2. LMR achieves >=30% node count reduction at depth 10 on standard benchmark position set (startpos, Kiwipete, perft suite positions) compared to baseline without LMR
3. No regression in WAC (Win At Chess) tactical test suite solve rate at fixed depth
4. LMR correctly skips reduction for TT moves, PV moves, killers, checks, captures, and promotions
5. Re-search at full depth triggers correctly when reduced search returns above alpha
6. `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` all pass