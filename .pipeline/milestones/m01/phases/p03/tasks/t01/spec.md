# T01 — `make_move`

## Objective

Implement `Position::make_move` to apply a chess move to the position, updating all internal state, and return an `UndoInfo` struct containing the state needed to reverse the move.

## Deliverables

1. **`UndoInfo` struct** holding:
   - Captured piece (`Option<Piece>`)
   - Prior castling rights (`CastlingRights`)
   - Prior en passant square (`Option<Square>`)
   - Prior halfmove clock (`u8` or equivalent)
   - Prior Zobrist hash

2. **`Position::make_move(&mut self, mv: Move) -> UndoInfo`** method that:
   - Removes the moving piece from its source square bitboard and places it on the destination square bitboard
   - Removes captured piece from its bitboard (if capture)
   - Updates all occupancy bitboards (per-color and total)
   - Handles all move types:
     - **Quiet moves**: standard piece relocation
     - **Captures**: remove opponent piece from destination
     - **Double pawn push**: set en passant square to the square behind the pawn
     - **Castling (O-O and O-O-O, both colors)**: move both king and rook to their destination squares
     - **En passant**: remove the captured pawn from its actual square (not the destination square)
     - **Promotions (all four piece types)**: remove pawn, place promoted piece; handle promotion-captures as well
   - Updates castling rights using a square-based mask table (const lookup indexed by from/to square provides mask of rights to clear)
   - Updates en passant square (set on double pawn push, clear otherwise)
   - Updates halfmove clock (reset on pawn move or capture, increment otherwise)
   - Updates fullmove counter (increment after Black's move)
   - Toggles side to move
   - Performs **incremental Zobrist hash update** (XOR out old state, XOR in new state — not full recomputation)
   - Returns the `UndoInfo` struct with saved prior state

3. **`pub(crate)` setter/mutation methods on `Position`** to keep the existing private-field access pattern consistent (as established in P02)

## Files to Create or Modify

- `crates/board/src/position.rs` — add `UndoInfo` struct, `make_move` method, `pub(crate)` mutation methods, and unit tests
- `crates/board/src/lib.rs` — export `UndoInfo` as a public type

## Key Constraints

- `Position` fields are private with public getters (from P02). Add `pub(crate)` setter/mutation methods rather than making fields `pub(crate)`
- Castling rights update must use a const lookup table indexed by square, avoiding per-move branching
- Zobrist hash must be updated incrementally (XOR delta), not recomputed from scratch
- No `unwrap()` in library code — use `Result`/`Option` propagation
- No `unsafe` unless provably necessary with `// SAFETY:` comment
- `///` doc comments on all public items

## Dependencies

- Phase P01 (Primitive Types) — complete. Provides `Bitboard`, `Square`, `File`, `Rank`, `Color`, `PieceKind`, `Piece`, `Move`, `MoveFlag` from the `types` crate
- Phase P02 (Position State) — complete. Provides `Position` struct, `CastlingRights`, FEN parsing/serialization, and Zobrist hashing infrastructure from the `board` crate
- No dependencies on other tasks within this phase

## Verification Criteria

- Unit tests pass for each move type: quiet, capture, double pawn push, castling (both sides for both colors), en passant, promotion (all four promoted piece types), promotion-capture
- After `make_move`, position state (bitboards, side to move, castling rights, en passant, clocks, Zobrist hash) reflects the applied move correctly
- Zobrist hash after `make_move` matches a full recomputation from the resulting position state
- `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` all pass