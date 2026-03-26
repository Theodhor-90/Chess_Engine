Now I have all the context I need. Let me verify the master plan's exact specification for M01 and produce the milestone specification.

# Milestone M01 — Core Types & Board Representation

## Goal

Establish the foundational data types and bitboard-based position representation that every subsequent milestone builds upon, including primitive chess types with bitboard operations, a complete position struct with FEN parsing and Zobrist hashing, and position manipulation via make/unmake move with attack detection.

## Scope

- `Bitboard` newtype with arithmetic/bitwise operator overloads, bit iteration, population count, LSB/MSB extraction
- `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank` enums with conversion traits and safe indexing
- `Move` as a packed `u16` with encode/decode for from, to, flags (promotion, castling, en passant, double push)
- `Position` struct with 12 piece bitboards (one per piece-color), occupancy bitboards, side to move, castling rights, en passant square, halfmove clock, fullmove counter
- FEN parser and serializer (`Position::from_fen`, `Position::to_fen`) with full validation
- Zobrist hashing with random keys for piece-square, side-to-move, castling, en passant; incremental hash update
- `make_move` applying a move to the position, updating all bitboards, castling rights, en passant, clocks, and Zobrist hash; returning captured piece info for unmake
- `unmake_move` restoring previous position state using captured piece info, exact inverse of `make_move`
- `is_square_attacked` determining if a given square is attacked by a given side
- Cargo workspace setup with `crates/types/` and `crates/board/` as defined in the project structure

## Phases

### Phase P01 — Primitive Types

Define the core value types that the entire engine operates on. This phase produces the `types` crate.

- **T01**: `Bitboard` newtype (`struct Bitboard(u64)`) with `BitAnd`, `BitOr`, `BitXor`, `Not`, `Shl`, `Shr` operator overloads, bit iteration (iterate over set bit indices), population count, and LSB/MSB extraction.
- **T02**: `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank` enums with conversion traits (e.g., `From`/`Into` between `Square` and `(File, Rank)`) and safe indexing. Square uses little-endian rank-file mapping (a1 = 0, h8 = 63).
- **T03**: `Move` as a packed `u16` with encode/decode methods. Bit layout: `from(6) | to(6) | flags(4)`. Flags encode promotion piece, castling, en passant, and double pawn push.

### Phase P02 — Position State

Build the position representation and its serialization/hashing infrastructure. This phase produces the `board` crate (depending on `types`).

- **T01**: `Position` struct containing 12 piece bitboards (one per piece-color combination), occupancy bitboards (per color and total), side to move, castling rights, en passant square, halfmove clock, and fullmove counter.
- **T02**: FEN parser (`Position::from_fen`) and serializer (`Position::to_fen`) with full validation of all six FEN fields.
- **T03**: Zobrist hashing — generate random keys for piece-square combinations, side-to-move, castling rights, and en passant file. Compute full hash from position state. Support incremental hash update (XOR in/out changed keys).

### Phase P03 — Position Manipulation

Implement the operations that modify position state. These are the primitives that move generation and search will call.

- **T01**: `make_move` — apply a move to the position, updating all piece and occupancy bitboards, castling rights, en passant square, halfmove clock, fullmove counter, and Zobrist hash. Handle all move types: quiet, capture, promotion, castling, en passant, double pawn push. Return captured piece info needed by `unmake_move`.
- **T02**: `unmake_move` — restore the position to its exact prior state using the captured piece info returned by `make_move`. Must be the exact inverse of `make_move` for all move types.
- **T03**: `is_square_attacked` — given a square and an attacking side, determine whether that square is attacked. Must account for all piece types (pawn, knight, bishop, rook, queen, king). This is the foundation for legality checks and king safety evaluation in later milestones.

## Exit Criteria

1. All core types (`Bitboard`, `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank`, `Move`) are implemented with unit tests covering construction, conversion, and operator behavior
2. `Position` correctly represents any legal chess position, verified by FEN round-tripping (`Position::from_fen(fen).to_fen() == fen` for canonical FEN strings)
3. Zobrist hashing produces consistent hashes — identical positions yield identical hashes, and incremental updates match full recomputation
4. `make_move` followed by `unmake_move` restores the position to its exact prior state for all move types (quiet, capture, promotion, castling, en passant, double pawn push)
5. `is_square_attacked` correctly identifies attacks for all piece types in varied board positions
6. Cargo workspace is configured with `crates/types/` and `crates/board/` crates, and the project builds and passes `cargo test` from the workspace root

## Dependencies

- Rust stable toolchain (2021 edition) installed
- Cargo workspace initialized at the project root
- No prior milestones required (M01 is the first milestone)