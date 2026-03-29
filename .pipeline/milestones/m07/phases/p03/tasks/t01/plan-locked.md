I now have comprehensive knowledge of the entire codebase. Let me produce the task plan.

## 1. Deliverables

- **Create**: `crates/eval/src/endgame.rs` — New module containing material configuration detection, specialized mating pattern evaluators (KBN vs K, KR vs K, KQ vs K), insufficient material detection returning draw score, and unit tests for all patterns.
- **Modify**: `crates/eval/src/lib.rs` — Add `pub mod endgame;` declaration and integrate endgame bypass logic at the top of `evaluate()` so that when a specialized endgame is detected, the function returns the endgame-specific score directly, skipping the general evaluation pipeline.

## 2. Dependencies

- `chess-types` crate (already a dependency) — `Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`, `File`, `Rank`
- `chess-board` crate (already a dependency) — `Position` with `piece_bitboard()`, `side_to_move()`
- No new crate dependencies required.

## 3. Implementation Details

### `crates/eval/src/endgame.rs`

**Purpose**: Detect specialized endgame material configurations and return appropriate scores, bypassing the general evaluation.

**Public function**:
- `pub fn probe_endgame(pos: &Position) -> Option<i32>` — Returns `Some(score)` from white's perspective if a recognized endgame pattern is detected, or `None` if the position should use general evaluation. The caller adjusts for side-to-move perspective.

**Material configuration detection logic** (internal helper):
- Count pieces per side using `pos.piece_bitboard(Piece::new(color, kind)).pop_count()` for each `PieceKind`.
- Identify total piece counts per side (excluding kings, which are always present).
- Match against known patterns:
  - **K vs K**: Both sides have zero non-king pieces → return `0` (draw)
  - **KB vs K**: One side has exactly 1 bishop, no other non-king pieces; other side has zero non-king pieces → return `0` (draw)
  - **KN vs K**: One side has exactly 1 knight, no other non-king pieces; other side has zero non-king pieces → return `0` (draw)
  - **KNN vs K**: One side has exactly 2 knights, no other non-king pieces; other side has zero non-king pieces → return `0` (draw)
  - **KQ vs K**: One side has exactly 1 queen, no other non-king pieces; other side has zero non-king pieces → call KQ vs K evaluator
  - **KR vs K**: One side has exactly 1 rook, no other non-king pieces; other side has zero non-king pieces → call KR vs K evaluator
  - **KBN vs K**: One side has exactly 1 bishop + 1 knight, no other non-king pieces; other side has zero non-king pieces → call KBN vs K evaluator

**Scoring approach**: All evaluators return scores from white's perspective (positive = white winning). When the strong side is black, negate the score.

**Constants**:
- `const KNOWN_WIN: i32 = 10000;` — Base score for a known winning endgame (large enough to dominate material-based scoring)

**KR vs K evaluator** (`fn eval_kr_vs_k`):
- Score = `KNOWN_WIN + edge_distance_bonus(losing_king_sq)` where edge distance bonus rewards the losing king being closer to the edge (rank or file 0 or 7). Uses `min(rank, 7-rank, file, 7-file)` as center distance; lower = closer to edge = higher bonus.
- Additional bonus for winning king proximity to losing king (using Chebyshev distance: `max(|file_diff|, |rank_diff|)`) — closer winning king gets a higher bonus.

**KQ vs K evaluator** (`fn eval_kq_vs_k`):
- Same structure as KR vs K — drive losing king to edge, reward winning king proximity.

**KBN vs K evaluator** (`fn eval_kbn_vs_k`):
- Determine bishop square color: `(bishop_sq.file() as u8 + bishop_sq.rank() as u8) % 2`. If 0 → dark square bishop; if 1 → light square bishop.
- Correct corners for dark-square bishop: a1 (index 0) and h8 (index 63). Correct corners for light-square bishop: a8 (index 56) and h1 (index 7).
- Score = `KNOWN_WIN + correct_corner_bonus(losing_king_sq) + winning_king_proximity_bonus`.
- `correct_corner_bonus`: Chebyshev distance from losing king to the nearest correct corner. Smaller distance = higher bonus. Bonus = `(7 - min_chebyshev_distance_to_correct_corner) * CORNER_WEIGHT`.
- Winning king proximity bonus: `(7 - chebyshev_distance(winning_king, losing_king)) * KING_PROXIMITY_WEIGHT`.

**Helper functions** (private, within the module):
- `fn chebyshev_distance(sq1: Square, sq2: Square) -> i32` — `max(|file1 - file2|, |rank1 - rank2|)`, computed using `sq.file() as i32` and `sq.rank() as i32`.
- `fn edge_distance(sq: Square) -> i32` — `min(file, 7 - file, rank, 7 - rank)`, where 0 = on the edge, 3 = center.
- `fn king_square(pos: &Position, color: Color) -> Square` — extracts king square from `pos.piece_bitboard(Piece::new(color, PieceKind::King)).lsb()`, converting `u32` to `Square`.

**Unit tests** (`#[cfg(test)] mod tests`):
- `kbn_vs_k_white_wins_dark_bishop`: FEN with white KBN vs black K, dark-square bishop. Assert `probe_endgame` returns `Some(score)` with `score > 0`.
- `kbn_vs_k_black_wins_light_bishop`: FEN with black KBN vs white K, light-square bishop. Assert `probe_endgame` returns `Some(score)` with `score < 0`.
- `kbn_vs_k_corner_drives_correct`: Two FEN positions — losing king near correct corner vs. center. Assert score is higher (more winning) when losing king is closer to the correct corner.
- `kr_vs_k_white_wins`: FEN with white KR vs black K. Assert `probe_endgame` returns `Some(score)` with `score > 0`.
- `kr_vs_k_edge_better`: Two positions — losing king on edge vs. center. Assert edge position scores higher for the winning side.
- `kq_vs_k_white_wins`: FEN with white KQ vs black K. Assert `probe_endgame` returns `Some(score)` with `score > 0`.
- `k_vs_k_draw`: FEN `4k3/8/8/8/8/8/8/4K3 w - - 0 1`. Assert `probe_endgame` returns `Some(0)`.
- `kb_vs_k_draw`: FEN `4k3/8/8/8/8/8/8/4KB2 w - - 0 1`. Assert `probe_endgame` returns `Some(0)`.
- `kn_vs_k_draw`: FEN `4k3/8/8/8/8/8/8/4KN2 w - - 0 1`. Assert `probe_endgame` returns `Some(0)`.
- `knn_vs_k_draw`: FEN `4k3/8/8/8/8/8/8/3NKN2 w - - 0 1`. Assert `probe_endgame` returns `Some(0)`.
- `non_endgame_returns_none`: FEN with many pieces (e.g., startpos). Assert `probe_endgame` returns `None`.

### `crates/eval/src/lib.rs`

**Modifications**:

1. Add `pub mod endgame;` to the module declarations (after existing modules).

2. In `evaluate()`, insert endgame bypass at the very top of the function body, before any other evaluation:
```rust
if let Some(eg_score) = endgame::probe_endgame(pos) {
    return if pos.side_to_move() == Color::White {
        eg_score
    } else {
        -eg_score
    };
}
```
This short-circuits the entire evaluation pipeline (material+PST, pawn structure, king safety, mobility, piece bonuses, tapered eval) when a recognized endgame is detected.

## 4. API Contracts

**`endgame::probe_endgame(pos: &Position) -> Option<i32>`**

- **Input**: Immutable reference to a `Position`
- **Output**: `Some(score)` — score from white's perspective (positive = white winning) if the position is a recognized specialized endgame. `None` if the position does not match any recognized pattern.
- **Recognized patterns**:
  - `K vs K` → `Some(0)`
  - `KB vs K` → `Some(0)`
  - `KN vs K` → `Some(0)`
  - `KNN vs K` → `Some(0)`
  - `KR vs K` → `Some(winning_score)` (positive if white has the rook, negative if black)
  - `KQ vs K` → `Some(winning_score)` (positive if white has the queen, negative if black)
  - `KBN vs K` → `Some(winning_score)` (positive if white has the BN, negative if black)

N/A for REST/HTTP.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| `kbn_vs_k_white_wins_dark_bishop` | KBN vs K detected for white with dark-square bishop, returns positive score |
| `kbn_vs_k_black_wins_light_bishop` | KBN vs K detected for black with light-square bishop, returns negative score |
| `kbn_vs_k_corner_drives_correct` | Score is higher when losing king is near the correct corner vs. center |
| `kr_vs_k_white_wins` | KR vs K detected, returns positive score for white |
| `kr_vs_k_edge_better` | Losing king on edge produces higher winning score than king in center |
| `kq_vs_k_white_wins` | KQ vs K detected, returns positive score for white |
| `k_vs_k_draw` | K vs K returns draw score (0) |
| `kb_vs_k_draw` | KB vs K returns draw score (0) |
| `kn_vs_k_draw` | KN vs K returns draw score (0) |
| `knn_vs_k_draw` | KNN vs K returns draw score (0) |
| `non_endgame_returns_none` | Non-endgame position returns `None`, letting general evaluation proceed |

## 6. Implementation Order

1. `crates/eval/src/endgame.rs` — implement helper functions (`chebyshev_distance`, `edge_distance`, `king_square`), then material detection logic in `probe_endgame`, then individual evaluators (`eval_kr_vs_k`, `eval_kq_vs_k`, `eval_kbn_vs_k`), then drawn endgame detection, then unit tests.
2. `crates/eval/src/lib.rs` — add `pub mod endgame;` declaration and insert endgame bypass at the top of `evaluate()`.

## 7. Verification Commands

```bash
cargo test -p chess-eval
cargo clippy -p chess-eval -- -D warnings
cargo fmt -p chess-eval --check
cargo test --workspace
```

## 8. Design Deviations

None.