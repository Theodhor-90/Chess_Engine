# Task T02 — Chess Enums

## Objective

Implement the chess enumeration types `Color`, `PieceKind`, `Piece`, `Square`, `File`, and `Rank` in the `crates/types/` crate, with conversion traits and safe indexing.

## Deliverables

- `Color` enum with variants `White` and `Black`, plus a method to return the opposite color
- `PieceKind` enum with variants `Pawn`, `Knight`, `Bishop`, `Rook`, `Queen`, `King`
- `Piece` type combining `Color` and `PieceKind` (either as a struct or an enum with 12 variants)
- `File` enum with variants A through H
- `Rank` enum with variants 1 through 8 (named `R1`..`R8` or similar since numeric identifiers aren't valid)
- `Square` enum or newtype with 64 variants using little-endian rank-file mapping: a1 = 0, b1 = 1, ..., h1 = 7, a2 = 8, ..., h8 = 63
- Conversion traits: `From`/`Into` between `Square` and `(File, Rank)`, `Square` from index, `File` and `Rank` from `Square`
- Safe indexing — all conversions from raw integers return `Option` or `Result` to prevent out-of-bounds access
- Explicit integer discriminants on enums so they can serve as array indices without conversion overhead

## Files to Create or Modify

- Create `crates/types/src/color.rs` (or a combined module structure)
- Create `crates/types/src/piece.rs`
- Create `crates/types/src/square.rs`
- Modify `crates/types/src/lib.rs` to export all new types

## Key Implementation Details

- **Little-endian rank-file mapping**: a1 = 0, h8 = 63. This is the standard convention for bitboard engines and must be consistent with `Bitboard` bit positions
- Enums should use explicit discriminants for zero-cost array indexing (e.g., `Color::White = 0, Color::Black = 1`)
- `Piece` needs to support conversion to a flat index (0..11) for use as array indices in later phases (e.g., piece bitboards, Zobrist keys)
- No `unwrap()` in library code
- No external dependencies beyond `thiserror`
- `///` doc comments on all public items

## Dependencies

- **T01 (Bitboard)** must be complete — `Square` indices correspond to bit positions in `Bitboard`, so the mapping convention must be established first

## Verification Criteria

1. Unit tests cover construction of all enum variants
2. Unit tests verify conversion round-trips: `Square` → `(File, Rank)` → `Square` for all 64 squares
3. Unit tests verify `Square` from index: valid indices (0..63) succeed, invalid indices fail gracefully
4. Unit tests verify `Color::opposite()` returns the other color
5. Unit tests verify boundary values: a1 = 0, h1 = 7, a8 = 56, h8 = 63
6. Unit tests verify invalid input handling (e.g., index 64 returns `None`/error)
7. `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` all pass