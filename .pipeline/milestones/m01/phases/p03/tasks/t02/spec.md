# T02 — `unmake_move`

## Objective

Implement `Position::unmake_move` to restore the position to its exact prior state using the `UndoInfo` returned by `make_move`. This must be the exact inverse of `make_move` for all move types.

## Deliverables

1. **`Position::unmake_move(&mut self, mv: Move, undo: UndoInfo)`** method that:
   - Toggles side to move back
   - Moves the piece from the destination square back to the source square
   - Restores captured piece to its square (if any)
   - Handles all move types in reverse:
     - **Quiet moves**: move piece back
     - **Captures**: move piece back, restore captured piece
     - **Double pawn push**: move pawn back (en passant restoration handled via UndoInfo)
     - **Castling (O-O and O-O-O, both colors)**: move both king and rook back to their original squares
     - **En passant**: move pawn back, restore the captured pawn to its actual square
     - **Promotions**: remove promoted piece, restore pawn to source square; handle promotion-captures
   - Restores castling rights from `UndoInfo`
   - Restores en passant square from `UndoInfo`
   - Restores halfmove clock from `UndoInfo`
   - Decrements fullmove counter (if side to move is now Black, i.e., undoing a Black move)
   - Restores Zobrist hash from `UndoInfo` (direct restoration, not incremental re-reversal)
   - Updates all piece and occupancy bitboards to reflect the restored state

2. **Unit tests** verifying round-trip identity for every move type

3. **FEN round-trip tests**: `pos.to_fen()` is identical before and after a `make_move`/`unmake_move` cycle

## Files to Create or Modify

- `crates/board/src/position.rs` — add `unmake_move` method and unit tests

## Key Constraints

- Must be the **exact inverse** of `make_move` — every `Position` field must be restored: all 12 piece bitboards, all occupancy bitboards, side to move, castling rights, en passant square, halfmove clock, fullmove counter, Zobrist hash
- No `unwrap()` in library code
- No `unsafe` unless provably necessary with `// SAFETY:` comment
- `///` doc comments on all public items

## Dependencies

- **T01 (`make_move`)** within this phase — `unmake_move` depends on the `UndoInfo` struct and `make_move` implementation
- Phase P01 and P02 — complete (same dependencies as T01)

## Verification Criteria

- For each move type (quiet, capture, double pawn push, castling both sides both colors, en passant, promotion all four types, promotion-capture): calling `make_move` followed by `unmake_move` restores **every** `Position` field to its original value
- FEN output (`pos.to_fen()`) is identical before and after a `make_move`/`unmake_move` cycle
- Zobrist hash after `unmake_move` equals the hash before the corresponding `make_move`
- Tests cover multiple positions (not just startpos) including positions with various castling rights, en passant states, and non-zero halfmove clocks
- `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` all pass