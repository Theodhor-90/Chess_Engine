Now I have all the information needed. Here is the complete implementation plan:

## 1. Deliverables

- **`crates/board/src/position.rs`** — modified to add:
  - `Position::is_square_attacked(&self, square: Square, by_side: Color) -> bool` public method
  - Ray-walking helper logic for sliding piece detection (bishop/rook/queen) using the position's `occupied` bitboard
  - Unit tests covering all six piece types, board-edge cases, blocking pieces, negative cases, and multiple-attacker scenarios

No new files are created. No modifications to `crates/board/src/lib.rs` are needed (no new public types are introduced).

## 2. Dependencies

No new dependencies. The implementation uses only existing types from:
- `chess_types`: `Bitboard`, `Color`, `Square`, `Piece`, `PieceKind`, `File`, `Rank`
- `crate::position`: `Position` (self)

## 3. Implementation Details

### `Position::is_square_attacked`

**Purpose**: Determine if a given square is attacked by any piece of the specified color.

**Signature**: `pub fn is_square_attacked(&self, square: Square, by_side: Color) -> bool`

**Key Logic** — check each piece type, returning `true` on first hit (short-circuit):

1. **Pawn attacks**: Compute the squares from which a pawn of `by_side` could attack the target square (reverse the pawn capture direction). For `by_side == White`: check squares at `(sq - 7)` and `(sq - 9)` — i.e., the squares diagonally below the target (from which white pawns capture upward). For `by_side == Black`: check squares at `(sq + 7)` and `(sq + 9)` — i.e., the squares diagonally above the target (from which black pawns capture downward). Must guard against file wrapping: a pawn on file A cannot capture from file H, and vice versa. Check if those squares contain a pawn of `by_side` by intersecting the pawn bitboard with the computed attack squares.

2. **Knight attacks**: From the target square, compute all 8 possible L-shaped offsets: `(+2,+1), (+2,-1), (-2,+1), (-2,-1), (+1,+2), (+1,-2), (-1,+2), (-1,-2)`. Use signed arithmetic on file/rank to guard against board-edge wrapping (validate that the resulting file is within 0..7 and rank is within 0..7). Check if any of those squares contain a knight of `by_side`.

3. **Bishop/Queen attacks (diagonal rays)**: Walk four diagonal directions from the target square: `(+1,+1), (+1,-1), (-1,+1), (-1,-1)` (expressed as file delta, rank delta). For each direction, step one square at a time. At each step, check if the resulting file and rank are in bounds (0..7). If the square is occupied (check `self.occupied`), examine the piece: if it is a bishop or queen of `by_side`, return `true`. Otherwise the ray is blocked — stop this direction.

4. **Rook/Queen attacks (orthogonal rays)**: Walk four orthogonal directions: `(+1,0), (-1,0), (0,+1), (0,-1)`. Same ray-walking logic as bishops. If the first occupied square on a ray is a rook or queen of `by_side`, return `true`.

5. **King attacks**: Check the 8 adjacent squares around the target. Use file/rank bounds checking to avoid wrapping. If any adjacent square contains a king of `by_side`, return `true`.

6. If none of the checks found an attacker, return `false`.

**Implementation approach**: Use `Square::index()` to compute `file = index % 8` and `rank = index / 8` as `i8` values for signed arithmetic. For each candidate attacker square, validate `0 <= file < 8` and `0 <= rank < 8`, then construct the `Square` and check against the relevant piece bitboard. For sliding pieces, iterate using a loop over `(df, dr)` direction pairs, stepping along the ray.

## 4. API Contracts

**Input**: `square: Square` (the square to query), `by_side: Color` (the attacking side)

**Output**: `bool` — `true` if at least one piece of `by_side` attacks the given square, `false` otherwise.

**Examples**:
- Starting position, `is_square_attacked(Square::E2, Color::Black)` → `false` (no black piece attacks e2)
- Starting position, `is_square_attacked(Square::E2, Color::White)` → `true` (white king on e1 attacks e2; white pawns on d2/f2 attack e3 but not e2 — however the king does)
- Position with white rook on a1 and empty a-file: `is_square_attacked(Square::A8, Color::White)` → `true` (rook attacks along file)
- Position with white bishop on c1, pawn on d2 blocking: `is_square_attacked(Square::F4, Color::White)` → `false` (diagonal blocked by pawn)

## 5. Test Plan

All tests are added as `#[test]` functions inside the existing `#[cfg(test)] mod tests` block in `crates/board/src/position.rs`.

**Test setup pattern**: Use `Position::from_fen(fen)` to create specific positions, then call `pos.is_square_attacked(square, color)` and assert the result.

### Per-test specification:

1. **`is_square_attacked_pawn_white`**: White pawn on d4. Assert e5 and c5 are attacked by white. Assert d5, d3, e4 are NOT attacked by white.

2. **`is_square_attacked_pawn_black`**: Black pawn on e5. Assert d4 and f4 are attacked by black. Assert e4, e6, d5 are NOT attacked by black.

3. **`is_square_attacked_pawn_edge_file`**: White pawn on a4. Assert b5 is attacked by white. Assert that no wrapping occurs — square on h-file is NOT attacked. Similarly, black pawn on h5 attacks g4 but not a4.

4. **`is_square_attacked_knight`**: White knight on d4. Assert all 8 L-shaped destinations (c2, e2, b3, f3, b5, f5, c6, e6) are attacked by white. Assert d5, d3, e4 are NOT attacked.

5. **`is_square_attacked_knight_corner`**: White knight on a1. Assert only the valid destinations (b3, c2) are attacked — verifies edge-of-board handling.

6. **`is_square_attacked_bishop_unblocked`**: White bishop on d4, no blocking pieces on diagonals. Assert a1, b2, c3, e5, f6, g7, h8, a7, g1 are attacked by white.

7. **`is_square_attacked_bishop_blocked`**: White bishop on d4, white pawn on f6 blocking the diagonal. Assert e5 IS attacked but f6, g7, h8 are NOT attacked by the bishop (blocked by own pawn).

8. **`is_square_attacked_rook_unblocked`**: White rook on d4, no blocking pieces on d-file or rank 4. Assert d1, d8, a4, h4 are attacked.

9. **`is_square_attacked_rook_blocked`**: White rook on a1, white pawn on a3 blocking the file. Assert a2 IS attacked but a4, a5..a8 are NOT attacked by the rook.

10. **`is_square_attacked_queen`**: White queen on d4. Assert it attacks both diagonal and orthogonal squares — combining bishop and rook coverage. Assert a specific diagonal (e.g., f6) and orthogonal (e.g., d8) square are attacked.

11. **`is_square_attacked_king`**: White king on e1. Assert all adjacent squares (d1, d2, e2, f2, f1) are attacked. Assert non-adjacent squares (e3, e4) are NOT attacked.

12. **`is_square_attacked_king_corner`**: White king on a1. Assert only the 3 valid adjacent squares (a2, b1, b2) are attacked.

13. **`is_square_attacked_no_attackers`**: A position where a specific square has no attackers of a given side. Assert `false`.

14. **`is_square_attacked_multiple_attackers`**: A position where a square is attacked by two or more pieces of the same side (e.g., queen and knight both attack the same square). Assert `true` — the function only returns a bool, but the presence of multiple attackers should not cause issues.

## 6. Implementation Order

1. **Add `is_square_attacked` method** to the `impl Position` block in `crates/board/src/position.rs`, after the existing `unmake_move` method and before the `pub(crate)` setter methods. Implement all six piece-type checks (pawn, knight, bishop, rook, queen, king) with ray-walking for sliding pieces.

2. **Add unit tests** to the existing `#[cfg(test)] mod tests` block in the same file.

3. **Verify** with `cargo fmt`, `cargo clippy`, and `cargo test`.

## 7. Verification Commands

```bash
cd /Users/theodhor/Desktop/Portfolio/Chess_Engine && cargo fmt --check
cd /Users/theodhor/Desktop/Portfolio/Chess_Engine && cargo clippy -- -D warnings
cd /Users/theodhor/Desktop/Portfolio/Chess_Engine && cargo test -p chess-board
```

## 8. Design Deviations

None.