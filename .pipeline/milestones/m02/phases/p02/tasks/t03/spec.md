# Task T03 — Queen Move Generation and Combined Sliding Piece Interface

## Objective

Implement queen move generation as the union of bishop and rook attacks, create a combined `generate_sliding_moves` function that generates all sliding piece moves in one call, and update `lib.rs` exports.

## Deliverables

- A `crates/movegen/src/queens.rs` module containing:
  - `generate_queen_moves(pos: &Position, moves: &mut Vec<Move>)` — computes queen attacks as the union of `bishop_attacks` and `rook_attacks` for the queen's square, masks out friendly pieces, and produces pseudo-legal `Move` values.
  - `generate_sliding_moves(pos: &Position, moves: &mut Vec<Move>)` — a single function that calls `generate_bishop_moves`, `generate_rook_moves`, and `generate_queen_moves` to generate all sliding piece moves in one call.
  - Unit tests verifying queen move generation for three specific positions:
    1. **Starting position** — queens blocked by own pieces.
    2. **Midgame open board** — FEN: `r1bqk2r/ppppbppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQ1RK1 b kq - 5 4` (queen, bishop, and rook on an open board).
    3. **Sliding pieces blocked by pawns** — FEN: `4k3/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQ - 0 1`.

- Updated `crates/movegen/src/lib.rs` with:
  - `mod queens;` declaration.
  - Re-exports of `generate_bishop_moves`, `generate_rook_moves`, `generate_queen_moves`, and `generate_sliding_moves`.

## Design Constraints

1. **Queen = bishop | rook**: queen move generation must be computed as the union of bishop and rook attacks for the same square and occupancy. No separate queen attack tables.
2. **Function signature**: must match `fn(pos: &Position, moves: &mut Vec<Move>)` pattern.
3. **Combined interface**: `generate_sliding_moves` calls the individual bishop, rook, and queen generators — it is a convenience aggregator.
4. **No `unwrap()` in library code**.

## Dependencies

- **T01 (Magic bitboard infrastructure)** — requires `bishop_attacks` and `rook_attacks` lookup functions.
- **T02 (Bishop and rook move generation)** — requires `generate_bishop_moves` and `generate_rook_moves` for the combined interface.
- **Phase P01 complete** — relies on `movegen` crate structure and conventions.
- **Milestone M01 complete** — requires `Bitboard`, `Square`, `Move`, `Position`.

## Files to Create or Modify

- **Create**: `crates/movegen/src/queens.rs`
- **Modify**: `crates/movegen/src/lib.rs` — add `mod queens;` and re-export `generate_bishop_moves`, `generate_rook_moves`, `generate_queen_moves`, `generate_sliding_moves`.

## Verification Criteria

1. `generate_queen_moves` produces correct pseudo-legal moves equal to the union of bishop and rook attacks for every position.
2. `generate_sliding_moves` correctly calls all three sliding piece generators and produces the combined move list.
3. Unit tests pass for the three specified FEN positions.
4. All four functions (`generate_bishop_moves`, `generate_rook_moves`, `generate_queen_moves`, `generate_sliding_moves`) are re-exported from `lib.rs`.
5. `cargo test -p movegen` passes with no failures.
6. `cargo clippy -p movegen -- -D warnings` passes.
7. `cargo fmt --check -p movegen` passes.