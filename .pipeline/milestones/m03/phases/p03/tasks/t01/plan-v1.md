Now I have everything I need. Let me produce the implementation plan.

## 1. Deliverables

- `crates/search/src/ordering.rs` — Move scoring function that assigns MVV-LVA scores to capture moves and a sort function that orders a move list by score. New file.
- `crates/search/src/lib.rs` — Modified to declare the `ordering` module, import the scoring/sorting function, and integrate MVV-LVA ordering into both `negamax` and `quiescence` before iterating over moves.

## 2. Dependencies

- `chess_eval::material::piece_value` — already available in the `chess-eval` dependency of the `chess-search` crate (`Cargo.toml` line 10).
- `chess_types::{Move, MoveFlag, PieceKind}` — already available via the `chess-types` dependency.
- `chess_board::Position` — already available via the `chess-board` dependency, provides `piece_on(Square) -> Option<Piece>` to identify the victim piece on the target square.
- No new crate dependencies required.

## 3. Implementation Details

### `crates/search/src/ordering.rs`

**Purpose**: Provide MVV-LVA scoring and move sorting for capture moves.

**Exports**:
- `pub fn score_mvv_lva(mv: Move, pos: &Position) -> i32` — Returns MVV-LVA score for a capture move. For non-capture moves, returns 0.
- `pub fn order_moves(moves: &mut Vec<Move>, pos: &Position)` — Sorts the move list in-place by MVV-LVA score, descending. Non-captures get score 0 and sort after all captures.

**Key logic for `score_mvv_lva`**:
1. Check `mv.is_capture()`. If not a capture, return 0.
2. Determine the **attacker** piece kind: call `pos.piece_on(mv.from_sq())` to get `Option<Piece>`, extract `.kind`. For promotions, the attacker is the pawn (which is still on `from_sq` before the move is made).
3. Determine the **victim** piece kind:
   - If `mv.is_en_passant()`: victim is `PieceKind::Pawn`.
   - Otherwise: call `pos.piece_on(mv.to_sq())` to get the captured piece, extract `.kind`.
4. Compute: `chess_eval::material::piece_value(victim_kind) * 100 - chess_eval::material::piece_value(attacker_kind)`.

**Key logic for `order_moves`**:
1. Compute a score for each move using `score_mvv_lva`.
2. Sort moves in descending score order using `sort_by_key` with negated scores (or `sort_unstable_by` with reverse comparison). Use `sort_unstable_by` for performance since move identity doesn't matter for equal scores.

**Unit tests** (colocated `#[cfg(test)] mod tests`):
- Test that `score_mvv_lva` returns a higher score for PxQ than PxP.
- Test that `score_mvv_lva` returns a higher score for PxR than NxR (same victim, lower attacker scores higher).
- Test that `score_mvv_lva` returns 0 for a quiet move.
- Test that `order_moves` places captures before quiet moves.

### `crates/search/src/lib.rs`

**Changes**:
1. Add `pub mod ordering;` declaration.
2. In `negamax` (line 102): after `let moves = chess_movegen::generate_legal_moves(pos);`, add `let mut moves = moves;` and call `ordering::order_moves(&mut moves, pos);` before the `moves.is_empty()` check. The empty check works the same since ordering doesn't change the set of moves.
3. In `quiescence` (line 58): after `let moves = chess_movegen::generate_legal_moves(pos);`, filter to captures/promotions first into a `let mut tactical: Vec<Move>`, then call `ordering::order_moves(&mut tactical, pos);`, then iterate over `tactical`. Alternatively, apply ordering to the full move list before filtering — but filtering first then ordering the smaller list is more efficient. The current code filters inline in the `for` loop; restructure to collect tactical moves, order them, then iterate.

## 4. API Contracts

```rust
use chess_search::ordering::{score_mvv_lva, order_moves};

// Score a single capture move
let score = score_mvv_lva(capture_move, &pos); // e.g., 89700 for PxQ (900*100 - 100 - pawn attacker vs queen victim)

// Order a full move list (captures sorted to front by MVV-LVA, quiets at end)
let mut moves = chess_movegen::generate_legal_moves(&mut pos);
order_moves(&mut moves, &pos);
```

N/A for REST/HTTP.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| `score_mvv_lva_pxq_higher_than_pxp` | Capturing a queen with a pawn scores higher than capturing a pawn with a pawn |
| `score_mvv_lva_lower_attacker_scores_higher` | PxR scores higher than NxR (same victim, lower-value attacker preferred) |
| `score_mvv_lva_quiet_returns_zero` | A non-capture move receives score 0 |
| `order_moves_captures_before_quiet` | After ordering, all capture moves appear before all quiet moves |
| Existing tests pass | All existing `negamax`, `quiescence`, and `search` tests in `lib.rs` continue to pass, confirming integration doesn't break behavior |

**Test setup**: Tests in `ordering.rs` use `Position::from_fen` to set up positions with known pieces, then create or extract specific moves to score. Use `chess_movegen::generate_legal_moves` to get real moves from positions rather than hand-crafting raw move bits.

## 6. Implementation Order

1. `crates/search/src/ordering.rs` — implement `score_mvv_lva` and `order_moves`, including unit tests
2. `crates/search/src/lib.rs` — add `pub mod ordering;` declaration
3. `crates/search/src/lib.rs` — integrate `ordering::order_moves` into `negamax` (after move generation, before iteration)
4. `crates/search/src/lib.rs` — integrate `ordering::order_moves` into `quiescence` (restructure tactical move filtering, then order, then iterate)
5. Run `cargo test -p chess-search` to verify all existing and new tests pass
6. Run `cargo fmt --check` and `cargo clippy -- -D warnings` for compliance

## 7. Verification Commands

```bash
cargo test -p chess-search
cargo fmt --check
cargo clippy -- -D warnings
```

## 8. Design Deviations

None.