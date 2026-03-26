## Phase P02 — Position State

### Goal

Build the position representation and its serialization/hashing infrastructure. This phase produces the `board` crate (depending on `types`) containing the `Position` struct that holds the complete state of a chess game, a FEN parser/serializer for interoperability, and Zobrist hashing for efficient position identification in search.

### Deliverables

- **T01 — Position Struct**: `Position` struct containing 12 piece bitboards (one per piece-color combination), occupancy bitboards (per color and total), side to move, castling rights, en passant square, halfmove clock, and fullmove counter.
- **T02 — FEN Parser/Serializer**: FEN parser (`Position::from_fen`) and serializer (`Position::to_fen`) with full validation of all six FEN fields. Must satisfy the round-trip property: `Position::from_fen(fen).to_fen() == fen` for canonical FEN strings.
- **T03 — Zobrist Hashing**: Generate random keys for piece-square combinations, side-to-move, castling rights, and en passant file. Compute full hash from position state. Support incremental hash update (XOR in/out changed keys). Identical positions must yield identical hashes, and incremental updates must match full recomputation.

### Technical Constraints

- This phase produces the `crates/board/` crate, which depends on `crates/types/`
- Position uses 12 piece bitboards (one per piece-color combination) plus occupancy bitboards
- Castling rights must track king-side and queen-side for both colors
- FEN validation must cover all six fields: piece placement, active color, castling availability, en passant target, halfmove clock, fullmove number
- Zobrist keys should be pre-generated random `u64` values; incremental update via XOR
- Same coding conventions as P01: no `unsafe` without justification, no `unwrap()` in library code, `thiserror` for errors, doc comments on all public items
- Must pass `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`

### Dependencies

- **Phase P01 (Primitive Types)** must be complete — this phase depends on `Bitboard`, `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank`, and `Move` types from the `types` crate