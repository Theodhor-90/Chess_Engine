Now I have full context. Let me produce the phase specification.

# Phase P02 — Position State

## Goal

Build the `board` crate containing the `Position` struct that holds the complete state of a chess game, a FEN parser and serializer for interoperability with standard tooling, and Zobrist hashing for efficient position identification — providing the position representation and serialization/hashing infrastructure that Phase P03 (Position Manipulation) and all subsequent milestones depend on.

## Design Decisions

1. **Separate `board` crate**: The `Position` struct and its associated logic live in `crates/board/`, which depends on `crates/types/`. This follows the workspace layout defined in the master plan and keeps position representation decoupled from move generation and search.

2. **12 piece bitboards plus occupancy bitboards**: The position stores one `Bitboard` per piece-color combination (e.g., white pawns, black knights — 12 total), plus three occupancy bitboards (white pieces, black pieces, all pieces). The occupancy boards are derived from the piece bitboards and must be kept consistent. This representation allows fast piece lookup by type and color.

3. **Castling rights as a bitfield**: Castling availability (white kingside, white queenside, black kingside, black queenside) is stored as a compact representation with four boolean flags. This supports efficient Zobrist key updates and clear FEN serialization.

4. **En passant stored as `Option<Square>`**: The en passant target square is optional — it is `None` when no en passant capture is possible. This directly maps to the FEN `-` notation for the en passant field.

5. **Zobrist keys as compile-time or lazily-initialized constants**: Random `u64` keys are pre-generated for all piece-square combinations (12 × 64 = 768 keys), one side-to-move key, 16 castling keys (one per combination of the 4 castling rights), and 8 en passant file keys. Keys are fixed values to ensure deterministic hashing across runs. Incremental update is performed via XOR.

6. **Error handling with `thiserror`**: FEN parsing errors use a dedicated error type with `thiserror` derive, providing descriptive messages for each of the six FEN fields that can fail validation. No `unwrap()` in library code.

## Tasks

### T01 — Position Struct

Define the `Position` struct and its construction logic within `crates/board/`.

**Deliverables:**
- `crates/board/` crate with `Cargo.toml` depending on `crates/types/`
- `Position` struct with fields: 12 piece bitboards (indexed by `Piece`), occupancy bitboards (per `Color` and total), side to move (`Color`), castling rights, en passant square (`Option<Square>`), halfmove clock (`u8`), fullmove counter (`u16`)
- `Position::startpos()` that returns the standard chess starting position
- Accessor methods for querying piece placement (e.g., get the bitboard for a given piece, get the piece on a given square)
- Unit tests verifying the starting position has correct piece placement, side to move, castling rights, and clocks

### T02 — FEN Parser and Serializer

Implement `Position::from_fen` and `Position::to_fen` with full validation.

**Deliverables:**
- `Position::from_fen(fen: &str) -> Result<Position, FenError>` parsing and validating all six FEN fields: piece placement, active color, castling availability, en passant target square, halfmove clock, fullmove number
- `Position::to_fen(&self) -> String` producing a canonical FEN string
- `FenError` type with variants covering each kind of validation failure
- Unit tests verifying: standard starting position FEN, various middlegame/endgame positions, round-trip property (`Position::from_fen(fen).to_fen() == fen` for canonical FENs), rejection of malformed FEN strings

### T03 — Zobrist Hashing

Implement Zobrist key generation, full hash computation, and incremental hash update support.

**Deliverables:**
- Pre-generated random `u64` Zobrist keys for: piece-square combinations (12 × 64), side-to-move (1), castling rights (16 combinations), en passant file (8)
- `Position` gains a `hash` field (`u64`) representing the current Zobrist hash
- Function to compute the full Zobrist hash from scratch given a position's state
- Methods to incrementally update the hash by XOR-ing in/out changed keys (piece placement changes, side-to-move toggle, castling rights changes, en passant file changes)
- `Position::from_fen` and `Position::startpos` compute and store the initial hash
- Unit tests verifying: identical positions produce identical hashes, incremental updates match full recomputation, different positions produce different hashes (spot-checked, not guaranteed)

## Exit Criteria

1. `Position` struct correctly represents any legal chess position with all required fields (12 piece bitboards, occupancy bitboards, side to move, castling rights, en passant square, halfmove clock, fullmove counter)
2. FEN round-tripping succeeds: `Position::from_fen(fen).to_fen() == fen` for canonical FEN strings across starting position, middlegame, and endgame positions
3. FEN parser rejects malformed input with descriptive errors for each invalid field
4. Zobrist hashing produces consistent hashes — identical positions yield identical hashes, and incremental updates match full recomputation
5. All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`

## Dependencies

- **Phase P01 (Primitive Types)** must be complete — this phase depends on `Bitboard`, `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank`, and `Move` types from the `types` crate

## Artifacts

- `crates/board/Cargo.toml` — crate manifest with dependency on `crates/types/`
- `crates/board/src/lib.rs` — crate root and public API exports
- `crates/board/src/position.rs` — `Position` struct, accessors, and `startpos()`
- `crates/board/src/fen.rs` — `from_fen`, `to_fen`, and `FenError`
- `crates/board/src/zobrist.rs` — Zobrist key tables, full hash computation, incremental update methods
- Updated workspace `Cargo.toml` — add `crates/board` to workspace members