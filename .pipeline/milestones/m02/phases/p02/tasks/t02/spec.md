# Task T02 — Bishop and Rook Move Generation

## Objective

Implement `generate_bishop_moves` and `generate_rook_moves` functions that use the magic bitboard lookup from T01 to produce pseudo-legal moves for bishops and rooks.

## Deliverables

- A `crates/movegen/src/bishops.rs` module containing:
  - `generate_bishop_moves(pos: &Position, moves: &mut Vec<Move>)` — uses `bishop_attacks` from `magic.rs`, masks out friendly piece occupancy, and produces pseudo-legal `Move` values appended to the move list.
  - Unit tests verifying correct move generation for representative positions.

- A `crates/movegen/src/rooks.rs` module containing:
  - `generate_rook_moves(pos: &Position, moves: &mut Vec<Move>)` — uses `rook_attacks` from `magic.rs`, masks out friendly piece occupancy, and produces pseudo-legal `Move` values appended to the move list.
  - Unit tests verifying correct move generation for representative positions.

## Design Constraints

1. **Function signature**: must match the existing pattern `fn(pos: &Position, moves: &mut Vec<Move>)` established in P01.
2. **Pseudo-legal moves only**: these functions generate all destination squares not occupied by friendly pieces. Legality filtering (king in check) is handled in Phase P03.
3. **Use magic bitboard lookup from T01**: call `bishop_attacks(square, occupancy)` / `rook_attacks(square, occupancy)` from `magic.rs` — do not reimplement attack generation.
4. **Move encoding**: use the `Move` type and `MoveFlag` conventions from the `types` crate (M01).
5. **No `unwrap()` in library code**.

## Dependencies

- **T01 (Magic bitboard infrastructure)** — requires `bishop_attacks` and `rook_attacks` lookup functions.
- **Phase P01 complete** — relies on `movegen` crate structure, module conventions, and the move generation function signature pattern.
- **Milestone M01 complete** — requires `Bitboard`, `Square`, `Piece`, `Move` with flags, and `Position` for piece/occupancy bitboards.

## Files to Create or Modify

- **Create**: `crates/movegen/src/bishops.rs`
- **Create**: `crates/movegen/src/rooks.rs`
- **Modify**: `crates/movegen/src/lib.rs` — add `mod bishops;` and `mod rooks;` declarations, re-export generation functions.

## Verification Criteria

1. `generate_bishop_moves` produces correct pseudo-legal moves that include all valid destination squares and exclude squares occupied by friendly pieces.
2. `generate_rook_moves` produces correct pseudo-legal moves that include all valid destination squares and exclude squares occupied by friendly pieces.
3. Unit tests pass for representative positions (open boards, blocked positions, edge cases).
4. `cargo test -p movegen` passes with no failures.
5. `cargo clippy -p movegen -- -D warnings` passes.
6. `cargo fmt --check -p movegen` passes.