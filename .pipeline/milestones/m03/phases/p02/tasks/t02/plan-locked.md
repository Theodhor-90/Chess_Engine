Now I have all the information needed. Let me produce the implementation plan.

## 1. Deliverables

| File | Purpose |
|------|---------|
| `crates/search/src/lib.rs` | Modified — add quiescence search function, integrate it at depth 0 in `negamax`, add capture/promotion filtering logic, add tests for tactical accuracy |

No new files are created. The `search` crate already exists from T01. This task modifies `lib.rs` to add quiescence search and wire it into the existing negamax function.

## 2. Dependencies

No new dependencies. The `chess-search` crate already depends on `chess-types`, `chess-board`, `chess-movegen`, and `chess-eval` (from T01). All required APIs are available:

- `chess_eval::evaluate(pos: &Position) -> i32` — stand-pat evaluation
- `chess_movegen::generate_legal_moves(pos: &mut Position) -> Vec<Move>` — move generation
- `Move::is_capture(self) -> bool` — identifies captures (flags 4, 5, or >= 12)
- `Move::is_promotion(self) -> bool` — identifies promotions (flags >= 8)
- `Position::make_move(&mut self, mv: Move) -> UndoInfo` and `Position::unmake_move(&mut self, mv: Move, undo: UndoInfo)` — apply/revert moves

## 3. Implementation Details

### `quiescence` function

- **Purpose**: Resolve tactical exchanges at leaf nodes to avoid the horizon effect.
- **Signature**: `pub fn quiescence(pos: &mut Position, mut alpha: i32, beta: i32, ply: u8) -> i32`
- **Exported**: `pub` so it can be tested directly and will be available for future integration.
- **Key logic**:
  1. Compute `stand_pat = chess_eval::evaluate(pos)`.
  2. If `stand_pat >= beta`, return `beta` (beta cutoff — position is already too good, opponent won't allow it).
  3. If `stand_pat > alpha`, update `alpha = stand_pat` (the side to move can always choose not to capture).
  4. Generate all legal moves via `chess_movegen::generate_legal_moves(pos)`.
  5. Filter to only captures and promotions: `moves.into_iter().filter(|mv| mv.is_capture() || mv.is_promotion())`.
  6. For each filtered move: call `pos.make_move(mv)`, recursively call `quiescence(pos, -beta, -alpha, ply + 1)`, negate the score, call `pos.unmake_move(mv, undo)`.
  7. If `score >= beta`, return `beta` (beta cutoff).
  8. If `score > alpha`, update `alpha = score`.
  9. After all moves, return `alpha`.

- **No checkmate/stalemate detection in quiescence**: When no captures/promotions remain, the stand-pat score is the result. Checkmate/stalemate are only detected in the main negamax search (which generates all legal moves). This is standard practice — quiescence only extends tactical lines.

### `negamax` modification

- **Change**: At line 25 (`if depth == 0`), replace `return (chess_eval::evaluate(pos), None);` with `return (quiescence(pos, alpha, beta, ply), None);`.
- **Purpose**: Instead of returning a raw static evaluation at depth 0, delegate to quiescence search to resolve pending captures/promotions before evaluating.

## 4. API Contracts

### `quiescence`

```rust
pub fn quiescence(pos: &mut Position, alpha: i32, beta: i32, ply: u8) -> i32
```

- **Input**: Mutable position reference, alpha-beta window bounds (`i32`), current ply depth (`u8`).
- **Output**: `i32` score from the perspective of the side to move, within the `[alpha, beta]` window.
- **Example**: Given a position where White has a hanging queen that Black can capture, `quiescence(pos, -INFINITY, INFINITY, 0)` returns a score reflecting the material gain from the capture sequence.

### `negamax` (modified)

Signature unchanged from T01:
```rust
pub fn negamax(pos: &mut Position, depth: u8, alpha: i32, beta: i32, ply: u8) -> (i32, Option<Move>)
```

The only behavioral change is that depth-0 nodes now invoke `quiescence` instead of `chess_eval::evaluate`.

## 5. Test Plan

All tests are added to the existing `#[cfg(test)] mod tests` block in `crates/search/src/lib.rs`.

### Test 1: `qsearch_stand_pat_cutoff`
- **Setup**: Use a position where the side to move has a clear material advantage and no captures are available (e.g., White up a queen, no captures on the board).
- **FEN example**: `4k3/8/8/8/8/8/8/Q3K3 w - - 0 1` (White has queen, no captures possible).
- **Assert**: `quiescence` returns a score equal to the static eval (stand-pat is the result when no captures exist).

### Test 2: `qsearch_finds_hanging_piece`
- **Setup**: A position where a high-value piece is hanging (can be captured for free).
- **FEN example**: `4k3/8/8/8/8/8/3q4/R3K3 w - - 0 1` (White rook can capture Black queen on d2).
- **Assert**: The score from `quiescence` is significantly better than the static eval (reflecting the material gain from capturing the queen). Specifically, `quiescence` score should be positive (White gains material).

### Test 3: `negamax_uses_quiescence_for_tactics`
- **Setup**: A position where a piece is hanging but only visible through quiescence (not within the main search depth without qsearch).
- **FEN example**: `4k3/8/8/3b4/8/8/8/R3K3 w - - 0 1` — White rook can take bishop. At depth 1, negamax calls qsearch at depth 0, which should find the capture.
- **Assert**: `negamax` at depth 1 finds the capture and returns a score reflecting material advantage.

### Test 4: `qsearch_beta_cutoff_in_captures`
- **Setup**: Call `quiescence` with a narrow alpha-beta window where the stand-pat is below alpha but a capture should trigger a beta cutoff.
- **Assert**: The function returns `beta` when a capture score exceeds beta, confirming alpha-beta pruning works within quiescence.

### Test 5: `qsearch_only_searches_captures_and_promotions`
- **Setup**: A position with both captures and quiet moves available. Verify that qsearch evaluates captures/promotions but ignores quiet moves.
- **FEN example**: `4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1` (White pawn can capture Black pawn on d5, or push to e5).
- **Assert**: The quiescence score accounts for the capture but the result is consistent with only tactical moves being searched (no quiet pawn pushes explored).

### Test 6: Existing T01 tests continue to pass
- All existing tests (`checkmate_black_is_mated`, `checkmate_detected_at_depth`, `stalemate_returns_zero`, `returns_legal_move_at_depth`, `prefers_capture_of_free_piece`, `alpha_beta_prunes`) must still pass. The `prefers_capture_of_free_piece` test is particularly relevant since it validates that captures are still found correctly with qsearch integrated.

## 6. Implementation Order

1. **Add `quiescence` function** to `crates/search/src/lib.rs` — implement the complete quiescence search with stand-pat evaluation, capture/promotion filtering, and alpha-beta pruning.
2. **Modify `negamax` depth-0 branch** — replace `return (chess_eval::evaluate(pos), None);` with `return (quiescence(pos, alpha, beta, ply), None);`.
3. **Add tests** — add the test cases specified in the Test Plan to the existing `mod tests` block.
4. **Run verification commands** — ensure all tests pass, code is formatted, and clippy is clean.

## 7. Verification Commands

```bash
# Format check
cargo fmt --check

# Clippy lint check
cargo clippy -- -D warnings

# Run all tests (includes existing T01 tests + new qsearch tests)
cargo test -p chess-search

# Run full workspace tests to ensure no regressions
cargo test
```

## 8. Design Deviations

None.