Now I have a complete picture of all the APIs. Let me produce the task plan.

## 1. Deliverables

- `crates/search/Cargo.toml` — new crate manifest declaring `chess-search` with dependencies on `chess-types`, `chess-board`, `chess-movegen`, and `chess-eval`
- `crates/search/src/lib.rs` — negamax function with alpha-beta pruning, score constants (`MATE_SCORE`, `INFINITY`), and unit tests
- `Cargo.toml` (workspace root) — updated `members` list to include `crates/search`

## 2. Dependencies

- `chess-types = { path = "../types" }` — provides `Move`, `Square`, `Piece`, `PieceKind`, `Color`, `Bitboard`
- `chess-board = { path = "../board" }` — provides `Position` with `make_move`, `unmake_move`, `is_square_attacked`, `side_to_move`, `piece_bitboard`
- `chess-movegen = { path = "../movegen" }` — provides `generate_legal_moves(pos: &mut Position) -> Vec<Move>`
- `chess-eval = { path = "../eval" }` — provides `evaluate(pos: &Position) -> i32`
- No new external dependencies required. All dependencies are existing workspace crates.

## 3. Implementation Details

### `Cargo.toml` (workspace root)

Add `"crates/search"` to the `members` list:

```toml
members = ["crates/types", "crates/board", "crates/movegen", "crates/eval", "crates/search", "tools/perft"]
```

### `crates/search/Cargo.toml`

```toml
[package]
name = "chess-search"
version = "0.1.0"
edition.workspace = true

[dependencies]
chess-types = { path = "../types" }
chess-board = { path = "../board" }
chess-movegen = { path = "../movegen" }
chess-eval = { path = "../eval" }
```

### `crates/search/src/lib.rs`

**Score Constants:**

- `pub const MATE_SCORE: i32 = 30000;` — base mate score, adjusted by ply distance
- `pub const INFINITY: i32 = 31000;` — used for initial alpha/beta window bounds

**Helper: `king_square`**

A small inline helper to locate the king for the side to move, used only for check detection:

```rust
fn king_square(pos: &Position, side: Color) -> Square {
    let king = Piece::new(side, PieceKind::King);
    let sq_idx = pos.piece_bitboard(king)
        .into_iter()
        .next()
        .expect("king must exist") as u8;
    Square::new(sq_idx).expect("valid square")
}
```

This mirrors the exact pattern used in `movegen::generate_legal_moves` (lines 36–42 of `crates/movegen/src/lib.rs`).

**`negamax` Function:**

```rust
pub fn negamax(
    pos: &mut Position,
    depth: u8,
    mut alpha: i32,
    beta: i32,
    ply: u8,
) -> (i32, Option<Move>)
```

Logic:
1. **Base case (depth == 0):** Return `(eval::evaluate(pos), None)`. This is a placeholder; T02 will replace this with quiescence search.
2. **Generate legal moves** via `movegen::generate_legal_moves(pos)`.
3. **No legal moves:**
   - Find the king square for the side to move: `king_square(pos, pos.side_to_move())`.
   - Determine opponent: `pos.side_to_move().opposite()`.
   - If `pos.is_square_attacked(king_sq, opponent)` → checkmate → return `(-MATE_SCORE + ply as i32, None)`.
   - Else → stalemate → return `(0, None)`.
4. **Move loop:** For each legal move `mv`:
   - `let undo = pos.make_move(mv);`
   - `let (score, _) = negamax(pos, depth - 1, -beta, -alpha, ply + 1);`
   - `let score = -score;`
   - `pos.unmake_move(mv, undo);`
   - If `score > alpha`:
     - Set `alpha = score` and `best_move = Some(mv)`.
     - If `alpha >= beta` → break (beta cutoff).
5. **Return** `(alpha, best_move)`.

Note: `alpha` is initialized to `-INFINITY` by the caller for the root call. Within the function, `best_move` starts as `None` and updates as alpha improves.

**Root Wrapper (for testing convenience):**

Not required by the task spec, so the tests will call `negamax` directly with `(-INFINITY, INFINITY)` as the initial window.

**Unit Tests (`#[cfg(test)] mod tests`):**

| Test | Setup | Assertion |
|------|-------|-----------|
| `checkmate_black_is_mated` | FEN: `"rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3"` (Scholar's mate, White to move, White is mated — no legal moves, king in check) | Returns `(-MATE_SCORE + ply, None)` where ply is 0 at root. Score is `(-MATE_SCORE + 0)` = `-30000`. |
| `checkmate_detected_at_depth` | FEN: `"rnb1kbnr/pppp1ppp/4p3/8/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 0 3"` — White is in checkmate | `negamax` at depth 1 returns `(-MATE_SCORE, None)` since the position has no legal moves and king is in check. |
| `stalemate_returns_zero` | FEN: `"k7/8/1K6/8/8/8/8/8 b - - 0 1"` — Black king on a8, White king on b6, Black to move, no legal moves, not in check | Returns `(0, None)`. |
| `returns_legal_move_at_depth` | Startpos, depth 1 | Returns `(score, Some(mv))` where `mv` is a valid legal move (exists in `generate_legal_moves`). |
| `prefers_capture_of_free_piece` | FEN: `"4k3/8/8/3q4/8/8/8/4K2R w - - 0 1"` — White rook on h1 can capture nothing, but depth 1 should pick a move; at depth 2+ with a position where a free piece is available, the best move captures it. Use FEN: `"4k3/8/8/8/8/8/3q4/R3K3 w - - 0 1"` — White rook on a1 can capture Black queen on d2 (free queen). At depth 1, should find Rxd2 or at least return positive score. | The returned score is positive (capturing the queen), and the returned move is a capture. |
| `alpha_beta_prunes` | Search startpos at depth 3 with full window and count nodes. Then compare — this is a sanity test that the function terminates in reasonable time at depth 3. No specific node count assertion needed; just verify it returns `Some(mv)` and terminates. | Returns `(_, Some(mv))` at depth 3. |

## 4. API Contracts

```rust
use chess_search::{negamax, MATE_SCORE, INFINITY};
use chess_board::Position;

let mut pos = Position::startpos();
let (score, best_move) = negamax(&mut pos, 4, -INFINITY, INFINITY, 0);
// score: i32 from the perspective of the side to move
// best_move: Option<Move> — the best move found, or None if no legal moves
```

N/A for REST/HTTP.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| `checkmate_black_is_mated` | Checkmate detected correctly when side to move has no legal moves and king is in check; returns `-MATE_SCORE + ply` |
| `checkmate_detected_at_depth` | Checkmate detected at the search leaf, returning correct mate score |
| `stalemate_returns_zero` | Stalemate returns draw score (0) when no legal moves and king not in check |
| `returns_legal_move_at_depth` | At depth > 0 with legal moves available, returns `Some(mv)` where `mv` is in the legal move list |
| `prefers_capture_of_free_piece` | Search finds an obvious winning capture (free queen), returning a positive score and a capture move |
| `alpha_beta_prunes` | Function terminates at depth 3 from startpos and returns a best move — validates that alpha-beta pruning makes search tractable |

Test setup: All tests use `Position::from_fen(...)` or `Position::startpos()` and call `negamax` directly with `(-INFINITY, INFINITY, 0)` as initial bounds.

## 6. Implementation Order

1. `Cargo.toml` (workspace root) — add `"crates/search"` to members
2. `crates/search/Cargo.toml` — create crate manifest with dependencies
3. `crates/search/src/lib.rs` — define `MATE_SCORE` and `INFINITY` constants
4. `crates/search/src/lib.rs` — implement `king_square` helper
5. `crates/search/src/lib.rs` — implement `negamax` function
6. `crates/search/src/lib.rs` — write unit tests

## 7. Verification Commands

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test -p chess-search
cargo test
```

## 8. Design Deviations

None.