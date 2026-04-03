I now have all the context I need. Let me produce the implementation plan.

## 1. Deliverables

| # | File | Purpose |
|---|------|---------|
| 1 | `crates/movegen/src/king.rs` | King attack table (`[Bitboard; 64]`) and `generate_king_moves` function with colocated unit tests |
| 2 | `crates/movegen/src/lib.rs` | Updated to declare the `king` module and re-export `generate_king_moves` |

## 2. Dependencies

No new dependencies required. The `movegen` crate already depends on `chess-types` and `chess-board` (established in T01). All necessary types and APIs exist:

- `chess-types`: `Bitboard`, `Square`, `Move`, `MoveFlag`, `Piece`, `PieceKind`, `Color`
- `chess-board`: `Position` (including `piece_bitboard`, `occupied_by`, `side_to_move`, `is_square_attacked`)

## 3. Implementation Details

### 3.1 `crates/movegen/src/king.rs`

**Purpose**: Pre-computed king attack table and pseudo-legal king move generation.

**Exports**: `generate_king_moves(pos: &Position, moves: &mut Vec<Move>)`

**Constants**:

- `KING_ATTACKS: [Bitboard; 64]` — compile-time `const` array. Each entry holds the bitboard of squares reachable by a king on that square (up to 8 adjacent squares: N, NE, E, SE, S, SW, W, NW).

**`init_king_attacks() -> [Bitboard; 64]`** (private `const fn`):

For each of the 64 squares, compute the king's attack pattern:
- Extract file (sq % 8) and rank (sq / 8) as `i8`
- For each of the 8 direction offsets `(df, dr)` in `[(−1,−1), (−1,0), (−1,1), (0,−1), (0,1), (1,−1), (1,0), (1,1)]`:
  - Compute target file `tf = file + df`, target rank `tr = rank + dr`
  - If `tf` and `tr` are both in `[0, 7]`, set bit `tr * 8 + tf` in the attack bitboard
- Store the resulting bitboard in `table[sq]`

This follows the exact same pattern as `init_knight_attacks()` in `knights.rs`, just with king offsets instead of knight offsets.

**`generate_king_moves(pos: &Position, moves: &mut Vec<Move>)`** (public):

1. `let us = pos.side_to_move()`
2. `let them = us.opposite()`
3. Get king bitboard: `pos.piece_bitboard(Piece::new(us, PieceKind::King))`
4. Extract king square via `.lsb()` — exactly one king exists, so `.lsb().expect("king must exist")` is valid (this is a library crate but the invariant of having a king is structural; however, to avoid `unwrap()`/`expect()` in library code per project conventions, use an early return if `lsb()` returns `None`)
5. Look up `KING_ATTACKS[king_sq as usize]`
6. Remove friendly-occupied squares: `let candidates = attacks & !pos.occupied_by(us)`
7. For each candidate target square in `candidates`:
   - Check `pos.is_square_attacked(target_sq, them)` — if attacked, skip this square
   - Otherwise: if the target square is occupied by an enemy piece (check against `pos.occupied_by(them)`), emit `Move::new(from, to, MoveFlag::CAPTURE)`; else emit `Move::new(from, to, MoveFlag::QUIET)`

**Important implementation note from the task spec**: The king itself may need to be temporarily removed from occupancy when checking if target squares are attacked. The `is_square_attacked` function in the board crate may use the current occupancy (which includes the king on its current square). If a sliding piece attacks through the king's current square to the target square, `is_square_attacked` might miss it because the king blocks the ray. This must be verified during implementation. If `is_square_attacked` does not handle this, the king's bit should be temporarily cleared from the position's occupancy before calling `is_square_attacked` for each candidate square — or alternatively, the caller can accept this as a pseudo-legal limitation that the legal move filter in P03/T02 will resolve. Per the task spec, this function generates **pseudo-legal** moves with the opponent-attack exclusion as a best-effort filter, and the legal move filter in P03/T02 handles the remaining cases.

**Castling**: NOT generated here. Castling belongs to Phase P03/T01 per the task spec.

### 3.2 `crates/movegen/src/lib.rs`

**Purpose**: Add the `king` module declaration and re-export.

**Changes**:
- Add `pub mod king;` alongside the existing `pub mod knights;` and `pub mod pawns;`
- Add `pub use king::generate_king_moves;` alongside existing re-exports

## 4. API Contracts

**`generate_king_moves(pos: &Position, moves: &mut Vec<Move>)`**

- **Input**: A reference to a `Position` and a mutable reference to a `Vec<Move>` to append moves to
- **Output**: Appends pseudo-legal king moves to `moves`. Each move has:
  - `from`: the king's current square
  - `to`: a target square from the attack table, not occupied by a friendly piece, not attacked by the opponent
  - `flags`: `MoveFlag::CAPTURE` (value `4`) if target is occupied by opponent, `MoveFlag::QUIET` (value `0`) otherwise
- **Does not generate**: Castling moves (O-O, O-O-O)

**Example**:
```
Position: "4k3/8/8/8/8/8/8/4K3 w - - 0 1" (white king on e1, black king on e8)
King on e1 has 5 attack squares: d1, f1, d2, e2, f2.
None occupied by friendly pieces.
None attacked by the opponent king (d2/e2 are not attacked by black king on e8).
Result: 5 quiet moves from e1 to {d1, d2, e2, f1, f2}.
```

**`KING_ATTACKS: [Bitboard; 64]`** (crate-private constant):

- `KING_ATTACKS[0]` (a1): 3 bits set (a2, b1, b2) → `pop_count() == 3`
- `KING_ATTACKS[24]` (a4, edge): 5 bits set → `pop_count() == 5`
- `KING_ATTACKS[27]` (d4, center): 8 bits set → `pop_count() == 8`

## 5. Test Plan

All tests colocated in `crates/movegen/src/king.rs` under `#[cfg(test)] mod tests`.

Uses the same test helper pattern as sibling modules:
```rust
fn has_move(moves: &[Move], from: Square, to: Square, flags: u16) -> bool {
    moves.contains(&Move::new(from, to, flags))
}
```

### Test 1: `test_king_attacks_corner_a1`
- **Verifies**: Attack table correctness for corner square
- **Setup**: Read `KING_ATTACKS[Square::A1.index() as usize]`
- **Assert**: `pop_count() == 3`, bits set for B1, A2, B2

### Test 2: `test_king_attacks_edge_a4`
- **Verifies**: Attack table correctness for edge square
- **Setup**: Read `KING_ATTACKS[Square::A4.index() as usize]`
- **Assert**: `pop_count() == 5`

### Test 3: `test_king_attacks_center_d4`
- **Verifies**: Attack table correctness for center square
- **Setup**: Read `KING_ATTACKS[Square::D4.index() as usize]`
- **Assert**: `pop_count() == 8`

### Test 4: `test_king_excludes_friendly`
- **Verifies**: Friendly-occupied squares excluded from targets
- **Setup**: FEN `"4k3/8/8/8/8/8/4N3/4K3 w - - 0 1"` (white king on e1, white knight on e2)
- **Assert**: No move from e1 to e2 (neither quiet nor capture)

### Test 5: `test_king_excludes_attacked_squares`
- **Verifies**: Squares attacked by opponent are excluded
- **Setup**: FEN `"4k3/8/8/8/8/8/8/r3K3 w - - 0 1"` (white king on e1, black rook on a1 — attacks d1 and f1 is safe but the entire first rank except e1 is attacked)
- **Assert**: King cannot move to d1 (attacked by rook). Specific squares attacked by the rook are excluded.

### Test 6: `test_king_captures`
- **Verifies**: Captures on opponent-occupied squares use `MoveFlag::CAPTURE`
- **Setup**: FEN `"4k3/8/8/8/8/8/4n3/4K3 w - - 0 1"` (white king on e1, black knight on e2)
- **Assert**: `has_move(moves, Square::E1, Square::E2, MoveFlag::CAPTURE)` is true

### Test 7: `test_king_quiet_moves`
- **Verifies**: Non-captures use `MoveFlag::QUIET`
- **Setup**: FEN `"4k3/8/8/8/8/8/8/4K3 w - - 0 1"` (white king on e1, no nearby pieces)
- **Assert**: All generated moves have `MoveFlag::QUIET`, count is 5 (d1, d2, e2, f1, f2)

### Test 8: `test_black_king_moves`
- **Verifies**: Works for black side to move
- **Setup**: FEN `"4k3/8/8/8/8/8/8/4K3 b - - 0 1"` (black king on e8)
- **Assert**: Generates king moves from e8, count is 5 (d8, f8, d7, e7, f7)

## 6. Implementation Order

1. **Create `crates/movegen/src/king.rs`**: Implement `init_king_attacks()` const fn and `KING_ATTACKS` constant, following the exact pattern from `knights.rs`
2. **Implement `generate_king_moves`** in the same file: king square extraction, attack lookup, friendly filtering, opponent-attack filtering, move emission with correct flags
3. **Add unit tests** in the same file under `#[cfg(test)] mod tests`
4. **Update `crates/movegen/src/lib.rs`**: Add `pub mod king;` and `pub use king::generate_king_moves;`
5. **Run verification commands** to confirm correctness

## 7. Verification Commands

```bash
cargo fmt --check -p chess-movegen
cargo clippy -p chess-movegen -- -D warnings
cargo test -p chess-movegen
```

## 8. Design Deviations

**Deviation: Handling of king moving through sliding piece rays (discovered attacks)**

- **What the task spec says**: "the king itself may need to be temporarily removed from occupancy when checking attacked squares"
- **Why this is complex**: Temporarily modifying `Position` occupancy to remove the king would require either a mutable reference to position (breaking the `&Position` API) or cloning the entire position per move candidate (expensive). The `is_square_attacked` function operates on the position as-is.
- **Chosen approach**: Call `is_square_attacked` with the position as-is. This means a sliding piece whose ray passes through the king's current square to a candidate target square may not be detected as attacking that target. This is acceptable because the function generates *pseudo-legal* moves, and the legal move filter in Phase P03/T02 will catch any remaining illegal king moves (moves that leave the king in check via discovered attack along the vacated square's ray). This matches the task spec's own characterization: "This is still pseudo-legal in the sense that discovered checks on the king (from moving the king away from a pin line) may not be fully handled by `is_square_attacked` alone."