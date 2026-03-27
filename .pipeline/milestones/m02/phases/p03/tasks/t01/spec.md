# T01 — Castling Move Generation

## Objective

Implement `generate_castling_moves(pos: &Position, moves: &mut Vec<Move>)` to generate O-O (kingside) and O-O-O (queenside) castling moves for the side to move.

## Deliverables

- New file: `crates/movegen/src/castling.rs` — castling move generation module
- Updated: `crates/movegen/src/lib.rs` — export the `castling` module

## Implementation Details

For each potential castling move (white kingside, white queenside, black kingside, black queenside), verify three conditions:

1. **Castling right is present** — e.g., `pos.castling_rights().contains(CastlingRights::WHITE_KINGSIDE)` for white kingside, and analogously for the other three.
2. **Path is clear** — no pieces occupy the squares between the king and rook (check the occupancy bitboard against the intermediate squares).
3. **No attacked squares** — no square the king passes through (including origin and destination) is attacked by the opponent, verified via `pos.is_square_attacked(square, opponent_color)`.

Emit moves using:
- `Move::new(king_from, king_to, MoveFlag::KING_CASTLE)` for kingside castling
- `Move::new(king_from, king_to, MoveFlag::QUEEN_CASTLE)` for queenside castling

The function follows the same signature pattern (`pos: &Position, moves: &mut Vec<Move>`) as all existing piece generators from P01 and P02.

## Dependencies

- **M01 complete** — `Position` struct with `castling_rights()`, `is_square_attacked(&self, Square, Color) -> bool`, and all core types (`Square`, `Move`, `MoveFlag`, `CastlingRights`, `Color`, `Bitboard`).
- **P01 & P02 complete** — existing piece generators establish the function signature convention.

## Verification Criteria

- Unit tests covering:
  - Castling available and correctly generated (both O-O and O-O-O for both colors)
  - Castling blocked by intervening pieces
  - Castling blocked by attacked squares (including king's origin, transit, and destination squares)
  - Castling rights revoked (king or rook has moved)
  - Both colors (white and black) tested independently
- All unit tests pass via `cargo test`
- `cargo clippy -- -D warnings` passes
- `cargo fmt --check` passes