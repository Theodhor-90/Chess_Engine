# Phase P01 — Primitive Types

## Goal

Define and deliver the `types` crate containing the foundational value types that every subsequent phase and milestone depends on: the `Bitboard` newtype with bitwise operators and bit-manipulation methods, the chess enumeration types (`Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank`) with conversion traits and safe indexing, and the packed 16-bit `Move` type with encode/decode support for all move flag variants.

## Design Decisions

1. **Bitboard as a newtype, not a type alias.** `struct Bitboard(u64)` provides type safety — prevents accidental mixing of raw `u64` values with board representations — while still allowing efficient codegen through operator overloads.

2. **Enums with explicit discriminants.** `Color`, `PieceKind`, `File`, `Rank`, and `Square` use explicit integer discriminants so they can serve as array indices without conversion overhead. `Piece` combines `Color` and `PieceKind` to identify any piece-color combination.

3. **Little-endian rank-file square mapping.** a1 = 0, b1 = 1, ..., h1 = 7, a2 = 8, ..., h8 = 63. This is the standard convention in bitboard engines and avoids confusion when interfacing with FEN, UCI, and external tools.

4. **16-bit move encoding.** `from(6) | to(6) | flags(4)` packs all move information into a `u16`. The 4-bit flags field encodes promotion piece type, castling, en passant, and double pawn push. This compact representation is cache-friendly for move lists in search.

5. **No external dependencies for core types.** The `types` crate depends only on the Rust standard library, keeping the foundation dependency-free. `thiserror` is used for structured error types.

## Tasks

### T01 — Bitboard

Implement the `Bitboard` newtype wrapper `struct Bitboard(u64)` in `crates/types/`.

**Deliverables:**
- `Bitboard` struct with `BitAnd`, `BitOr`, `BitXor`, `Not`, `Shl`, `Shr` operator overloads
- Population count method (number of set bits)
- LSB extraction (index of least significant set bit)
- MSB extraction (index of most significant set bit)
- Bit iteration yielding the index of each set bit in order
- Unit tests covering operator behavior, population count, LSB/MSB extraction, and iteration over edge cases (empty bitboard, single bit, all bits set)

### T02 — Chess Enums

Implement `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank` enums in `crates/types/`.

**Deliverables:**
- `Color` enum (`White`, `Black`) with a method to return the opposite color
- `PieceKind` enum (`Pawn`, `Knight`, `Bishop`, `Rook`, `Queen`, `King`)
- `Piece` type combining `Color` and `PieceKind`
- `File` enum (A through H) and `Rank` enum (1 through 8)
- `Square` enum (64 variants or a newtype) with little-endian rank-file mapping (a1 = 0, h8 = 63)
- Conversion traits: `From`/`Into` between `Square` and `(File, Rank)`, `Square` from index, `File` and `Rank` from `Square`
- Safe indexing — all conversions from raw integers return `Option` or `Result` to prevent out-of-bounds access
- Unit tests covering construction, conversion round-trips, boundary values, and invalid input handling

### T03 — Move Encoding

Implement the `Move` type as a packed `u16` in `crates/types/`.

**Deliverables:**
- `Move` struct wrapping a `u16`
- Bit layout: `from(6) | to(6) | flags(4)`
- Encode methods: construct a `Move` from source square, destination square, and flags
- Decode methods: extract source square, destination square, and flags from a `Move`
- Flag definitions for: quiet move, double pawn push, king-side castling, queen-side castling, capture, en passant capture, and four promotion variants (knight, bishop, rook, queen promotion, each with capture and non-capture)
- Unit tests covering encode/decode round-trips for every flag type, boundary squares (a1, h8), and field extraction correctness

## Exit Criteria

1. All core types (`Bitboard`, `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank`, `Move`) are implemented with unit tests covering construction, conversion, and operator behavior
2. All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`

## Dependencies

- No prior phases required (this is the first phase of the first milestone)
- Rust stable toolchain (2021 edition) installed

## Artifacts

- `crates/types/` crate containing all source files for `Bitboard`, `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank`, and `Move`
- `Cargo.toml` workspace root configured with the `types` crate as a workspace member
- `crates/types/Cargo.toml` with crate metadata and `thiserror` dependency