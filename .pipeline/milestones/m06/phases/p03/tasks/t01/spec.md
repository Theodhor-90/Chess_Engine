# Task T01 — Check Extension

## Objective

Implement check extension in the `negamax` function so that when the side to move is in check, the search depth is extended by one ply. This prevents the engine from missing forced tactics by ensuring checked positions are searched deeper.

## Deliverables

- Check extension logic in `negamax` in `crates/search/src/lib.rs`
- Unit tests confirming extension activates in check positions
- WAC solve-rate comparison test

## Files to Modify

- `crates/search/src/lib.rs` — add check extension logic at the top of `negamax`, before any pruning decisions

## Implementation Details

1. **Placement**: Check extension is applied at the top of the `negamax` function, before any pruning decisions (null move pruning, LMR, futility pruning). When the side to move is in check, increment the search depth by one ply.
2. **In-check detection**: Use the existing `is_square_attacked` from M01/M02 to determine if the king of the side to move is in check.
3. **Pruning guards**: Ensure that nodes where the side to move is in check skip null move pruning, futility pruning, and reverse futility pruning. These guards should already exist from M06/P01, but verify correctness with the added extension.
4. **No LMR when in check**: Positions in check must not be subject to late move reductions.

## Dependencies

- **M06/P01 (Pruning Techniques)** — null move pruning, LMR, and futility pruning must be in place so that check extension correctly interacts with pruning guards
- **M06/P02 (Move Ordering Improvements)** — history, counter-move, and SEE ordering must be in place
- **M05 (Transposition Table)** — TT probing must be functional
- **M01–M02** — `is_square_attacked` for in-check detection, legal move generation

## Verification Criteria

1. When the side to move is in check, the search depth is incremented by one ply (unit test with a known check position verifying depth parameter)
2. Nodes in check are NOT subject to null move pruning, LMR, or futility pruning
3. Check extension improves solve rate on WAC tactical test suite positions at fixed depth compared to baseline without the extension
4. All existing tests continue to pass (`cargo test`)
5. `cargo clippy -- -D warnings` and `cargo fmt --check` pass