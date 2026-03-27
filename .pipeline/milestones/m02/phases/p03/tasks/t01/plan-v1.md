Now I have all the information needed. Let me produce the task plan.

## 1. Deliverables

- **`crates/movegen/src/castling.rs`** (new) — Castling move generation module implementing `generate_castling_moves(pos: &Position, moves: &mut Vec<Move>)` with unit tests.
- **`crates/movegen/src/lib.rs`** (modified) — Add `castling` module declaration and `pub use castling::generate_castling_moves` export.

## 2. Dependencies

- `chess-types` (already a dependency of `chess-movegen`) — provides `Move`, `MoveFlag`, `Square`, `Bitboard`, `Piece`, `PieceKind`, `Color`.
- `chess-board` (already a dependency of `chess-movegen`) — provides `Position`, `CastlingRights`.
- No new dependencies required.

## 3. Implementation Details

### `crates/movegen/src/castling.rs`

**Purpose**: Generate O-O and O-O-O castling moves for the side to move.

**Imports**:
```rust
use chess_board::{CastlingRights, Position};
use chess_types::{Bitboard, Color, Move, MoveFlag, Piece, PieceKind, Square};
```

**Constants** — define the intermediate squares that must be clear, and the transit squares the king passes through (including origin and destination) for attack checking:

| Castling | King From | King To | Path Clear Mask (between king and rook) | King Transit Squares (for attack check) |
|----------|-----------|---------|----------------------------------------|----------------------------------------|
| White Kingside  | E1 | G1 | F1, G1 | E1, F1, G1 |
| White Queenside | E1 | C1 | B1, C1, D1 | E1, D1, C1 |
| Black Kingside  | E8 | G8 | F8, G8 | E8, F8, G8 |
| Black Queenside | E8 | C8 | B8, C8, D8 | E8, D8, C8 |

Define these as `Bitboard` constants for path clearance and `[Square; 3]` arrays for attack checking:

```rust
const WHITE_KS_PATH: Bitboard = Bitboard::new((1u64 << 5) | (1u64 << 6));       // F1, G1
const WHITE_QS_PATH: Bitboard = Bitboard::new((1u64 << 1) | (1u64 << 2) | (1u64 << 3)); // B1, C1, D1
const BLACK_KS_PATH: Bitboard = Bitboard::new((1u64 << 61) | (1u64 << 62));     // F8, G8
const BLACK_QS_PATH: Bitboard = Bitboard::new((1u64 << 57) | (1u64 << 58) | (1u64 << 59)); // B8, C8, D8

const WHITE_KS_KING_TRANSIT: [Square; 3] = [Square::E1, Square::F1, Square::G1];
const WHITE_QS_KING_TRANSIT: [Square; 3] = [Square::E1, Square::D1, Square::C1];
const BLACK_KS_KING_TRANSIT: [Square; 3] = [Square::E8, Square::F8, Square::G8];
const BLACK_QS_KING_TRANSIT: [Square; 3] = [Square::E8, Square::D8, Square::C8];
```

**Public function**:
```rust
pub fn generate_castling_moves(pos: &Position, moves: &mut Vec<Move>)
```

**Logic**:
1. Determine side to move and opponent color.
2. Based on side to move, check two castling possibilities (kingside and queenside):
   - **Check castling right**: `pos.castling_rights().contains(CastlingRights::WHITE_KINGSIDE)` (etc.)
   - **Check path clear**: `(pos.occupied() & PATH_MASK).is_empty()`
   - **Check no attacked transit squares**: iterate through the 3 king transit squares, call `pos.is_square_attacked(sq, opponent)` — if any is attacked, skip this castling.
3. If all three conditions pass, emit the move:
   - Kingside: `Move::new(king_from, king_to, MoveFlag::KING_CASTLE)`
   - Queenside: `Move::new(king_from, king_to, MoveFlag::QUEEN_CASTLE)`

**Structure**: Use a helper to avoid duplicating the three-condition check four times. A private function or a `struct CastleSpec` with the right flag, path mask, transit squares, from/to squares, and move flag could keep the code DRY. However, since there are only four cases and inlining is straightforward, the implementation may either inline each case within a match on `Color` or use a static array of specs — follow whichever is cleaner. The simplest approach: match on `pos.side_to_move()`, then check kingside and queenside sequentially within each arm.

**Unit tests** (`#[cfg(test)] mod tests`):

Follow the same testing pattern as `king.rs` — use `has_move` helper, create positions from FEN, generate moves, assert presence/absence.

| Test | FEN | What it verifies |
|------|-----|-----------------|
| `test_white_kingside_castling` | `r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1` | White O-O is generated |
| `test_white_queenside_castling` | Same FEN | White O-O-O is generated |
| `test_black_kingside_castling` | Same FEN but `b` to move | Black O-O is generated |
| `test_black_queenside_castling` | Same FEN but `b` to move | Black O-O-O is generated |
| `test_castling_blocked_by_pieces` | `r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R2QK2R w KQkq - 0 1` | Queenside blocked by piece on D1, not generated |
| `test_castling_blocked_by_attacked_origin` | `r3k2r/pppppppp/8/8/4r3/8/PPPP1PPP/R3K2R w KQkq - 0 1` | King on E1 attacked by rook on E4, no castling |
| `test_castling_blocked_by_attacked_transit` | `r3k2r/pppppppp/8/8/5b2/8/PPPPPPPP/R3K2R w KQkq - 0 1` | F1 attacked by bishop, kingside blocked |
| `test_castling_blocked_by_attacked_destination` | Position where G1 is attacked | Kingside castling not generated |
| `test_castling_rights_revoked` | `r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w - - 0 1` | No castling rights, no castling moves generated |

### `crates/movegen/src/lib.rs`

**Changes**:
1. Add `pub mod castling;` to the module declarations.
2. Add `pub use castling::generate_castling_moves;` to the re-exports.

## 4. API Contracts

```rust
use chess_movegen::generate_castling_moves;
use chess_board::Position;
use chess_types::Move;

let pos = Position::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
let mut moves: Vec<Move> = Vec::new();
generate_castling_moves(&pos, &mut moves);
// moves contains Move::new(Square::E1, Square::G1, MoveFlag::KING_CASTLE)
//             and Move::new(Square::E1, Square::C1, MoveFlag::QUEEN_CASTLE)
```

N/A for REST/HTTP.

## 5. Test Plan

All tests are colocated in `crates/movegen/src/castling.rs` under `#[cfg(test)] mod tests`, following the existing crate convention.

**Test setup**: Use `Position::from_fen(...)` to construct positions. Use a `has_move(moves, from, to, flags) -> bool` helper (same pattern as `king.rs`).

| Test | What it verifies |
|------|-----------------|
| `test_white_kingside_castling` | O-O generated when right present, path clear, no attacked squares |
| `test_white_queenside_castling` | O-O-O generated when right present, path clear, no attacked squares |
| `test_black_kingside_castling` | Black O-O generated correctly |
| `test_black_queenside_castling` | Black O-O-O generated correctly |
| `test_castling_blocked_by_pieces` | Castling not generated when pieces occupy intermediate squares |
| `test_castling_blocked_by_attacked_origin` | Castling not generated when king's origin square is attacked |
| `test_castling_blocked_by_attacked_transit` | Castling not generated when a transit square is attacked |
| `test_castling_blocked_by_attacked_destination` | Castling not generated when king's destination square is attacked |
| `test_castling_rights_revoked` | No castling moves generated when castling rights are absent |

These tests directly map to the verification criteria in the task spec: castling available/generated, blocked by pieces, blocked by attacked squares (origin, transit, destination), rights revoked, and both colors.

## 6. Implementation Order

1. **`crates/movegen/src/castling.rs`** — define constants (path masks, transit squares), implement `generate_castling_moves`, write all unit tests.
2. **`crates/movegen/src/lib.rs`** — add `pub mod castling;` and `pub use castling::generate_castling_moves;`.

## 7. Verification Commands

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test -p chess-movegen
```

## 8. Design Deviations

None.