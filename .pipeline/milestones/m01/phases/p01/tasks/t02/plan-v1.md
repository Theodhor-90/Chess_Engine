Now I have a complete picture of the existing codebase and what needs to be implemented. Let me produce the implementation plan.

## 1. Deliverables

| File | Purpose |
|------|---------|
| `crates/types/src/color.rs` | `Color` enum (`White`, `Black`) with `opposite()` method, explicit discriminants for array indexing |
| `crates/types/src/piece.rs` | `PieceKind` enum (6 variants) and `Piece` struct combining `Color` + `PieceKind`, with flat index (0..11) support |
| `crates/types/src/square.rs` | `Square` newtype, `File` enum (A–H), `Rank` enum (R1–R8), with conversion traits and LERF mapping (a1=0, h8=63) |
| `crates/types/src/lib.rs` | Modified to declare new modules and re-export all public types |

## 2. Dependencies

- No new crate dependencies required. The `types` crate already has `thiserror = "2"` which can be used for error types if needed.
- **T01 (Bitboard)** is complete — `Square` indices correspond to `Bitboard` bit positions.
- Rust stable toolchain (2021 edition) — already configured.

## 3. Implementation Details

### `crates/types/src/color.rs`

**Purpose:** Defines the `Color` enum for the two sides.

**Exports:** `Color`

**Implementation:**

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum Color {
    White = 0,
    Black = 1,
}
```

- `Color::opposite(&self) -> Color` — returns the other color.
- `Color::COUNT` — associated constant set to `2`.
- `TryFrom<u8> for Color` — returns `Option<Color>` semantics via `Result` (0 → White, 1 → Black, else error). Use `thiserror` for the error type or a simple unit error.
- `Display` impl — prints `"White"` / `"Black"`.

Colocated `#[cfg(test)] mod tests` covering:
- Construction of both variants
- `opposite()` round-trips (`c.opposite().opposite() == c`)
- `TryFrom<u8>` for valid (0, 1) and invalid (2, 255) values
- Explicit discriminant values (`White as u8 == 0`, `Black as u8 == 1`)

### `crates/types/src/piece.rs`

**Purpose:** Defines `PieceKind` (the 6 piece types) and `Piece` (a color–kind pair).

**Exports:** `PieceKind`, `Piece`

**Implementation:**

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum PieceKind {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}
```

- `PieceKind::COUNT` — associated constant set to `6`.
- `TryFrom<u8> for PieceKind` — 0..5 valid, else error.

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Piece {
    pub color: Color,
    pub kind: PieceKind,
}
```

- `Piece::new(color: Color, kind: PieceKind) -> Piece`
- `Piece::index(&self) -> usize` — flat index 0..11, computed as `color as usize * 6 + kind as usize`. This is the canonical mapping for piece-bitboard arrays and Zobrist keys in later phases.
- `Piece::from_index(index: usize) -> Option<Piece>` — inverse of `index()`, returns `None` for index >= 12.
- `Display` impl — prints the piece character (e.g., `'P'`, `'n'`): uppercase for White, lowercase for Black.

Colocated `#[cfg(test)] mod tests` covering:
- Construction of all 12 piece combinations
- `index()` produces values 0..11 with no collisions
- `from_index()` round-trip: `Piece::from_index(p.index()) == Some(p)` for all 12 pieces
- `from_index(12)` and `from_index(255)` return `None`
- `TryFrom<u8>` for `PieceKind`: valid (0..5) and invalid (6)

### `crates/types/src/square.rs`

**Purpose:** Defines `File`, `Rank`, and `Square` with LERF mapping and conversion traits.

**Exports:** `File`, `Rank`, `Square`

**Implementation:**

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum File {
    A = 0, B = 1, C = 2, D = 3, E = 4, F = 5, G = 6, H = 7,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum Rank {
    R1 = 0, R2 = 1, R3 = 2, R4 = 3, R5 = 4, R6 = 5, R7 = 6, R8 = 7,
}
```

- `File::COUNT` = 8, `Rank::COUNT` = 8.
- `TryFrom<u8> for File` — 0..7 valid.
- `TryFrom<u8> for Rank` — 0..7 valid.
- `Display` for `File` — prints `'a'`..`'h'`.
- `Display` for `Rank` — prints `'1'`..`'8'`.

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Square(u8);
```

- `Square::new(index: u8) -> Option<Square>` — returns `None` if index > 63.
- `Square::index(self) -> u8` — returns the raw index.
- `Square::file(self) -> File` — `index % 8` mapped to `File`.
- `Square::rank(self) -> Rank` — `index / 8` mapped to `Rank`.
- `Square::from_file_rank(file: File, rank: Rank) -> Square` — computes `rank as u8 * 8 + file as u8`. Always valid, so returns `Square` directly (not `Option`).
- `Square::COUNT` = 64.
- Named constants for notable squares: `Square::A1`, `Square::H1`, `Square::A8`, `Square::H8` (and all 64 if convenient, but at minimum these four for test readability).
- `From<Square> for (File, Rank)` — decomposes into file and rank.
- `From<(File, Rank)> for Square` — constructs from file and rank.
- `Display` for `Square` — prints algebraic notation (e.g., `"e4"`).

Colocated `#[cfg(test)] mod tests` covering:
- Conversion round-trip: `Square → (File, Rank) → Square` for all 64 squares
- Boundary values: a1 index == 0, h1 index == 7, a8 index == 56, h8 index == 63
- `Square::new(64)` returns `None`, `Square::new(0)` returns `Some`
- `Square::new(255)` returns `None`
- `File` and `Rank` extraction for known squares
- `TryFrom<u8>` for `File`: valid (0..7) and invalid (8)
- `TryFrom<u8>` for `Rank`: valid (0..7) and invalid (8)
- `Color::opposite()` (already covered in color.rs, not duplicated here)
- Display output for a few squares (e.g., `Square::A1` displays as `"a1"`, `Square::H8` displays as `"h8"`)

### `crates/types/src/lib.rs`

**Purpose:** Crate root — declares all modules and re-exports public types.

**Current content:**
```rust
pub mod bitboard;
pub use bitboard::Bitboard;
```

**Modified content:**
```rust
pub mod bitboard;
pub mod color;
pub mod piece;
pub mod square;

pub use bitboard::Bitboard;
pub use color::Color;
pub use piece::{Piece, PieceKind};
pub use square::{File, Rank, Square};
```

## 4. API Contracts

### `Color`

| Method | Signature | Example |
|--------|-----------|---------|
| `opposite` | `fn opposite(self) -> Color` | `Color::White.opposite() == Color::Black` |
| `TryFrom<u8>` | `fn try_from(v: u8) -> Result<Color, _>` | `Color::try_from(0) == Ok(Color::White)` |

### `PieceKind`

| Method | Signature | Example |
|--------|-----------|---------|
| `TryFrom<u8>` | `fn try_from(v: u8) -> Result<PieceKind, _>` | `PieceKind::try_from(0) == Ok(PieceKind::Pawn)` |

### `Piece`

| Method | Signature | Example |
|--------|-----------|---------|
| `new` | `fn new(color: Color, kind: PieceKind) -> Piece` | `Piece::new(Color::White, PieceKind::King)` |
| `index` | `fn index(self) -> usize` | `Piece::new(Color::White, PieceKind::Pawn).index() == 0` |
| `from_index` | `fn from_index(idx: usize) -> Option<Piece>` | `Piece::from_index(0) == Some(white_pawn)` |

### `File` / `Rank`

| Method | Signature | Example |
|--------|-----------|---------|
| `TryFrom<u8>` | `fn try_from(v: u8) -> Result<File, _>` | `File::try_from(0) == Ok(File::A)` |
| `TryFrom<u8>` | `fn try_from(v: u8) -> Result<Rank, _>` | `Rank::try_from(7) == Ok(Rank::R8)` |

### `Square`

| Method | Signature | Example |
|--------|-----------|---------|
| `new` | `fn new(idx: u8) -> Option<Square>` | `Square::new(0) == Some(Square::A1)` |
| `index` | `fn index(self) -> u8` | `Square::A1.index() == 0` |
| `file` | `fn file(self) -> File` | `Square::A1.file() == File::A` |
| `rank` | `fn rank(self) -> Rank` | `Square::A1.rank() == Rank::R1` |
| `from_file_rank` | `fn from_file_rank(f: File, r: Rank) -> Square` | `Square::from_file_rank(File::E, Rank::R4)` |
| `From<(File,Rank)>` | trait impl | `Square::from((File::A, Rank::R1)) == Square::A1` |
| `From<Square>` for `(File,Rank)` | trait impl | `(File, Rank)::from(Square::A1) == (File::A, Rank::R1)` |

## 5. Test Plan

All tests are colocated unit tests in `#[cfg(test)] mod tests` within each source file.

### `color.rs` tests

| Test | Verifies |
|------|----------|
| `test_opposite` | `White.opposite() == Black` and `Black.opposite() == White` |
| `test_opposite_round_trip` | `c.opposite().opposite() == c` for both colors |
| `test_discriminant_values` | `White as u8 == 0`, `Black as u8 == 1` |
| `test_try_from_valid` | `TryFrom<u8>` succeeds for 0 and 1 |
| `test_try_from_invalid` | `TryFrom<u8>` fails for 2 and 255 |

### `piece.rs` tests

| Test | Verifies |
|------|----------|
| `test_piece_kind_discriminants` | `Pawn as u8 == 0` through `King as u8 == 5` |
| `test_piece_kind_try_from_valid` | `TryFrom<u8>` succeeds for 0..5 |
| `test_piece_kind_try_from_invalid` | `TryFrom<u8>` fails for 6 |
| `test_piece_index_all` | All 12 pieces produce unique indices in 0..11 |
| `test_piece_from_index_round_trip` | `Piece::from_index(p.index()) == Some(p)` for all 12 |
| `test_piece_from_index_invalid` | `from_index(12)` returns `None` |

### `square.rs` tests

| Test | Verifies |
|------|----------|
| `test_square_boundary_values` | a1=0, h1=7, a8=56, h8=63 |
| `test_square_round_trip_all` | `Square → (File, Rank) → Square` for all 64 squares |
| `test_square_new_valid` | `Square::new(0)` through `Square::new(63)` all return `Some` |
| `test_square_new_invalid` | `Square::new(64)` and `Square::new(255)` return `None` |
| `test_file_rank_extraction` | `Square::A1.file() == File::A`, `Square::A1.rank() == Rank::R1`, etc. |
| `test_file_try_from_valid` | `TryFrom<u8>` for File succeeds 0..7 |
| `test_file_try_from_invalid` | `TryFrom<u8>` for File fails at 8 |
| `test_rank_try_from_valid` | `TryFrom<u8>` for Rank succeeds 0..7 |
| `test_rank_try_from_invalid` | `TryFrom<u8>` for Rank fails at 8 |
| `test_display` | `Square::A1` displays as `"a1"`, `Square::H8` displays as `"h8"` |

## 6. Implementation Order

1. **`crates/types/src/color.rs`** — No dependencies on other new types. Implement `Color` enum with `opposite()`, discriminants, `TryFrom<u8>`, `Display`, and tests.

2. **`crates/types/src/piece.rs`** — Depends on `Color`. Implement `PieceKind` enum with discriminants and `TryFrom<u8>`. Implement `Piece` struct with `new()`, `index()`, `from_index()`, `Display`, and tests.

3. **`crates/types/src/square.rs`** — Independent of `Color`/`Piece`. Implement `File`, `Rank`, `Square` with all conversions, `Display`, named constants, and tests.

4. **`crates/types/src/lib.rs`** — Add module declarations and re-exports for all new types.

5. **Verify** — Run `cargo fmt`, `cargo clippy`, `cargo test`.

## 7. Verification Commands

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --workspace
```

## 8. Design Deviations

**`Square` as a newtype struct vs enum with 64 variants.**

- The task spec says: "Square enum or newtype with 64 variants using little-endian rank-file mapping."
- A `#[repr(u8)]` enum with 64 explicitly named variants (A1=0, B1=1, ..., H8=63) is verbose and makes generic indexing awkward (match arms or unsafe transmute for `from_index`).
- A newtype `struct Square(u8)` with a constructor that validates range 0..63 is idiomatic, compact, and matches the `Bitboard` newtype convention established in T01. Named constants (`Square::A1`, `Square::H1`, etc.) provide the same ergonomics as enum variants.
- The task spec explicitly allows either approach ("Square enum or newtype"). The newtype is chosen for consistency with `Bitboard(u64)` and to avoid a 64-arm match in `TryFrom`.