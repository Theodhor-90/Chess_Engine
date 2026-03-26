Good, `thiserror` is already in the workspace (version 2, used by `chess-types`). Now I have all the information needed to produce the implementation plan.

## 1. Deliverables

| # | File | Purpose |
|---|------|---------|
| 1 | `crates/board/Cargo.toml` | Add `thiserror = "2"` dependency for `FenError` |
| 2 | `crates/board/src/fen.rs` | New file: `FenError` enum, `Position::from_fen` logic, `Position::to_fen` logic, and colocated unit tests |
| 3 | `crates/board/src/lib.rs` | Add `mod fen;` declaration and re-export `FenError` |
| 4 | `crates/board/src/position.rs` | Add `from_fen` and `to_fen` methods to `impl Position` (delegating to/from `fen.rs` internals, or implemented directly — see Implementation Details) |

## 2. Dependencies

- **`thiserror = "2"`** — must be added to `crates/board/Cargo.toml`. Already present in the workspace via `crates/types/Cargo.toml`, so Cargo will resolve it from the lockfile.
- **T01 (Position Struct)** — completed. Provides `Position`, `CastlingRights`, and all fields/accessors.
- **`chess-types` crate** — already a dependency. Provides `Bitboard`, `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank`.

## 3. Implementation Details

### 3.1 `crates/board/Cargo.toml`

Add `thiserror = "2"` under `[dependencies]` alongside the existing `chess-types` dependency.

### 3.2 `crates/board/src/fen.rs`

**Purpose**: Contains `FenError` type definition and the parsing/serialization helper functions.

**Exports**: `FenError`

#### `FenError` enum

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FenError {
    #[error("invalid FEN: expected 6 space-separated fields, got {0}")]
    WrongFieldCount(usize),
    #[error("invalid piece placement: {0}")]
    InvalidPiecePlacement(String),
    #[error("invalid active color: expected 'w' or 'b', got '{0}'")]
    InvalidActiveColor(String),
    #[error("invalid castling availability: '{0}'")]
    InvalidCastling(String),
    #[error("invalid en passant square: '{0}'")]
    InvalidEnPassant(String),
    #[error("invalid halfmove clock: '{0}'")]
    InvalidHalfmoveClock(String),
    #[error("invalid fullmove number: '{0}'")]
    InvalidFullmoveNumber(String),
}
```

#### `parse_fen` function (crate-internal)

Signature: `pub(crate) fn parse_fen(fen: &str) -> Result<Position, FenError>`

Logic:
1. Split `fen` on whitespace, collect into a `Vec<&str>`. If length ≠ 6, return `FenError::WrongFieldCount`.
2. **Piece placement (field 0)**: Split by `/`. Validate exactly 8 ranks. Iterate ranks from rank 8 (index 7) down to rank 1 (index 0) — FEN lists rank 8 first. For each rank string:
   - Track file index 0..7.
   - For digit characters `'1'`–`'8'`: advance file index by that digit value.
   - For piece characters `KQRBNPkqrbnp`: map to `Piece` using a match on the character, place on the square `Square::from_file_rank(file, rank)` by setting the bit in the appropriate `piece_bb[piece.index()]`. Advance file by 1.
   - If any rank doesn't sum to exactly 8 squares, return `FenError::InvalidPiecePlacement`.
   - If an unrecognized character is found, return `FenError::InvalidPiecePlacement`.
   - After all ranks, compute `occupied_by[White]` (OR of `piece_bb[0..6]`), `occupied_by[Black]` (OR of `piece_bb[6..12]`), and `occupied` (white | black).
3. **Active color (field 1)**: Match `"w"` → `Color::White`, `"b"` → `Color::Black`. Otherwise `FenError::InvalidActiveColor`.
4. **Castling availability (field 2)**: If `"-"`, set `CastlingRights::NONE`. Otherwise iterate characters: `'K'` → set `WHITE_KINGSIDE`, `'Q'` → set `WHITE_QUEENSIDE`, `'k'` → set `BLACK_KINGSIDE`, `'q'` → set `BLACK_QUEENSIDE`. Any other character or empty string → `FenError::InvalidCastling`. Also validate no duplicate characters.
5. **En passant (field 3)**: If `"-"`, set `None`. Otherwise parse as algebraic square: first char must be `'a'`–`'h'` (file), second char must be `'3'` or `'6'` (valid en passant ranks), length must be exactly 2. Convert to `Square::from_file_rank(file, rank)`. On failure → `FenError::InvalidEnPassant`.
6. **Halfmove clock (field 4)**: Parse as `u8`. On failure → `FenError::InvalidHalfmoveClock`.
7. **Fullmove number (field 5)**: Parse as `u16`. Must be ≥ 1. On failure → `FenError::InvalidFullmoveNumber`.
8. Construct and return the `Position` with all fields.

Character-to-piece mapping:
- `'P'` → `Piece::new(Color::White, PieceKind::Pawn)`, `'N'` → Knight, `'B'` → Bishop, `'R'` → Rook, `'Q'` → Queen, `'K'` → King
- `'p'` → `Piece::new(Color::Black, PieceKind::Pawn)`, etc.

#### `format_fen` function (crate-internal)

Signature: `pub(crate) fn format_fen(pos: &Position) -> String`

Logic:
1. **Piece placement**: Iterate ranks from 7 (rank 8) down to 0 (rank 1). For each rank, iterate files 0..8. Call `pos.piece_on(square)`. If `Some(piece)`, flush any accumulated empty count as a digit, then append the piece character (using the Display impl on `Piece` which already produces `KQRBNPkqrbnp`). If `None`, increment empty counter. After each rank, flush empty count. Separate ranks with `/`.
2. **Active color**: `pos.side_to_move()` — `Color::White` → `"w"`, `Color::Black` → `"b"`.
3. **Castling availability**: Check each of the four flags in order `K`, `Q`, `k`, `q`. If none set, output `"-"`.
4. **En passant**: `pos.en_passant()` — if `Some(sq)`, format as algebraic (use `Square::Display` which outputs e.g. `"e3"`). If `None`, output `"-"`.
5. **Halfmove clock**: `pos.halfmove_clock()` as decimal.
6. **Fullmove number**: `pos.fullmove_counter()` as decimal.
7. Join all six fields with spaces.

### 3.3 `crates/board/src/lib.rs`

Add `pub mod fen;` and add `FenError` to the re-exports:

```rust
pub mod fen;
pub mod position;

pub use fen::FenError;
pub use position::{CastlingRights, Position};
```

### 3.4 `crates/board/src/position.rs`

Add `from_fen` and `to_fen` methods to `impl Position`. These delegate to `fen.rs`:

```rust
use crate::fen::{self, FenError};

impl Position {
    pub fn from_fen(fen: &str) -> Result<Position, FenError> {
        fen::parse_fen(fen)
    }

    pub fn to_fen(&self) -> String {
        fen::format_fen(self)
    }
}
```

This requires `Position` fields to be accessible from `fen.rs`. Since both modules are within the same crate, `pub(crate)` visibility on the `Position` fields is sufficient. Currently the fields are private (no `pub` modifier), so a constructor function is needed. The cleanest approach: add a `pub(crate)` constructor in `position.rs` that takes all fields:

```rust
impl Position {
    pub(crate) fn new(
        piece_bb: [Bitboard; 12],
        occupied_by: [Bitboard; 2],
        occupied: Bitboard,
        side_to_move: Color,
        castling_rights: CastlingRights,
        en_passant: Option<Square>,
        halfmove_clock: u8,
        fullmove_counter: u16,
    ) -> Position {
        Position {
            piece_bb,
            occupied_by,
            occupied,
            side_to_move,
            castling_rights,
            en_passant,
            halfmove_clock,
            fullmove_counter,
        }
    }
}
```

This allows `fen.rs` to construct a `Position` without making fields `pub`.

## 4. API Contracts

### `Position::from_fen`

**Input**: `fen: &str` — a FEN string with 6 space-separated fields.

**Output**: `Result<Position, FenError>`

**Examples**:
```rust
// Success
let pos = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
assert_eq!(pos.side_to_move(), Color::White);

// Error: wrong field count
let err = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w");
assert!(err.is_err());
```

### `Position::to_fen`

**Input**: `&self` — a valid `Position`.

**Output**: `String` — canonical FEN string.

**Example**:
```rust
let pos = Position::startpos();
assert_eq!(pos.to_fen(), "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
```

### `FenError`

Enum with variants: `WrongFieldCount(usize)`, `InvalidPiecePlacement(String)`, `InvalidActiveColor(String)`, `InvalidCastling(String)`, `InvalidEnPassant(String)`, `InvalidHalfmoveClock(String)`, `InvalidFullmoveNumber(String)`. Implements `std::error::Error` and `Display` via `thiserror`.

## 5. Test Plan

All tests are colocated in `crates/board/src/fen.rs` under `#[cfg(test)] mod tests`.

### Test Setup

Each test constructs positions via `Position::from_fen` and/or `Position::startpos()`.

### Tests

**T1: `startpos_round_trip`**
- Parse the starting position FEN `"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"`
- Assert `from_fen` succeeds
- Assert `to_fen()` produces the exact same string
- Verify `side_to_move() == Color::White`, `castling_rights() == CastlingRights::ALL`, `en_passant() == None`, `halfmove_clock() == 0`, `fullmove_counter() == 1`

**T2: `middlegame_round_trips`**
- Parse each of these FEN strings:
  - `"r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2"` (Alekhine's Defense)
  - `"r1bqkb1r/pppppppp/2n2n2/8/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3"` (Italian-like)
  - `"rnbqk2r/pppp1ppp/4pn2/8/1bPP4/2N5/PP2PPPP/R1BQKBNR w KQkq - 2 4"` (Nimzo-Indian)
- For each: assert `from_fen` succeeds and `to_fen()` equals the input string
- Spot-check active color and a few piece placements

**T3: `endgame_round_trips`**
- Parse each of these FEN strings:
  - `"8/8/4k3/8/8/4K3/4P3/8 w - - 0 1"` (king+pawn vs king)
  - `"8/5k2/8/8/8/2K5/8/4R3 w - - 0 1"` (rook endgame)
- For each: assert `from_fen` succeeds and `to_fen()` equals the input string
- Verify castling rights are `CastlingRights::NONE` and en passant is `None`

**T4: `startpos_from_fen_matches_startpos`**
- Parse starting FEN, compare piece bitboards with `Position::startpos()` to verify they agree on every field

**T5: `reject_wrong_field_count`**
- FEN with 5 fields (missing fullmove): assert `Err` matches `FenError::WrongFieldCount`
- FEN with 7 fields: assert `Err` matches `FenError::WrongFieldCount`

**T6: `reject_invalid_piece_placement`**
- Rank with 9 squares (e.g., `"rnbqkbnrr/..."`) → `FenError::InvalidPiecePlacement`
- Invalid piece character (e.g., `"xnbqkbnr/..."`) → `FenError::InvalidPiecePlacement`
- Only 7 ranks → `FenError::InvalidPiecePlacement`

**T7: `reject_invalid_active_color`**
- Active color field `"x"` → `FenError::InvalidActiveColor`

**T8: `reject_invalid_castling`**
- Castling field `"X"` → `FenError::InvalidCastling`

**T9: `reject_invalid_en_passant`**
- En passant field `"e9"` → `FenError::InvalidEnPassant`
- En passant field `"z3"` → `FenError::InvalidEnPassant`

**T10: `reject_invalid_clocks`**
- Halfmove clock `"abc"` → `FenError::InvalidHalfmoveClock`
- Fullmove number `"0"` → `FenError::InvalidFullmoveNumber`
- Fullmove number `"-1"` → `FenError::InvalidFullmoveNumber`

## 6. Implementation Order

1. **Add `thiserror` dependency** to `crates/board/Cargo.toml`
2. **Add `pub(crate) fn new(...)` constructor** to `Position` in `crates/board/src/position.rs` — enables `fen.rs` to construct positions without exposing fields publicly
3. **Create `crates/board/src/fen.rs`** with:
   - `FenError` enum (with `thiserror` derive)
   - `pub(crate) fn parse_fen(fen: &str) -> Result<Position, FenError>` — the full FEN parser
   - `pub(crate) fn format_fen(pos: &Position) -> String` — the FEN serializer
   - `#[cfg(test)] mod tests` — all unit tests
4. **Add `from_fen` and `to_fen` methods** to `impl Position` in `crates/board/src/position.rs`
5. **Update `crates/board/src/lib.rs`** — add `pub mod fen;` and `pub use fen::FenError;`
6. **Run verification commands** to confirm everything passes

## 7. Verification Commands

```bash
# Format check
cargo fmt --check --manifest-path crates/board/Cargo.toml

# Lint check
cargo clippy -p chess-board -- -D warnings

# Run all tests in the board crate
cargo test -p chess-board

# Run all workspace tests to ensure nothing is broken
cargo test --workspace
```

## 8. Design Deviations

**Deviation 1: `pub(crate)` constructor instead of `pub` fields**

- **Parent spec states**: `crates/board/src/position.rs` — "may need to add the `from_fen` and `to_fen` methods to `Position`"
- **Issue**: The `Position` struct fields are private (no `pub` modifier), and `fen.rs` is a separate module that needs to construct `Position` instances. Making all fields `pub` would break encapsulation. Adding `pub(crate)` to each field would be invasive.
- **Alternative chosen**: Add a `pub(crate) fn new(...)` constructor to `Position` that accepts all fields. This allows `fen.rs` to construct positions while keeping fields private to external consumers. This is the minimal change and follows Rust encapsulation conventions.