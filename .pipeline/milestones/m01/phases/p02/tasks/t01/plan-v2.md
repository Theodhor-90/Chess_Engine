## 1. Deliverables

- **`crates/board/Cargo.toml`** — New crate manifest for the `chess-board` crate, depending on `chess-types` via relative path
- **`crates/board/src/lib.rs`** — Crate root, declares modules and re-exports the public API (`Position`, `CastlingRights`)
- **`crates/board/src/position.rs`** — `Position` struct with all fields, `CastlingRights` type, `Position::startpos()` constructor, accessor methods, and colocated unit tests
- **`Cargo.toml` (workspace root)** — Updated to add `"crates/board"` to workspace members

## 2. Dependencies

- **`chess-types`** (local path dependency `../types`) — provides `Bitboard`, `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank`
- No external crates are needed for this task. The `thiserror` dependency is not required until T02 (FEN parsing).

## 3. Implementation Details

### `crates/board/Cargo.toml`

```toml
[package]
name = "chess-board"
version = "0.1.0"
edition.workspace = true

[dependencies]
chess-types = { path = "../types" }
```

### `crates/board/src/lib.rs`

- Declare `pub mod position;`
- Re-export: `pub use position::{Position, CastlingRights};`

### `crates/board/src/position.rs`

#### `CastlingRights`

A compact representation of the four castling flags. Stored as a `u8` bitfield:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct CastlingRights(u8);
```

Constants:
- `WHITE_KINGSIDE: u8 = 0b0001`
- `WHITE_QUEENSIDE: u8 = 0b0010`
- `BLACK_KINGSIDE: u8 = 0b0100`
- `BLACK_QUEENSIDE: u8 = 0b1000`
- `ALL: CastlingRights = CastlingRights(0b1111)`
- `NONE: CastlingRights = CastlingRights(0)`

Methods:
- `contains(self, flag: u8) -> bool` — check if a specific right is set
- `set(&mut self, flag: u8)` — enable a right
- `remove(&mut self, flag: u8)` — disable a right

This fulfills the phase spec's design decision #3: "Castling availability stored as a compact representation with four boolean flags."

#### `Position`

```rust
#[derive(Clone, Debug)]
pub struct Position {
    piece_bb: [Bitboard; 12],        // indexed by Piece::index()
    occupied_by: [Bitboard; 2],      // indexed by Color as usize
    occupied: Bitboard,              // all occupied squares
    side_to_move: Color,
    castling_rights: CastlingRights,
    en_passant: Option<Square>,
    halfmove_clock: u8,
    fullmove_counter: u16,
}
```

Field notes:
- `piece_bb[12]` — one bitboard per piece-color (white pawn=0, white knight=1, ..., black king=11), indexed via `Piece::index()` which computes `color as usize * 6 + kind as usize`
- `occupied_by[2]` — per-color occupancy, indexed by `Color as usize`
- `occupied` — union of all pieces

#### `Position::startpos()`

Returns the standard starting position. Implementation:
1. Set piece bitboards for all 32 pieces on their starting squares using `Bitboard::new()` with the appropriate bitmask for each piece type:
   - White pawns: rank 2 (bits 8–15 → `0x0000_0000_0000_FF00`)
   - White rooks: a1, h1 → `0x0000_0000_0000_0081`
   - White knights: b1, g1 → `0x0000_0000_0000_0042`
   - White bishops: c1, f1 → `0x0000_0000_0000_0024`
   - White queen: d1 → `0x0000_0000_0000_0008`
   - White king: e1 → `0x0000_0000_0000_0010`
   - Black pieces mirror on ranks 7–8 (shift left by 48 for pawns on rank 7, shift left by 56 for back rank)
2. Compute occupancy bitboards from piece bitboards
3. Set `side_to_move = Color::White`, `castling_rights = CastlingRights::ALL`, `en_passant = None`, `halfmove_clock = 0`, `fullmove_counter = 1`

#### Accessor Methods

- `piece_bitboard(&self, piece: Piece) -> Bitboard` — returns `self.piece_bb[piece.index()]`
- `occupied_by(&self, color: Color) -> Bitboard` — returns `self.occupied_by[color as usize]`
- `occupied(&self) -> Bitboard` — returns `self.occupied`
- `piece_on(&self, square: Square) -> Option<Piece>` — iterates through all 12 piece bitboards checking if the square is set; returns the first matching `Piece` via `Piece::from_index()`, or `None` if the square is empty
- `side_to_move(&self) -> Color`
- `castling_rights(&self) -> CastlingRights`
- `en_passant(&self) -> Option<Square>`
- `halfmove_clock(&self) -> u8`
- `fullmove_counter(&self) -> u16`

Implementation note for `piece_on`: Check `self.occupied` first for a fast-path empty-square return. Then iterate `0..12`, testing `self.piece_bb[i] & Bitboard::new(1u64 << square.index())` for each.

### `Cargo.toml` (workspace root)

Change `members = ["crates/types"]` to `members = ["crates/types", "crates/board"]`.

## 4. API Contracts

```rust
use chess_types::{Bitboard, Color, Piece, PieceKind, Square};
use chess_board::{Position, CastlingRights};

// Construction
let pos = Position::startpos();

// Query piece placement
let wp_bb: Bitboard = pos.piece_bitboard(Piece::new(Color::White, PieceKind::Pawn));
let piece: Option<Piece> = pos.piece_on(Square::E1); // Some(White King)
let piece: Option<Piece> = pos.piece_on(Square::E4); // None

// Query occupancy
let white_occ: Bitboard = pos.occupied_by(Color::White);
let all_occ: Bitboard = pos.occupied();

// Query game state
let stm: Color = pos.side_to_move();           // White
let cr: CastlingRights = pos.castling_rights(); // All set
let ep: Option<Square> = pos.en_passant();      // None
let hmc: u8 = pos.halfmove_clock();             // 0
let fmc: u16 = pos.fullmove_counter();          // 1
```

N/A for REST/HTTP.

## 5. Test Plan

All tests are colocated in `crates/board/src/position.rs` under `#[cfg(test)] mod tests`.

| Test | What it verifies |
|------|-----------------|
| `startpos_piece_placement` | Each of the 12 piece bitboards in `startpos()` has the correct squares set (pop count and specific bit positions for all 32 pieces) |
| `startpos_occupancy` | `occupied_by(White)` contains 16 set bits on ranks 1–2, `occupied_by(Black)` contains 16 set bits on ranks 7–8, `occupied()` contains 32 set bits total |
| `startpos_game_state` | `side_to_move() == White`, all four castling rights set, `en_passant() == None`, `halfmove_clock() == 0`, `fullmove_counter() == 1` |
| `piece_on_startpos` | `piece_on(E1) == Some(White King)`, `piece_on(D8) == Some(Black Queen)`, `piece_on(A2) == Some(White Pawn)`, `piece_on(E4) == None` |
| `occupancy_consistency` | `occupied_by(White) | occupied_by(Black) == occupied()` and no overlap between `occupied_by(White)` and `occupied_by(Black)` |

## 6. Implementation Order

1. `Cargo.toml` (workspace root) — add `"crates/board"` to workspace members
2. `crates/board/Cargo.toml` — create crate manifest
3. `crates/board/src/position.rs` — implement `CastlingRights`, `Position` struct, `startpos()`, accessor methods, and unit tests
4. `crates/board/src/lib.rs` — declare module and re-exports
5. Verify with `cargo test -p chess-board` and `cargo clippy -p chess-board -- -D warnings`

## 7. Verification Commands

```bash
cargo fmt --check
cargo clippy -p chess-board -- -D warnings
cargo test -p chess-board
```

## 8. Design Deviations

None.