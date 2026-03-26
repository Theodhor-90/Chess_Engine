## Phase P03 — Position Manipulation

### Goal

Implement the operations that modify position state — make/unmake move and attack detection. These are the primitives that move generation (M02) and search (M03+) will call directly. Correctness is critical: `make_move` followed by `unmake_move` must restore the position exactly, and `is_square_attacked` must correctly identify attacks for all piece types.

### Deliverables

- **T01 — make_move**: Apply a move to the position, updating all piece and occupancy bitboards, castling rights, en passant square, halfmove clock, fullmove counter, and Zobrist hash. Handle all move types: quiet, capture, promotion, castling, en passant, double pawn push. Return captured piece info needed by `unmake_move`.
- **T02 — unmake_move**: Restore the position to its exact prior state using the captured piece info returned by `make_move`. Must be the exact inverse of `make_move` for all move types (quiet, capture, promotion, castling, en passant, double pawn push).
- **T03 — is_square_attacked**: Given a square and an attacking side, determine whether that square is attacked. Must account for all piece types (pawn, knight, bishop, rook, queen, king). This is the foundation for legality checks in move generation (M02) and king safety evaluation in later milestones.

### Technical Constraints

- Lives in the `crates/board/` crate alongside `Position`
- `make_move` must update Zobrist hash incrementally (not recompute from scratch)
- `make_move`/`unmake_move` must handle all special move types: castling (moving both king and rook), en passant (removing captured pawn from different square), promotions (replacing pawn with promoted piece), double pawn push (setting en passant square)
- `is_square_attacked` must work without full move generation — it is used as a building block by the move generator, not the other way around. For sliding pieces (bishop, rook, queen), this phase may use ray-based attack detection (magic bitboards come in M02)
- Same coding conventions: no `unsafe` without justification, no `unwrap()` in library code, doc comments on all public items
- Must pass `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`

### Exit Criteria (from milestone spec)

- `make_move` followed by `unmake_move` restores the position to its exact prior state for all move types
- `is_square_attacked` correctly identifies attacks for all piece types in varied board positions
- Zobrist hash remains consistent through make/unmake cycles

### Dependencies

- **Phase P01 (Primitive Types)** must be complete — depends on `Bitboard`, `Square`, `Move`, `Piece`, `Color` types
- **Phase P02 (Position State)** must be complete — depends on `Position` struct, FEN parsing (for test setup), and Zobrist hashing infrastructure