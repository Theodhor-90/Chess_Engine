# Task T02 — Knight Move Generation

## Objective

Implement pseudo-legal knight move generation using pre-computed attack tables within the `movegen` crate created in T01.

## Deliverables

1. **Knight attack table** — a `[Bitboard; 64]` array where each entry contains the bitboard of all squares a knight on that square can reach
2. **Attack table initialization** — computed at compile time (`const`) or via runtime-once initialization (`OnceLock` / `lazy_static`)
3. **Knight move generation function** that produces all pseudo-legal knight moves for the side to move

## Implementation Details

- For each of the 64 squares, pre-compute the knight's attack pattern (up to 8 target squares per position, fewer on edges/corners)
- The knight attack offsets from square (file, rank) are: (±1, ±2) and (±2, ±1), clamped to the board
- The move generation function:
  1. Gets the bitboard of friendly knights from `Position::piece_bitboard(Piece::new(side, PieceKind::Knight))`
  2. Iterates over each knight square
  3. Looks up `KNIGHT_ATTACKS[square.index()]`
  4. Removes squares occupied by friendly pieces (`& !Position::occupied_by(side)`)
  5. For each target square: if occupied by opponent → `MoveFlag::CAPTURE`, else → `MoveFlag::QUIET`
  6. Appends `Move::new(from, to, flag)` to the move list

## Implementation Constraints

- Pre-computed table must be initialized before first use (prefer `const` if feasible, otherwise `OnceLock`)
- Lives in the `movegen` crate alongside pawn generation from T01
- No `unsafe` unless provably necessary. No `unwrap()` in library code.
- Unit tests colocated via `#[cfg(test)] mod tests`

## Key Dependencies (from M01)

- `Bitboard` — bitwise ops, iteration
- `Square` — `index()`, `file()`, `rank()`, `new()`
- `Move`, `MoveFlag` — `QUIET`, `CAPTURE`
- `Color`, `Piece`, `PieceKind::Knight`
- `Position` — `piece_bitboard()`, `occupied_by()`, `side_to_move()`

## Dependencies on Other Tasks

- **T01** — the `movegen` crate and its `Cargo.toml` / workspace registration must exist before this task can add knight generation to it

## Verification Criteria

1. `cargo test -p movegen` passes all knight-related unit tests
2. Knight attack table contains correct attack bitboards for all 64 squares
3. Unit tests verify:
   - Knight on a corner square (e.g., A1) has exactly 2 legal targets
   - Knight on an edge square (e.g., A4) has exactly 4 legal targets
   - Knight on a center square (e.g., D4) has exactly 8 legal targets
   - Friendly-occupied squares are excluded from targets
   - Captures on opponent-occupied squares use `MoveFlag::CAPTURE`
   - Non-captures use `MoveFlag::QUIET`
4. `cargo clippy -- -D warnings` passes
5. `cargo fmt --check` passes