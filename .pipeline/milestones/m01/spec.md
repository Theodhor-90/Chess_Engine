## Milestone M01 — Core Types & Board Representation

### Goal

Establish the foundational data types and bitboard-based position representation. This is the substrate everything else builds on — correctness here is non-negotiable.

### Phases

#### Phase P01 — Primitive Types
- **T01**: `Bitboard` newtype with arithmetic/bitwise operator overloads, bit iteration, population count, LSB/MSB extraction
- **T02**: `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank` enums with conversion traits and safe indexing
- **T03**: `Move` as a packed `u16` with encode/decode for from, to, flags (promotion, castling, en passant, double push)

#### Phase P02 — Position State
- **T01**: `Position` struct — 12 piece bitboards (one per piece-color), occupancy bitboards, side to move, castling rights, en passant square, halfmove clock, fullmove counter
- **T02**: FEN parser and serializer (`Position::from_fen`, `Position::to_fen`) with full validation
- **T03**: Zobrist hashing — random keys for piece-square, side-to-move, castling, en passant; incremental hash update

#### Phase P03 — Position Manipulation
- **T01**: `make_move` — apply a move to the position, updating all bitboards, castling rights, en passant, clocks, and Zobrist hash. Return captured piece info for unmake.
- **T02**: `unmake_move` — restore previous position state using captured piece info. Must be exact inverse of `make_move`.
- **T03**: `is_square_attacked` — determine if a given square is attacked by a given side. Foundation for legality checks and king safety.

### Exit Criteria
- All core types (`Bitboard`, `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank`, `Move`) are implemented with full test coverage
- `Position` correctly represents any legal chess position via FEN round-tripping
- Zobrist hashing produces consistent, incrementally-updated hashes
- `make_move`/`unmake_move` are exact inverses for all move types (normal, capture, promotion, castling, en passant, double push)
- `is_square_attacked` correctly identifies attacks for all piece types
- All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`