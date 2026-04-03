`Square` is a newtype struct `Square(u8)`, and the constants use `Square(n)` directly which is only available within the `chess_types` crate. From outside (in movegen), I can't construct with `Square(n)` — I must use `Square::new()` which returns `Option`. Since `Square::new()` is not `const`, I cannot build a `const` knight attack table that uses `Square`-based indexing at compile time.

However, the attack table is `[Bitboard; 64]` indexed by `square.index()` as a `usize`, so the table itself only stores `Bitboard` values. The computation just needs `u64` arithmetic. Let me verify I can build the table as `const` using pure `u64`/`Bitboard` arithmetic.

Since `Bitboard::new` is `const fn` and the knight offset computation only needs integer arithmetic, I can build the entire `[Bitboard; 64]` table as a `const` — no `Square` construction needed during table building.

Now I have everything needed to write the plan.

# Task T02 — Knight Move Generation: Implementation Plan

## 1. Deliverables

| # | File | Purpose |
|---|------|---------|
| 1 | `crates/movegen/src/knights.rs` | Knight attack table (`[Bitboard; 64]`) and `generate_knight_moves` function with colocated unit tests |
| 2 | `crates/movegen/src/lib.rs` | Updated to declare the `knights` module and re-export `generate_knight_moves` |

## 2. Dependencies

No new dependencies. The `movegen` crate already declares dependencies on `chess-types` and `chess-board` in its `Cargo.toml` (established by T01). The required types are:

- From `chess-types`: `Bitboard`, `Square`, `Color`, `Move`, `MoveFlag`, `Piece`, `PieceKind`
- From `chess-board`: `Position`

## 3. Implementation Details

### 3.1 `crates/movegen/src/knights.rs`

**Knight Attack Table**

A `const KNIGHT_ATTACKS: [Bitboard; 64]` array, computed at compile time. For each square index `0..64`, the entry is the bitboard of all squares reachable by a knight on that square.

Computation logic (executed at compile time via `const fn`):
1. Extract file (`sq % 8`) and rank (`sq / 8`) from the square index
2. For each of the 8 knight offsets `(±1, ±2)` and `(±2, ±1)`:
   - Compute target file and rank
   - If both are in bounds (`0..8`), set the bit at `target_rank * 8 + target_file`
3. Store the resulting `u64` as a `Bitboard`

The 8 knight offsets (as `(file_delta, rank_delta)`) are:
`(-2, -1), (-2, +1), (-1, -2), (-1, +2), (+1, -2), (+1, +2), (+2, -1), (+2, +1)`

The table is built by a `const fn init_knight_attacks() -> [Bitboard; 64]` helper that loops over all 64 squares and computes the attack bitboard using integer arithmetic only (no `Square::new` needed since `Bitboard::new` is `const fn`).

**`generate_knight_moves` Function**

```rust
pub fn generate_knight_moves(pos: &Position, moves: &mut Vec<Move>)
```

Logic:
1. Get the side to move: `let us = pos.side_to_move()`
2. Get friendly knight bitboard: `pos.piece_bitboard(Piece::new(us, PieceKind::Knight))`
3. Get friendly occupancy: `pos.occupied_by(us)`
4. Get enemy occupancy: `pos.occupied_by(us.opposite())`
5. Iterate over each knight square (via `for from_idx in knights_bb`):
   - Look up `KNIGHT_ATTACKS[from_idx as usize]`
   - Mask out friendly pieces: `targets = attacks & !friendly`
   - For captures: `captures = targets & enemies` — iterate, emit `Move::new(from, to, MoveFlag::CAPTURE)`
   - For quiets: `quiets = targets & !enemies` (equivalently `targets & !(friendly | enemies)`, but since friendly is already removed, `targets & !enemies`) — iterate, emit `Move::new(from, to, MoveFlag::QUIET)`
6. Construct `Square` values using `Square::new(idx as u8).expect("valid square")` (safe because indices come from valid bitboard iteration over the 0..63 range)

**Unit Tests** (`#[cfg(test)] mod tests`)

Tests follow the conventions established in `pawns.rs` — use `has_move` helper, `Position::from_fen`, and targeted assertions.

### 3.2 `crates/movegen/src/lib.rs`

Add `pub mod knights;` declaration and `pub use knights::generate_knight_moves;` re-export, following the existing pattern for `pawns`.

## 4. API Contracts

### Public Function

```rust
pub fn generate_knight_moves(pos: &Position, moves: &mut Vec<Move>)
```

**Input:**
- `pos`: A `&Position` representing the current board state
- `moves`: A `&mut Vec<Move>` to append generated moves to

**Output:** No return value. Moves are appended to `moves`.

**Behavior:**
- Appends all pseudo-legal knight moves for `pos.side_to_move()` to `moves`
- Each move landing on an opponent-occupied square has flag `MoveFlag::CAPTURE`
- Each move landing on an empty square has flag `MoveFlag::QUIET`
- Squares occupied by friendly pieces are excluded

**Example:**
```rust
// Knight on D4, no other pieces except kings
let pos = Position::from_fen("4k3/8/8/8/3N4/8/8/4K3 w - - 0 1").unwrap();
let mut moves = Vec::new();
generate_knight_moves(&pos, &mut moves);
assert_eq!(moves.len(), 8); // C2, E2, B3, F3, B5, F5, C6, E6
```

### Internal Constant

```rust
const KNIGHT_ATTACKS: [Bitboard; 64]
```

Not publicly exported — internal to the `knights` module.

## 5. Test Plan

All tests are colocated in `crates/movegen/src/knights.rs` within a `#[cfg(test)] mod tests` block.

| # | Test Name | Setup (FEN) | Verification |
|---|-----------|------------|--------------|
| 1 | `test_knight_attacks_corner_a1` | N/A (table check) | `KNIGHT_ATTACKS[Square::A1.index() as usize].pop_count() == 2` — targets are B3 and C2 |
| 2 | `test_knight_attacks_edge_a4` | N/A (table check) | `KNIGHT_ATTACKS[Square::A4.index() as usize].pop_count() == 4` — targets are B2, C3, C5, B6 |
| 3 | `test_knight_attacks_center_d4` | N/A (table check) | `KNIGHT_ATTACKS[Square::D4.index() as usize].pop_count() == 8` — all 8 targets present |
| 4 | `test_knight_moves_center` | `4k3/8/8/8/3N4/8/8/4K3 w - - 0 1` | 8 moves generated, all `MoveFlag::QUIET` |
| 5 | `test_knight_excludes_friendly` | `4k3/8/2N5/8/3N4/8/8/4K3 w - - 0 1` | Knight on D4 does not generate a move to C6 (friendly knight there) |
| 6 | `test_knight_captures` | `4k3/8/2n5/8/3N4/8/8/4K3 w - - 0 1` | D4→C6 uses `MoveFlag::CAPTURE`; other 7 targets use `MoveFlag::QUIET` |
| 7 | `test_black_knight_moves` | `4k3/8/8/8/3n4/8/8/4K3 b - - 0 1` | 8 moves generated for black knight on D4 |

Tests map to verification criteria:
- Criteria 2 (correct attack bitboards for all 64 squares): tests 1–3 verify corner, edge, center
- Criteria 3 (friendly pieces excluded): test 5
- Criteria 5 (captures use `MoveFlag::CAPTURE`): test 6
- Criteria 6 (non-captures use `MoveFlag::QUIET`): tests 4, 6
- Criteria 1 (`cargo test -p movegen` passes): all tests together

## 6. Implementation Order

1. **Create `crates/movegen/src/knights.rs`** — implement `init_knight_attacks` const fn and `KNIGHT_ATTACKS` const array
2. **Implement `generate_knight_moves`** in the same file — the move generation function using the attack table
3. **Add unit tests** in the `#[cfg(test)] mod tests` block within `knights.rs`
4. **Update `crates/movegen/src/lib.rs`** — add `pub mod knights;` and `pub use knights::generate_knight_moves;`
5. **Run verification commands** to confirm correctness

## 7. Verification Commands

```bash
# Run all movegen tests (includes both pawn and knight tests)
cargo test -p chess-movegen

# Run only knight-specific tests
cargo test -p chess-movegen knights

# Check formatting
cargo fmt --check

# Check lints
cargo clippy -p chess-movegen -- -D warnings
```

## 8. Design Deviations

None.