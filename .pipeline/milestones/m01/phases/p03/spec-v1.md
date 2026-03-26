I now have all the context needed. Here is the phase specification:

# Phase P03 — Position Manipulation

## Goal

Implement the three operations that mutate and query position state — `make_move`, `unmake_move`, and `is_square_attacked` — within the `crates/board/` crate. These are the primitives that move generation (M02) and search (M03+) will call directly; `make_move` followed by `unmake_move` must restore the position exactly, and `is_square_attacked` must correctly identify attacks from all piece types.

## Design Decisions

1. **`make_move` returns an undo struct, not a cloned position.** Rather than cloning the entire `Position` before modification, `make_move` returns a lightweight `UndoInfo` struct containing only the fields needed for restoration (captured piece, prior castling rights, prior en passant square, prior halfmove clock, prior Zobrist hash). This avoids copying 12 bitboards per move, which is critical for search performance.

2. **`is_square_attacked` uses ray-based sliding attack detection.** Since magic bitboards are not introduced until M02, sliding piece attacks (bishop, rook, queen) are computed by walking rays until hitting an occupied square. This is correct and sufficient for this phase; it will be used as the foundation for legality checks.

3. **`Position` fields are mutated in-place via `pub(crate)` setter methods.** The `Position` struct has private fields with public getters (established in P02). Rather than making fields `pub(crate)`, T01 will add `pub(crate)` setter/mutation methods to keep the existing access pattern consistent.

4. **Castling rights update via square-based mask table.** When a king or rook moves (or a rook is captured), castling rights must be updated. A const lookup table indexed by square provides the mask of rights to clear, avoiding per-move branching for every possible castling right change.

## Tasks

### T01 — `make_move`

Apply a move to the position, updating all piece and occupancy bitboards, castling rights, en passant square, halfmove clock, fullmove counter, and Zobrist hash. Handle all move types: quiet, capture, double pawn push, castling (move both king and rook), en passant (remove captured pawn from its actual square), and promotions (replace pawn with promoted piece). Return an `UndoInfo` struct containing all state needed by `unmake_move`.

**Deliverables:**
- `UndoInfo` struct holding captured piece, prior castling rights, prior en passant square, prior halfmove clock, and prior Zobrist hash
- `Position::make_move(&mut self, mv: Move) -> UndoInfo` method
- Incremental Zobrist hash update (not full recomputation)
- Unit tests for each move type: quiet, capture, double pawn push, castling (both sides, both colors), en passant, promotion (all four piece types), promotion-capture

### T02 — `unmake_move`

Restore the position to its exact prior state using the `UndoInfo` returned by `make_move`. Must be the exact inverse of `make_move` for all move types.

**Deliverables:**
- `Position::unmake_move(&mut self, mv: Move, undo: UndoInfo)` method
- Unit tests verifying round-trip identity: for each move type, `make_move` followed by `unmake_move` restores every `Position` field (piece bitboards, occupancy, side to move, castling rights, en passant, clocks, Zobrist hash) to its original value
- FEN round-trip tests: `pos.to_fen()` is identical before and after a `make_move`/`unmake_move` cycle

### T03 — `is_square_attacked`

Determine whether a given square is attacked by a given side. Must account for all six piece types: pawn (directional captures), knight (L-shaped), bishop (diagonal rays), rook (orthogonal rays), queen (diagonal + orthogonal rays), king (adjacent squares). This function operates without full move generation — it is a building block used by the move generator, not the other way around.

**Deliverables:**
- `Position::is_square_attacked(&self, square: Square, by_side: Color) -> bool` method
- Ray-based sliding piece detection using the position's occupancy bitboard to stop at blocking pieces
- Unit tests covering: pawn attacks (both colors, both capture directions), knight attacks from all relative offsets, bishop/rook/queen attacks including blocking pieces, king attacks, positions with no attackers, multiple attackers on the same square

## Exit Criteria

1. `make_move` followed by `unmake_move` restores the position to its exact prior state for all move types (quiet, capture, promotion, castling, en passant, double pawn push) — verified by comparing all `Position` fields and FEN output
2. `is_square_attacked` correctly identifies attacks for all piece types in varied board positions
3. Zobrist hash remains consistent through make/unmake cycles — the hash after `unmake_move` equals the hash before the corresponding `make_move`
4. All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`

## Dependencies

- **Phase P01 (Primitive Types)** — complete. Provides `Bitboard`, `Square`, `File`, `Rank`, `Color`, `PieceKind`, `Piece`, `Move`, `MoveFlag` from the `types` crate
- **Phase P02 (Position State)** — complete. Provides `Position` struct, `CastlingRights`, FEN parsing/serialization, and Zobrist hashing infrastructure from the `board` crate

## Artifacts

- `crates/board/src/position.rs` — modified to add `pub(crate)` mutation methods, `UndoInfo` struct, `make_move`, `unmake_move`, and `is_square_attacked` implementations with unit tests
- `crates/board/src/lib.rs` — modified to export new public types (`UndoInfo`)