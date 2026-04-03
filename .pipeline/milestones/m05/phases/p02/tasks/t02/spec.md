# T02 — Internal Iterative Deepening (IID)

## Objective

Implement internal iterative deepening in the `negamax` function. At nodes with high remaining depth and no TT move, perform a reduced-depth search first to populate the TT, then use the resulting TT move for ordering during the full-depth search.

## Deliverables

- **Compile-time constant** `const IID_MIN_DEPTH: i32 = 4;` defined at the top of `crates/search/src/lib.rs`
- **IID logic in `negamax`** in `crates/search/src/lib.rs` that triggers when `depth >= IID_MIN_DEPTH` and no TT move is available from the probe
- **Tests** verifying:
  - (a) IID reduces total node count compared to search without IID on positions at sufficient depth
  - (b) IID does not trigger at depths below 4
  - (c) IID correctly finds a move that the subsequent full-depth search uses for ordering

## Files to Modify

- `crates/search/src/lib.rs` — Add `IID_MIN_DEPTH` constant and IID logic in `negamax`

## Key Implementation Details

- IID triggers at nodes where remaining depth >= 4 (`IID_MIN_DEPTH`) and the TT probe did not return a best move
- The reduced-depth search uses depth - 2 (i.e., reduced by 2 plies)
- The reduced search populates the TT, so after IID runs, re-probe the TT to obtain a move for ordering
- IID reuses the existing `negamax` function — no separate search routine is needed
- The threshold `IID_MIN_DEPTH = 4` is a compile-time constant, not a tunable parameter at this stage

## Dependencies

- **T01** (TT-Based Move Ordering): TT move legality validation must be in place since the IID search populates the TT with moves that will be used for ordering
- **M05/P01** (Transposition Table): TT probe/store infrastructure
- **M03** (Basic Search): `negamax` function that IID extends

## Verification Criteria

1. IID reduces total node count compared to searching without IID at high-depth nodes lacking a TT entry
2. IID does not activate at depths below the `IID_MIN_DEPTH` threshold (4)
3. All existing tests in `crates/search/` continue to pass
4. `cargo test` passes across the full workspace