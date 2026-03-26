# T03 — `is_square_attacked`

## Objective

Implement `Position::is_square_attacked` to determine whether a given square is attacked by a given side, accounting for all six piece types. This function is a building block for legality checks (is the king in check?) and king safety evaluation — it operates without full move generation.

## Deliverables

1. **`Position::is_square_attacked(&self, square: Square, by_side: Color) -> bool`** method that checks:
   - **Pawn attacks**: directional captures (white pawns attack diagonally up, black pawns attack diagonally down). Check if any pawn of `by_side` is on a square that attacks the target square.
   - **Knight attacks**: L-shaped offsets from the target square. Check if any knight of `by_side` is on one of those squares.
   - **Bishop attacks**: walk the four diagonal rays from the target square, stopping at the first occupied square. If that piece is a bishop or queen of `by_side`, the square is attacked.
   - **Rook attacks**: walk the four orthogonal rays from the target square, stopping at the first occupied square. If that piece is a rook or queen of `by_side`, the square is attacked.
   - **Queen attacks**: covered by the bishop + rook ray checks above.
   - **King attacks**: check the eight adjacent squares for a king of `by_side`.

2. **Ray-based sliding piece detection** using the position's occupancy bitboard to stop at blocking pieces. Since magic bitboards are not yet available (introduced in M02), sliding attacks are computed by walking rays step by step.

3. **Unit tests** covering all attack scenarios

## Files to Create or Modify

- `crates/board/src/position.rs` — add `is_square_attacked` method and unit tests

## Key Constraints

- **No magic bitboards** — sliding piece detection must use ray walking (iterating along directions until hitting an occupied square or edge of board). Magic bitboards come in M02.
- The function is read-only (`&self`) — it does not modify position state
- Must be efficient enough for use in legality checking (called for every pseudo-legal move to verify king is not in check)
- No `unwrap()` in library code
- No `unsafe` unless provably necessary with `// SAFETY:` comment
- `///` doc comments on all public items

## Dependencies

- **T01 (`make_move`)** and **T02 (`unmake_move`)** within this phase — while `is_square_attacked` doesn't call make/unmake, the phase is designed so T03 builds on T01/T02 being complete (the position mutation methods should already be in place)
- Phase P01 and P02 — complete (provides all types and Position infrastructure)

## Verification Criteria

- Pawn attacks: both colors, both capture directions, edge cases at board edges
- Knight attacks: all 8 relative offsets, edge cases where some offsets are off-board
- Bishop attacks: diagonal rays including blocking pieces (friendly and enemy) stopping the ray
- Rook attacks: orthogonal rays including blocking pieces stopping the ray
- Queen attacks: combined diagonal and orthogonal coverage
- King attacks: all 8 adjacent squares, edge cases at board corners/edges
- Negative cases: positions with no attackers on the target square
- Multiple attackers: square attacked by more than one piece simultaneously
- `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` all pass