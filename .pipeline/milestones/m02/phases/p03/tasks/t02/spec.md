# T02 — Legal Move Filter

## Objective

Implement a legality filter that removes pseudo-legal moves leaving the own king in check, and expose a top-level `generate_legal_moves(pos: &mut Position) -> Vec<Move>` entry point that becomes the canonical interface for move generation.

## Deliverables

- Updated: `crates/movegen/src/lib.rs` — add the legality filter and `generate_legal_moves` public function

## Implementation Details

### Legality Filter Algorithm

For each pseudo-legal move:
1. Call `pos.make_move(mv)` to get an `UndoInfo`.
2. Find the own king's square and check if it is attacked by the opponent via `pos.is_square_attacked(king_sq, opponent_color)`.
3. Call `pos.unmake_move(mv, undo)` to restore the position.
4. Retain the move only if the king was not left in check.

### Top-Level Entry Point

`generate_legal_moves(pos: &mut Position) -> Vec<Move>` must:
- Collect all pseudo-legal moves by calling `generate_pawn_moves`, `generate_knight_moves`, `generate_king_moves`, `generate_sliding_moves`, and `generate_castling_moves` (from T01) with a shared `Vec<Move>`.
- Apply the legality filter to remove illegal moves.
- Return the filtered list.
- Leave the position in its original state after the call — the mutation via make/unmake is purely internal.

The function takes `&mut Position` because the legality filter calls `make_move`/`unmake_move` internally.

## Dependencies

- **T01 (Castling Move Generation)** — `generate_castling_moves` must be available.
- **P01 (Non-Sliding Pieces)** — `generate_pawn_moves`, `generate_knight_moves`, `generate_king_moves`.
- **P02 (Sliding Pieces & Magic Bitboards)** — `generate_sliding_moves` (bishop, rook, queen via magic bitboard lookup).
- **M01 complete** — `Position` with `make_move`, `unmake_move`, `is_square_attacked`, and `UndoInfo`.

## Verification Criteria

- Unit tests covering:
  - Discovered check removal (moving a pinned piece exposes the king)
  - Pinned piece restriction (pinned piece can only move along the pin ray)
  - En passant legality (the rare case where capturing en passant removes two pawns from a rank and exposes the king to a rook/queen attack along that rank)
  - Positions where all pseudo-legal moves are legal vs. positions where some are filtered out
- `generate_legal_moves` produces no illegal moves and misses no legal moves for all tested positions
- Position is left unchanged after `generate_legal_moves` returns
- All unit tests pass via `cargo test`
- `cargo clippy -- -D warnings` passes
- `cargo fmt --check` passes