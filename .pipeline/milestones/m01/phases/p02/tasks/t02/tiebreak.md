I now have all the information I need. Here is the definitive implementation plan:

# T02 — FEN Parser and Serializer: Implementation Plan

## 1. Deliverables

| # | File | Purpose |
|---|------|---------|
| 1 | `crates/board/Cargo.toml` | Add `thiserror = "2"` dependency for `FenError` |
| 2 | `crates/board/src/fen.rs` | New file: `FenError` enum, `parse_fen` and `format_fen` functions, and colocated unit tests |
| 3 | `crates/board/src/lib.rs` | Add `mod fen;` declaration and re-export `FenError` |
| 4 | `crates/board/src/position.rs` | Add `pub(crate) fn new(...)` constructor, `from_fen` and `to_fen` methods to `impl Position` |

## 2. Dependencies

- **`thiserror = "2"`** — must be added to `crates/board/Cargo.toml`. Already present in the workspace via `crates/types/Cargo.toml`, so Cargo will resolve it from the lockfile.
- **T01 (Position Struct)** — completed. Provides `Position`, `CastlingRights`, and all fields/accessors.
- **`chess-types` crate** — already a dependency of `chess-board`. Provides `Bitboard`, `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank`.

## 3. Implementation Details

### 3.1 `crates/board/Cargo.toml`

Add `thiserror = "2"` under `[dependencies]` alongside the existing `chess-types` dependency:

```toml
[dependencies]
chess-types = { path = "../types" }
thiserror = "2"
```

### 3.2 `crates/board/src/position.rs`

Add a `pub(crate) fn new(...)` constructor to `impl Position`. This allows `fen.rs` to construct `Position` instances without exposing struct fields publicly:

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

Add `from_fen` and `to_fen` methods that delegate to `fen.rs`:

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

### 3.3 `crates/board/src/fen.rs`

**Purpose**: Contains `FenError` type definition, `parse_fen`, and `format_fen` functions, plus colocated unit tests.

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

#### `parse_fen` function

Signature: `pub(crate) fn parse_fen(fen: &str) -> Result<Position, FenError>`

Logic:

1. Split `fen` on whitespace via `fen.split_whitespace()`, collect into a `Vec<&str>`. If length ≠ 6, return `FenError::WrongFieldCount(parts.len())`.

2. **Piece placement (field 0)**: Split field by `'/'`. Validate exactly 8 rank strings; if count ≠ 8, return `FenError::InvalidPiecePlacement("expected 8 ranks".into())`. Initialize `let mut piece_bb = [Bitboard::EMPTY; 12]`. Iterate the split rank strings using `enumerate()`. For each `(rank_str_idx, rank_str)`, compute `rank_idx = 7 - rank_str_idx` (FEN lists rank 8 first, rank 1 last) and convert: `let rank = Rank::try_from(rank_idx as u8).map_err(|_| FenError::InvalidPiecePlacement(...))`. Track `file_idx: u8 = 0`. For each character `ch` in `rank_str`:
   - If `ch` is a digit `'1'`–`'8'`: advance `file_idx` by `ch as u8 - b'0'`.
   - If `ch` is a piece character: map to `Piece` using the match table below, convert file index: `let file = File::try_from(file_idx).map_err(|_| FenError::InvalidPiecePlacement(...))`, compute the square: `let sq = Square::from_file_rank(file, rank)`, set the bit: `piece_bb[piece.index()] |= Bitboard::new(1u64 << sq.index())`. Advance `file_idx` by 1.
   - If `ch` is any other character: return `FenError::InvalidPiecePlacement(format!("unexpected character '{ch}'"))`.
   - After processing all characters in the rank, if `file_idx != 8`, return `FenError::InvalidPiecePlacement(format!("rank {} sums to {file_idx} squares, expected 8", rank_idx + 1))`.

   Character-to-piece mapping (all 12 entries):
   - `'P'` → `Piece::new(Color::White, PieceKind::Pawn)`
   - `'N'` → `Piece::new(Color::White, PieceKind::Knight)`
   - `'B'` → `Piece::new(Color::White, PieceKind::Bishop)`
   - `'R'` → `Piece::new(Color::White, PieceKind::Rook)`
   - `'Q'` → `Piece::new(Color::White, PieceKind::Queen)`
   - `'K'` → `Piece::new(Color::White, PieceKind::King)`
   - `'p'` → `Piece::new(Color::Black, PieceKind::Pawn)`
   - `'n'` → `Piece::new(Color::Black, PieceKind::Knight)`
   - `'b'` → `Piece::new(Color::Black, PieceKind::Bishop)`
   - `'r'` → `Piece::new(Color::Black, PieceKind::Rook)`
   - `'q'` → `Piece::new(Color::Black, PieceKind::Queen)`
   - `'k'` → `Piece::new(Color::Black, PieceKind::King)`

   After all ranks, compute occupancy:
   ```rust
   let mut white_occ = Bitboard::EMPTY;
   for bb in &piece_bb[..6] {
       white_occ |= *bb;
   }
   let mut black_occ = Bitboard::EMPTY;
   for bb in &piece_bb[6..] {
       black_occ |= *bb;
   }
   let occupied = white_occ | black_occ;
   let occupied_by = [white_occ, black_occ];
   ```

   This works because `Piece::index()` returns `color as usize * 6 + kind as usize`, meaning indices 0–5 are white pieces and indices 6–11 are black pieces.

3. **Active color (field 1)**: Match `"w"` → `Color::White`, `"b"` → `Color::Black`. Any other string → return `FenError::InvalidActiveColor(field.to_string())`.

4. **Castling availability (field 2)**: If the field is `"-"`, set `let castling_rights = CastlingRights::NONE`. Otherwise, start with `let mut castling_rights = CastlingRights::NONE` and iterate characters. For each character:
   - `'K'` → check `!castling_rights.contains(CastlingRights::WHITE_KINGSIDE)`, then `castling_rights.set(CastlingRights::WHITE_KINGSIDE)`
   - `'Q'` → check `!castling_rights.contains(CastlingRights::WHITE_QUEENSIDE)`, then `castling_rights.set(CastlingRights::WHITE_QUEENSIDE)`
   - `'k'` → check `!castling_rights.contains(CastlingRights::BLACK_KINGSIDE)`, then `castling_rights.set(CastlingRights::BLACK_KINGSIDE)`
   - `'q'` → check `!castling_rights.contains(CastlingRights::BLACK_QUEENSIDE)`, then `castling_rights.set(CastlingRights::BLACK_QUEENSIDE)`
   - Any other character → return `FenError::InvalidCastling(field.to_string())`

   If the flag is already set when encountered (duplicate character), return `FenError::InvalidCastling(field.to_string())`.

   Note: `CastlingRights::WHITE_KINGSIDE` etc. are `u8` constants (0b0001, 0b0010, 0b0100, 0b1000). `contains(flag: u8) -> bool` checks a specific flag. `set(&mut self, flag: u8)` enables a flag.

5. **En passant (field 3)**: If the field is `"-"`, set `let en_passant = None`. Otherwise:
   - Validate length is exactly 2; if not, return `FenError::InvalidEnPassant(field.to_string())`.
   - Extract characters: `let mut chars = field.chars(); let file_char = chars.next().unwrap(); let rank_char = chars.next().unwrap();` (safe because length was checked to be exactly 2).
   - Convert file: compute `file_char as u8 - b'a'` (will overflow for chars before `'a'` but `File::try_from` will reject). Call `let file = File::try_from(file_char as u8 - b'a').map_err(|_| FenError::InvalidEnPassant(field.to_string()))?`.
   - Convert rank: compute `rank_char as u8 - b'1'`. Call `let rank = Rank::try_from(rank_char as u8 - b'1').map_err(|_| FenError::InvalidEnPassant(field.to_string()))?`.
   - Validate the rank is `Rank::R3` or `Rank::R6` (the only valid en passant ranks); otherwise return `FenError::InvalidEnPassant(field.to_string())`.
   - Construct: `let en_passant = Some(Square::from_file_rank(file, rank))`.

   Note: For chars before `'a'` (e.g., `'Z'`), `file_char as u8 - b'a'` produces a wrapped `u8` value ≥ 8, which `File::try_from` correctly rejects. To avoid wrapping subtraction on debug builds, use `file_char as u8.wrapping_sub(b'a')` or check `file_char.is_ascii_lowercase()` first.

6. **Halfmove clock (field 4)**: Parse via `field.parse::<u8>()`. On failure, return `FenError::InvalidHalfmoveClock(field.to_string())`.

7. **Fullmove number (field 5)**: Parse via `field.parse::<u16>()`. On failure, return `FenError::InvalidFullmoveNumber(field.to_string())`. If the parsed value is `0`, return `FenError::InvalidFullmoveNumber(field.to_string())` (fullmove number must be ≥ 1).

8. Construct and return:
   ```rust
   Ok(Position::new(
       piece_bb,
       occupied_by,
       occupied,
       side_to_move,
       castling_rights,
       en_passant,
       halfmove_clock,
       fullmove_counter,
   ))
   ```

#### `format_fen` function

Signature: `pub(crate) fn format_fen(pos: &Position) -> String`

Logic:

1. **Piece placement**: Use a `String` buffer. Iterate rank indices from 7 down to 0 (rank 8 to rank 1). For each `rank_idx`, convert via `Rank::try_from(rank_idx as u8).unwrap()` (safe because `rank_idx` is always 0..=7). Track `empty_count: u8 = 0`. Iterate file indices 0..8. For each `file_idx`, convert via `File::try_from(file_idx as u8).unwrap()` (safe because `file_idx` is always 0..=7). Compute `let sq = Square::from_file_rank(file, rank)`. Call `pos.piece_on(sq)`:
   - If `Some(piece)`: if `empty_count > 0`, append the digit character (`(b'0' + empty_count) as char`) and reset `empty_count = 0`. Then append the piece character using the `Piece` `Display` impl (which produces `KQRBNPkqrbnp` — uppercase for white, lowercase for black) via `write!(result, "{piece}")`.
   - If `None`: increment `empty_count`.
   
   After processing all 8 files in a rank, if `empty_count > 0`, flush by appending the digit character and reset. If `rank_idx > 0`, append `'/'`.

2. **Active color**: Append `" w"` if `pos.side_to_move() == Color::White`, `" b"` if `Color::Black`.

3. **Castling availability**: Append a space. Track whether any flag was written. Check each flag in order:
   - If `pos.castling_rights().contains(CastlingRights::WHITE_KINGSIDE)`, append `'K'`
   - If `pos.castling_rights().contains(CastlingRights::WHITE_QUEENSIDE)`, append `'Q'`
   - If `pos.castling_rights().contains(CastlingRights::BLACK_KINGSIDE)`, append `'k'`
   - If `pos.castling_rights().contains(CastlingRights::BLACK_QUEENSIDE)`, append `'q'`
   
   If no flags were written, append `'-'`.

4. **En passant**: Append a space. If `pos.en_passant()` is `Some(sq)`, format using `Square`'s `Display` impl via `write!(result, "{sq}")` (produces e.g. `"e3"`, `"d6"`). If `None`, append `'-'`.

5. **Halfmove clock**: Append a space, then `pos.halfmove_clock()` formatted as decimal.

6. **Fullmove number**: Append a space, then `pos.fullmove_counter()` formatted as decimal.

7. Return the built `String`.

### 3.4 `crates/board/src/lib.rs`

Update to declare the `fen` module and re-export `FenError`:

```rust
pub mod fen;
pub mod position;

pub use fen::FenError;
pub use position::{CastlingRights, Position};
```

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
- Spot-check: verify `side_to_move()` matches the FEN (White for Alekhine and Nimzo, Black for Italian-like)

**T3: `endgame_round_trips`**
- Parse each of these FEN strings:
  - `"8/8/4k3/8/8/4K3/4P3/8 w - - 0 1"` (king+pawn vs king)
  - `"8/5k2/8/8/8/2K5/8/4R3 w - - 0 1"` (rook endgame)
- For each: assert `from_fen` succeeds and `to_fen()` equals the input string
- Verify castling rights are `CastlingRights::NONE` and en passant is `None`

**T4: `startpos_from_fen_matches_startpos`**
- Parse starting FEN, compare all fields with `Position::startpos()`: all 12 piece bitboards (via `piece_bitboard()` for each `Piece`), occupancy bitboards (via `occupied_by()` for each `Color` and `occupied()`), side to move, castling rights, en passant, halfmove clock, fullmove counter

**T5: `reject_wrong_field_count`**
- FEN with 5 fields (e.g., `"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0"`): assert `Err` and match against `FenError::WrongFieldCount(5)`
- FEN with 7 fields: assert `Err` and match against `FenError::WrongFieldCount(7)`

**T6: `reject_invalid_piece_placement`**
- Rank with 9 squares (e.g., `"rnbqkbnrr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"`) → `FenError::InvalidPiecePlacement`
- Invalid piece character (e.g., `"xnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"`) → `FenError::InvalidPiecePlacement`
- Only 7 ranks (e.g., `"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP w KQkq - 0 1"`) → `FenError::InvalidPiecePlacement`

**T7: `reject_invalid_active_color`**
- Active color field `"x"` (e.g., `"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR x KQkq - 0 1"`) → `FenError::InvalidActiveColor`

**T8: `reject_invalid_castling`**
- Castling field `"X"` (e.g., `"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w X - 0 1"`) → `FenError::InvalidCastling`

**T9: `reject_invalid_en_passant`**
- En passant field `"e9"` → `FenError::InvalidEnPassant`
- En passant field `"z3"` → `FenError::InvalidEnPassant`

**T10: `reject_invalid_clocks`**
- Halfmove clock `"abc"` (e.g., `"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - abc 1"`) → `FenError::InvalidHalfmoveClock`
- Fullmove number `"0"` → `FenError::InvalidFullmoveNumber`
- Fullmove number `"-1"` → `FenError::InvalidFullmoveNumber`

## 6. Implementation Order

1. **Add `thiserror` dependency** to `crates/board/Cargo.toml`
2. **Add `pub(crate) fn new(...)` constructor** to `Position` in `crates/board/src/position.rs` — enables `fen.rs` to construct positions without exposing fields publicly
3. **Create `crates/board/src/fen.rs`** with:
   - `FenError` enum (with `thiserror` derive)
   - `pub(crate) fn parse_fen(fen: &str) -> Result<Position, FenError>` — the full FEN parser
   - `pub(crate) fn format_fen(pos: &Position) -> String` — the FEN serializer
   - `#[cfg(test)] mod tests` — all unit tests (T1–T10)
4. **Add `from_fen` and `to_fen` methods** to `impl Position` in `crates/board/src/position.rs` (delegating to `fen::parse_fen` and `fen::format_fen`)
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
- **Issue**: The `Position` struct fields are private (no `pub` modifier), and `fen.rs` is a separate module that needs to construct `Position` instances. Making all fields `pub` would break encapsulation. Adding `pub(crate)` to each field individually would be invasive and leak internal representation to sibling modules beyond what is necessary.
- **Alternative chosen**: Add a single `pub(crate) fn new(...)` constructor to `Position` that accepts all fields. This allows `fen.rs` to construct positions while keeping fields private to external consumers. This is the minimal change and follows Rust encapsulation conventions. The same constructor will also be usable by T03 (Zobrist hashing) when it needs to construct positions.