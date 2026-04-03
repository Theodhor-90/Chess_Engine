# Task T03 — Principal Variation Search (PVS)

## Objective

Replace the plain alpha-beta search with Principal Variation Search (PVS) in the `negamax` function. PVS searches the first move with a full window and all subsequent moves with a zero-window, re-searching with a full window on fail-high. This produces the same results as alpha-beta but with fewer nodes in well-ordered trees.

## Deliverables

- PVS logic replacing plain alpha-beta in `negamax` in `crates/search/src/lib.rs`
- PVS + LMR re-search cascade implementation
- WAC solve-rate and node-count comparison test

## Files to Modify

- `crates/search/src/lib.rs` — replace the alpha-beta loop in `negamax` with PVS logic

## Implementation Details

1. **First move**: Search the first move at each node with the full `(alpha, beta)` window, as in standard alpha-beta.
2. **Subsequent moves**: Search all remaining moves with a zero-window `(alpha, alpha + 1)` (also called a null-window or scout search).
3. **Fail-high re-search**: If the zero-window search returns a score > alpha (fail-high), re-search with the full `(alpha, beta)` window to get the exact score.
4. **PVS + LMR interaction**: For moves that are both late (LMR candidates) and non-PV, apply both reduced depth and zero-window. The re-search cascade is:
   - **Step 1**: Reduced-depth zero-window search
   - **Step 2**: If fail-high → full-depth zero-window search
   - **Step 3**: If fail-high again → full-depth full-window search
5. **PV node tracking**: The first move searched determines whether a node is a PV node. The node type affects whether zero-window or full-window is used for the initial search of each child.
6. **Integration**: PVS fully replaces the existing plain alpha-beta loop. The TT move (from M05) and move ordering (from M06/P02) are critical for PVS efficiency — the first move searched should ideally be the best move.

## Dependencies

- **T01 (Check Extension)** — check extension must be in place
- **T02 (Singular Extension)** — singular extension must be in place
- **M06/P01 (Pruning Techniques)** — LMR must be in place for the PVS + LMR re-search cascade
- **M06/P02 (Move Ordering)** — history, counter-move, and SEE ordering ensure the first move is likely the best, which is critical for PVS efficiency
- **M05 (Transposition Table)** — TT move is used as the first move for PVS

## Verification Criteria

1. PVS finds the same or better best move on WAC test suite positions compared to plain alpha-beta at the same depth
2. PVS searches equal or fewer nodes than plain alpha-beta on the WAC test suite at the same depth
3. The re-search cascade works correctly: reduced zero-window → full-depth zero-window → full-depth full-window (verified via unit tests or node-count analysis)
4. PVS interacts correctly with LMR — late moves get both reduced depth and zero-window
5. PVS interacts correctly with check extension and singular extension
6. All existing tests continue to pass (`cargo test`)
7. `cargo clippy -- -D warnings` and `cargo fmt --check` pass