No `tests/` or `tools/` directory exists yet. Now I have all the context needed. Let me produce the plan.

## 1. Deliverables

| # | File | Action | Purpose |
|---|------|--------|---------|
| 1 | `crates/movegen/src/lib.rs` | Modified | Add `generate_legal_moves` public function and re-export it |

## 2. Dependencies

No new dependencies. The implementation uses only existing crate APIs:
- `chess-types`: `Move`, `Color`, `Piece`, `PieceKind`, `Square`
- `chess-board`: `Position`, `UndoInfo`
- All pseudo-legal move generators already exported from `crates/movegen/src/lib.rs`: `generate_pawn_moves`, `generate_knight_moves`, `generate_king_moves`, `generate_sliding_moves`, `generate_castling_moves`

## 3. Implementation Details

### `crates/movegen/src/lib.rs`

**Purpose**: Add the `generate_legal_moves` public function that serves as the canonical move generation entry point.

**New export**: `pub fn generate_legal_moves(pos: &mut Position) -> Vec<Move>`

**Key logic**:

1. **Collect pseudo-legal moves**: Allocate a `Vec<Move>`, then call each generator in order:
   - `generate_pawn_moves(pos, &mut moves)`
   - `generate_knight_moves(pos, &mut moves)`
   - `generate_king_moves(pos, &mut moves)`
   - `generate_sliding_moves(pos, &mut moves)` (bishops, rooks, queens)
   - `generate_castling_moves(pos, &mut moves)`

2. **Legality filter**: Use `Vec::retain` (or equivalent filter) to keep only legal moves. For each move `mv`:
   - Call `let undo = pos.make_move(mv)` 
   - Determine the king square of the side that just moved (which is now `pos.side_to_move().opposite()`) by finding the king bitboard and extracting the LSB square
   - Check `pos.is_square_attacked(king_sq, pos.side_to_move())` — if true, the move is illegal
   - Call `pos.unmake_move(mv, undo)` to restore position
   - Retain the move only if the king was NOT attacked

3. **King square lookup**: After `make_move`, the side that moved is `pos.side_to_move().opposite()`. Find the king via `pos.piece_bitboard(Piece::new(our_color, PieceKind::King)).lsb()` (or equivalent bit extraction from the Bitboard iterator).

4. **Position invariant**: After the function returns, the position is in its original state because every `make_move` is paired with an `unmake_move`.

**Implementation note on retain vs. manual loop**: Since `Vec::retain` passes `&T` (not `&mut T`), and we need `&mut Position` to call `make_move`/`unmake_move`, the filter should be done with a manual loop that builds a new `Vec<Move>`, or by collecting indices. The simplest correct approach: iterate over the pseudo-legal moves, for each one do make/check/unmake, and push legal ones to a result vector.

```rust
pub fn generate_legal_moves(pos: &mut Position) -> Vec<Move> {
    let mut pseudo_legal = Vec::new();
    generate_pawn_moves(pos, &mut pseudo_legal);
    generate_knight_moves(pos, &mut pseudo_legal);
    generate_king_moves(pos, &mut pseudo_legal);
    generate_sliding_moves(pos, &mut pseudo_legal);
    generate_castling_moves(pos, &mut pseudo_legal);

    let mut legal = Vec::new();
    let us = pos.side_to_move();
    let them = us.opposite();
    let king_piece = Piece::new(us, PieceKind::King);

    for mv in pseudo_legal {
        let undo = pos.make_move(mv);
        let king_sq = {
            let king_bb = pos.piece_bitboard(king_piece);
            // Extract the single set bit as a square index
            Square::new(king_bb.lsb() as u8).expect("king must exist")
        };
        let in_check = pos.is_square_attacked(king_sq, them);
        pos.unmake_move(mv, undo);
        if !in_check {
            legal.push(mv);
        }
    }

    legal
}
```

**Note on Bitboard::lsb()**: Need to verify the exact API. The `Bitboard` type implements `Iterator` (as seen in `queens.rs` line 11: `for from_idx in queens`), iterating over set bit indices. The implementation should use whichever method is available to extract the king square index. If `lsb()` is not available, use the iterator: `king_bb.into_iter().next().expect(...)`.

**Imports needed**: `chess_board::Position`, `chess_types::{Color, Move, Piece, PieceKind, Square}`.

**Unit tests** (added as `#[cfg(test)] mod tests` in `lib.rs`):

1. **Discovered check removal**: Position where a piece is pinned to the king — moving the pinned piece should be filtered out. FEN: `4k3/8/8/8/8/8/4r3/4K3 w - - 0 1` — white king on e1, black rook on e2. The only moves for the white king are those that don't stay on the e-file (d1, d2, f1, f2 — but d2 and f2 also need checking). A better test: `8/8/8/8/8/8/3k4/R3K3 w - - 0 1` — check specific legal move count.

2. **Pinned piece restriction**: FEN with a piece pinned along a ray — verify that it can only move along the pin line. Example: `4k3/4r3/8/8/8/8/4B3/4K3 w - - 0 1` — white bishop on e2 is pinned by black rook on e7 to white king on e1. The bishop has zero legal moves (it's on a diagonal, pin is on a file).

3. **En passant legality (discovered check)**: The rare horizontal-pin en passant case. FEN: `8/8/8/8/k2Pp2R/8/8/4K3 b - d3 0 1` — black king on a4, white pawn on d4, black pawn on e4, white rook on h4, EP square d3. The en passant capture `exd3` would remove both pawns from rank 4, exposing the black king to the white rook. This en passant must be filtered out.

4. **All pseudo-legal moves are legal**: Starting position — verify that `generate_legal_moves` returns 20 moves (the well-known starting position legal move count).

5. **Position unchanged after call**: Call `generate_legal_moves`, verify FEN and hash are identical before and after.

## 4. API Contracts

### `generate_legal_moves`

```rust
pub fn generate_legal_moves(pos: &mut Position) -> Vec<Move>
```

**Input**: Mutable reference to a `Position` (mutable because make/unmake is used internally).

**Output**: `Vec<Move>` containing all legal moves for the side to move.

**Postcondition**: The `Position` is in its original state after the call returns. The FEN and Zobrist hash are unchanged.

**Examples**:
- Starting position → 20 legal moves
- Stalemate position (`5k2/5P2/5K2/8/8/8/8/8 b - - 0 1`) → 0 legal moves
- Position in check with escape moves → only escape moves returned

## 5. Test Plan

All tests are colocated unit tests in `crates/movegen/src/lib.rs` within a `#[cfg(test)] mod tests` block.

| # | Test Name | Setup | Assertion |
|---|-----------|-------|-----------|
| 1 | `test_discovered_check_filtered` | FEN with a pinned piece that would expose king if moved off the pin line | Verify the pinned piece's illegal moves are not in the result |
| 2 | `test_pinned_piece_restriction` | FEN: `4k3/4r3/8/8/8/8/4B3/4K3 w - - 0 1` (bishop pinned on file) | Bishop has no legal moves (diagonal moves blocked by file pin); confirm by checking no moves have from=E2 |
| 3 | `test_en_passant_discovered_check` | FEN: `8/8/8/8/k2Pp2R/8/8/4K3 b - d3 0 1` | En passant capture e4d3 is NOT in the legal move list |
| 4 | `test_startpos_legal_moves` | `Position::startpos()` | `generate_legal_moves` returns exactly 20 moves |
| 5 | `test_position_unchanged_after_generate` | Any position (e.g., startpos) | FEN and hash are identical before and after calling `generate_legal_moves` |

## 6. Implementation Order

1. **Add imports and the `generate_legal_moves` function to `crates/movegen/src/lib.rs`** — implement the pseudo-legal collection and legality filter as described in Section 3. Verify Bitboard API for king square extraction by checking the existing iterator pattern used in other generators.

2. **Add the `pub use` re-export** — ensure `generate_legal_moves` is publicly accessible from the crate root (it's defined directly in `lib.rs`, so it's already public at crate level — no additional re-export needed beyond the `pub fn`).

3. **Add unit tests** — implement the five tests described in the test plan within a `#[cfg(test)] mod tests` block in `lib.rs`.

4. **Run `cargo test -p chess-movegen`** to verify all tests pass.

5. **Run `cargo clippy -- -D warnings`** to verify no warnings.

6. **Run `cargo fmt --check`** to verify formatting.

## 7. Verification Commands

```bash
cargo test -p chess-movegen
cargo clippy -- -D warnings
cargo fmt --check
```

## 8. Design Deviations

**Deviation: Bitboard king square extraction method**

- The task spec says "Find the own king's square" without specifying the mechanism.
- The `Bitboard` type implements `Iterator` yielding `u32` bit indices (as used in `queens.rs`: `for from_idx in queens`). There is no documented `lsb()` method visible in the public API examined.
- The implementation will use the iterator pattern: `pos.piece_bitboard(king_piece).into_iter().next().expect("king must exist")` to extract the king's square index, then convert via `Square::new(idx as u8)`. This is consistent with the pattern used across all existing generators.

No other deviations are needed. The make/unmake legality filter approach matches the phase spec's design decision #2 exactly.