Good — the analysis confirms the black queen on d8 has 0 legal moves in that position. Now I have all the information needed to produce the revised plan.

## 1. Deliverables

| # | Action | File | Purpose |
|---|--------|------|---------|
| 1 | **Create** | `crates/movegen/src/queens.rs` | Queen move generation via `generate_queen_moves(pos, moves)` using bishop+rook attack union; combined sliding piece interface via `generate_sliding_moves(pos, moves)`; unit tests for three specified positions |
| 2 | **Modify** | `crates/movegen/src/lib.rs` | Add `pub mod queens;` declaration; add re-exports for `generate_queen_moves`, `generate_sliding_moves`, `generate_bishop_moves`, and `generate_rook_moves` |

## 2. Dependencies

No new external dependencies. The `movegen` crate already depends on:
- `chess-types` (path `../types`) — provides `Move`, `MoveFlag`, `Piece`, `PieceKind`, `Square`, `Bitboard`
- `chess-board` (path `../board`) — provides `Position`

The task depends on T01 (`magic.rs` with `bishop_attacks`/`rook_attacks`) and T02 (`bishops.rs` with `generate_bishop_moves`, `rooks.rs` with `generate_rook_moves`), both already completed.

## 3. Implementation Details

### 3.1 `crates/movegen/src/queens.rs`

**Purpose**: Queen move generation and combined sliding piece interface.

**Imports**:
```rust
use chess_board::Position;
use chess_types::{Move, MoveFlag, Piece, PieceKind, Square};
```

**Public functions**:

#### `generate_queen_moves(pos: &Position, moves: &mut Vec<Move>)`

Logic:
1. Get `us = pos.side_to_move()`.
2. Get the queen bitboard: `pos.piece_bitboard(Piece::new(us, PieceKind::Queen))`.
3. Get `friendly = pos.occupied_by(us)`, `enemies = pos.occupied_by(us.opposite())`, `occupancy = pos.occupied()`.
4. Iterate over each set bit in the queen bitboard (each queen square):
   - Construct `Square` from the bit index via `Square::new(from_idx as u8).expect("valid square")` (follows the established convention in sibling files `bishops.rs` and `rooks.rs`).
   - Compute attacks as `bishop_attacks(from, occupancy) | rook_attacks(from, occupancy)` — this is the union of bishop and rook attacks for the queen's square.
   - Compute `targets = attacks & !friendly` to exclude friendly-occupied squares.
   - Split into `captures = targets & enemies` and `quiets = targets & !enemies`.
   - For each capture target, push `Move::new(from, to, MoveFlag::CAPTURE)`.
   - For each quiet target, push `Move::new(from, to, MoveFlag::QUIET)`.

This follows the exact same pattern as `generate_bishop_moves` and `generate_rook_moves`.

#### `generate_sliding_moves(pos: &Position, moves: &mut Vec<Move>)`

Logic:
1. Call `super::bishops::generate_bishop_moves(pos, moves)`.
2. Call `super::rooks::generate_rook_moves(pos, moves)`.
3. Call `generate_queen_moves(pos, moves)`.

This is a convenience aggregator that produces all sliding piece moves in one call.

**Unit tests** (`#[cfg(test)] mod tests`):

Five tests for queen generation and the combined interface:

1. **Starting position** (`rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1`): White queen on d1 is completely blocked by pawns on c2, d2, e2. Expected: 0 queen moves.

2. **Midgame open board** (`r1bqk2r/ppppbppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQ1RK1 b kq - 5 4`): Black to move. Black queen on d8 is fully surrounded by own pieces: c8 (bishop), e8 (king), d7 (pawn), c7 (pawn), e7 (bishop). All eight ray directions are blocked on the first adjacent square. Expected: 0 queen moves.

3. **Sliding pieces blocked by pawns** (`4k3/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQ - 0 1`): White queen on d1 is blocked by own pawns on c2, d2, e2. Expected: 0 queen moves.

4. **Queen on empty board** (`4k3/8/8/8/3Q4/8/8/4K3 w - - 0 1`): Queen on d4 with no blocking pieces. Expected: 27 moves (13 diagonal + 14 orthogonal), all with `MoveFlag::QUIET`.

5. **Combined sliding moves** (`4k3/8/8/8/3Q4/8/8/4K3 w - - 0 1`): `generate_sliding_moves` produces 27 moves — only the queen is on the board (no bishops or rooks), so the combined function produces the same 27 moves as `generate_queen_moves` alone.

### 3.2 `crates/movegen/src/lib.rs` modifications

Current state:
```rust
pub mod bishops;
pub mod king;
pub mod knights;
pub mod magic;
pub mod pawns;
pub mod rooks;

pub use bishops::generate_bishop_moves;
pub use king::generate_king_moves;
pub use knights::generate_knight_moves;
pub use pawns::generate_pawn_moves;
pub use rooks::generate_rook_moves;
```

Changes:
- Add `pub mod queens;` (alphabetical order, between `pawns` and `rooks`).
- Add `pub use queens::generate_queen_moves;` and `pub use queens::generate_sliding_moves;` re-exports.
- The re-exports for `generate_bishop_moves` and `generate_rook_moves` already exist in `lib.rs`, so no additional changes needed for those.

## 4. API Contracts

### `generate_queen_moves`
```rust
pub fn generate_queen_moves(pos: &Position, moves: &mut Vec<Move>)
```
- **Input**: `pos` — a valid `Position`; `moves` — a mutable `Vec<Move>` to append to.
- **Output**: Appends pseudo-legal queen moves to `moves`. Each move has `MoveFlag::CAPTURE` if the target square is occupied by an enemy piece, or `MoveFlag::QUIET` otherwise. No moves to squares occupied by friendly pieces are generated.
- **Example**: Position `4k3/8/8/8/3Q4/8/8/4K3 w - - 0 1` — queen on d4, empty board. Generates 27 moves (13 diagonal + 14 orthogonal), all with `MoveFlag::QUIET`.

### `generate_sliding_moves`
```rust
pub fn generate_sliding_moves(pos: &Position, moves: &mut Vec<Move>)
```
- **Input**: Same as above.
- **Output**: Appends all pseudo-legal bishop, rook, and queen moves to `moves`. Equivalent to calling `generate_bishop_moves`, `generate_rook_moves`, and `generate_queen_moves` sequentially.
- **Example**: Same position as above — generates 27 moves (only the queen, no bishops or rooks on board).

## 5. Test Plan

All tests are colocated in `crates/movegen/src/queens.rs` under `#[cfg(test)] mod tests`.

### Test 1: `test_queen_moves_starting_position`
- **Setup**: `Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")`
- **Action**: Call `generate_queen_moves(&pos, &mut moves)`
- **Assert**: `moves.len() == 0` — queen on d1 is fully blocked by pawns on c2, d2, e2.

### Test 2: `test_queen_moves_midgame`
- **Setup**: `Position::from_fen("r1bqk2r/ppppbppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQ1RK1 b kq - 5 4")`
- **Action**: Call `generate_queen_moves(&pos, &mut moves)`
- **Assert**: `moves.len() == 0` — black queen on d8 is fully surrounded by own pieces on all adjacent squares in every ray direction (c8 bishop to the west, e8 king to the east, d7 pawn to the south, c7 pawn to the southwest, e7 bishop to the southeast; north/northeast/northwest rays are off-board from rank 8).

### Test 3: `test_queen_moves_blocked_by_pawns`
- **Setup**: `Position::from_fen("4k3/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQ - 0 1")`
- **Action**: Call `generate_queen_moves(&pos, &mut moves)`
- **Assert**: `moves.len() == 0` — white queen on d1 is fully blocked by own pawns on c2, d2, e2.

### Test 4: `test_queen_moves_empty_board`
- **Setup**: `Position::from_fen("4k3/8/8/8/3Q4/8/8/4K3 w - - 0 1")`
- **Action**: Call `generate_queen_moves(&pos, &mut moves)`
- **Assert**: `moves.len() == 27` — 13 diagonal + 14 orthogonal. All moves have `MoveFlag::QUIET`.

### Test 5: `test_generate_sliding_moves_combined`
- **Setup**: `Position::from_fen("4k3/8/8/8/3Q4/8/8/4K3 w - - 0 1")`
- **Action**: Call `generate_sliding_moves(&pos, &mut moves)`
- **Assert**: `moves.len() == 27` — only the queen is on the board (no bishops or rooks), so the combined function should produce the same 27 moves.

Note: Tests 1, 2, and 3 correspond to the three FEN positions explicitly required by the task spec. Tests 4 and 5 verify basic correctness of the queen-as-bishop+rook union and the combined interface.

## 6. Implementation Order

1. **Create `crates/movegen/src/queens.rs`**:
   - Write the `generate_queen_moves` function following the pattern from `bishops.rs`/`rooks.rs`.
   - Write the `generate_sliding_moves` function calling all three generators.
   - Write all unit tests.

2. **Modify `crates/movegen/src/lib.rs`**:
   - Add `pub mod queens;` in alphabetical order.
   - Add `pub use queens::generate_queen_moves;` and `pub use queens::generate_sliding_moves;` re-exports.

3. **Verify**: Run `cargo test -p movegen`, `cargo clippy -p movegen -- -D warnings`, `cargo fmt --check -p movegen`.

## 7. Verification Commands

```bash
cargo fmt --check -p movegen
cargo clippy -p movegen -- -D warnings
cargo test -p movegen
```

## 8. Design Deviations

None.