# T02 — Quiescence Search

## Objective

Implement quiescence search to resolve tactical positions at leaf nodes, replacing the raw static evaluation call at depth 0. This prevents the horizon effect where the engine misjudges positions with pending captures.

## Deliverables

- Quiescence search function in `crates/search/src/lib.rs`
- Capture/promotion move filtering logic
- Integration into negamax at depth 0 (replace direct `eval::evaluate` call)
- Tests verifying the search does not miss hanging pieces or simple captures

## Implementation Details

### Quiescence Search Function

- **Signature**: Takes `&mut Position`, alpha (`i32`), beta (`i32`), and ply (`u8`). Returns `i32`.
- **Stand-pat evaluation**: Compute static eval via `eval::evaluate(pos)`. If `stand_pat >= beta`, return beta (beta cutoff). Update alpha to `max(alpha, stand_pat)`.
- **Move generation and filtering**: Generate all legal moves via `movegen::generate_legal_moves`, then filter to only captures (using `Move::is_capture()`) and promotions.
- **Recursive search**: For each capture/promotion move, apply `make_move`/`unmake_move` and recursively call quiescence search with `(-beta, -alpha)`. Negate returned score. Apply alpha-beta pruning (beta cutoff when `score >= beta`).
- **Termination**: No explicit depth limit — terminates naturally because sequential captures are finite. The stand-pat evaluation provides the lower bound (the side to move can choose not to capture).

### Integration with Negamax

- Modify the T01 negamax function: at depth 0, call the quiescence search function instead of `eval::evaluate` directly.

## Dependencies

- **T01 (Negamax Search)**: The quiescence search integrates into the negamax framework created in T01. It replaces the depth-0 static eval call.
- **eval crate**: `evaluate(pos: &Position) -> i32`
- **movegen crate**: `generate_legal_moves(pos: &mut Position) -> Vec<Move>`
- **types crate**: `Move::is_capture()` method for filtering

## Verification Criteria

1. Quiescence search is called at depth 0 instead of a direct static evaluation
2. The search correctly identifies and evaluates only captures and promotions at leaf nodes
3. Stand-pat cutoff works — if static eval is already >= beta, the search returns immediately
4. The search does not miss hanging pieces or simple captures within its search depth (e.g., in a position where a queen is hanging, the engine finds the capture even at low main search depths)
5. Alpha-beta pruning is correctly applied within quiescence search
6. Tests verify tactical accuracy on positions with hanging material
7. All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`