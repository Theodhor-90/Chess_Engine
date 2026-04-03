I now have all the information needed. Let me produce the task plan.

## 1. Deliverables

- **`Cargo.toml` (workspace root)** — update `members` list to include `crates/eval`
- **`crates/eval/Cargo.toml`** — crate manifest with dependencies on `chess-types` and `chess-board`
- **`crates/eval/src/lib.rs`** — public evaluation API (`evaluate` function) and module declarations
- **`crates/eval/src/material.rs`** — material value constants and material counting logic, with unit tests

## 2. Dependencies

- `chess-types` crate (path dependency `../types`) — provides `Bitboard`, `Color`, `Piece`, `PieceKind`
- `chess-board` crate (path dependency `../board`) — provides `Position` with `piece_bitboard()`, `occupied_by()`, `side_to_move()`
- No new external crates required.

## 3. Implementation Details

### `Cargo.toml` (workspace root)
- Add `"crates/eval"` to the workspace `members` array, after `"crates/movegen"`:
  ```toml
  members = ["crates/types", "crates/board", "crates/movegen", "crates/eval", "tools/perft"]
  ```

### `crates/eval/Cargo.toml`
- Follow the naming convention established by sibling crates (`chess-board`, `chess-movegen`):
  ```toml
  [package]
  name = "chess-eval"
  version = "0.1.0"
  edition.workspace = true

  [dependencies]
  chess-types = { path = "../types" }
  chess-board = { path = "../board" }
  ```

### `crates/eval/src/material.rs`
- **Constants**: Define named material value constants as `pub const` values of type `i32`:
  - `PAWN_VALUE: i32 = 100`
  - `KNIGHT_VALUE: i32 = 320`
  - `BISHOP_VALUE: i32 = 330`
  - `ROOK_VALUE: i32 = 500`
  - `QUEEN_VALUE: i32 = 900`
- **Helper function** `piece_value(kind: PieceKind) -> i32`: match on `PieceKind` to return the corresponding constant. `PieceKind::King` returns 0 (kings are not counted in material balance).
- **Function** `material_score(pos: &Position) -> i32`: Iterate over the 5 non-king piece kinds (Pawn, Knight, Bishop, Rook, Queen). For each kind, get the white piece bitboard via `pos.piece_bitboard(Piece::new(Color::White, kind))` and the black piece bitboard via `pos.piece_bitboard(Piece::new(Color::Black, kind))`. Multiply each bitboard's `pop_count()` by `piece_value(kind)`. Accumulate `white_material - black_material`. Return the raw white-relative score (the caller in `lib.rs` handles side-to-move conversion).
- **Unit tests** (`#[cfg(test)] mod tests`):
  - `equal_material_is_zero`: Starting position FEN → `material_score` returns 0.
  - `white_extra_queen`: FEN with white having an extra queen → returns positive value (900).
  - `black_extra_knight`: FEN with black having an extra knight, white to move → returns negative value (-320).

### `crates/eval/src/lib.rs`
- Declare `pub mod material;`
- **Public function** `pub fn evaluate(pos: &Position) -> i32`:
  - Call `material::material_score(pos)` to get the white-relative material score.
  - If `pos.side_to_move() == Color::White`, return the score as-is.
  - If `pos.side_to_move() == Color::Black`, return `-score` (negated).
- **Re-exports**: `pub use material::{PAWN_VALUE, KNIGHT_VALUE, BISHOP_VALUE, ROOK_VALUE, QUEEN_VALUE};` — these constants will be needed by T03 (phase calculation) and later phases.
- **Unit tests** (`#[cfg(test)] mod tests`):
  - `startpos_eval_is_zero`: Starting position → `evaluate` returns 0.
  - `white_advantage_positive_for_white`: FEN with white to move and extra material → `evaluate` returns positive.
  - `white_advantage_negative_for_black`: Same material advantage for white, but black to move → `evaluate` returns negative.

## 4. API Contracts

```rust
use chess_board::Position;
use chess_eval::evaluate;

let pos = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
let score: i32 = evaluate(&pos);
// score == 0 (equal material, white to move)
```

```rust
use chess_eval::{PAWN_VALUE, KNIGHT_VALUE, BISHOP_VALUE, ROOK_VALUE, QUEEN_VALUE};
// PAWN_VALUE == 100, KNIGHT_VALUE == 320, BISHOP_VALUE == 330, ROOK_VALUE == 500, QUEEN_VALUE == 900
```

N/A for REST/HTTP.

## 5. Test Plan

All tests are colocated in source files via `#[cfg(test)] mod tests`, following the project convention.

| Test | File | What it verifies |
|------|------|-----------------|
| `equal_material_is_zero` | `material.rs` | Starting position has material score 0 |
| `white_extra_queen` | `material.rs` | Position with white extra queen returns +900 |
| `black_extra_knight` | `material.rs` | Position with black extra knight returns -320 |
| `startpos_eval_is_zero` | `lib.rs` | `evaluate` returns 0 for starting position |
| `white_advantage_positive_for_white` | `lib.rs` | Side-to-move conversion: white to move, white ahead → positive |
| `white_advantage_negative_for_black` | `lib.rs` | Side-to-move conversion: black to move, white ahead → negative |

## 6. Implementation Order

1. **`Cargo.toml` (workspace root)** — add `"crates/eval"` to workspace members
2. **`crates/eval/Cargo.toml`** — create crate manifest
3. **`crates/eval/src/material.rs`** — implement material constants, `piece_value`, `material_score`, and unit tests
4. **`crates/eval/src/lib.rs`** — implement `evaluate` with side-to-move conversion, re-exports, and unit tests

## 7. Verification Commands

```bash
cargo build -p chess-eval
cargo test -p chess-eval
cargo fmt --check
cargo clippy -- -D warnings
```

## 8. Design Deviations

None.