# T01 — Null Move Pruning

## Objective

Implement null move pruning to skip large portions of the search tree when the position strongly favors one side. Add `make_null_move` and `unmake_null_move` methods to `Position` and integrate null move pruning logic into the `negamax` function.

## Deliverables

1. `Position::make_null_move` method in `crates/board/src/position.rs` — flips side to move, clears en passant square, updates Zobrist hash, without moving any piece
2. `Position::unmake_null_move` method in `crates/board/src/position.rs` — restores prior state
3. Null move pruning logic in `negamax` in `crates/search/src/lib.rs`
4. Unit tests for null move correctness and zugzwang guard

## Files to Modify

- `crates/board/src/position.rs` — add `make_null_move` and `unmake_null_move` methods
- `crates/search/src/lib.rs` — add null move pruning conditional block within `negamax`

## Implementation Details

### Null Move Mechanism
- `make_null_move` flips side to move, clears en passant square, updates Zobrist hash — no piece movement
- `unmake_null_move` restores the prior state

### Null Move Pruning in negamax
- Before searching child moves, perform a null-move search at reduced depth with zero-window around beta: `negamax(pos, depth - 1 - R, -beta, -beta + 1, ...)`
- If null-move search score >= beta, return beta (prune the subtree)
- Reduction depth R: R=2 when remaining depth <= 6, R=3 when remaining depth > 6

### Disable Conditions
- Side to move has zero non-pawn material (only king and pawns remain) — check piece bitboards for knights, bishops, rooks, queens
- Current node is already a null-move search (no consecutive null moves)
- Position is in check
- `beta.abs() >= MATE_SCORE - MAX_PLY` (avoid pruning when mate scores need exact resolution)

## Dependencies

- No dependencies on other tasks within this phase
- Requires: M05 TT probing/ordering, M03 negamax/alpha-beta/quiescence/iterative deepening/MVV-LVA/killer/PV ordering, M01-M02 movegen/make_move/unmake_move/is_square_attacked

## Verification Criteria

1. `make_null_move` correctly flips side to move, clears en passant, updates Zobrist hash; `unmake_null_move` is exact inverse
2. Null move pruning activates in positions where both sides have non-pawn material and are not in check
3. Null move pruning is skipped in king-and-pawns-only positions (zugzwang guard)
4. Null move pruning is skipped when in check, during consecutive null-move searches, and when beta is a mate score
5. No illegal positions or hash corruption from null move application
6. Unit tests pass for all the above scenarios
7. `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` all pass