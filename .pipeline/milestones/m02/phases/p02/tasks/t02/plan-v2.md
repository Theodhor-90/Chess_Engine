Now I have all the context. The feedback is simple: the only issue is that line 1 of the plan contains LLM preamble text that needs to be removed. The plan content itself is sound. I just need to output the same plan without the preamble line.

## 1. Deliverables

| # | Action | File | Purpose |
|---|--------|------|---------|
| 1 | **Create** | `crates/movegen/src/bishops.rs` | Bishop move generation: `generate_bishop_moves(pos: &Position, moves: &mut Vec<Move>)` using magic bitboard lookup, with unit tests |
| 2 | **Create** | `crates/movegen/src/rooks.rs` | Rook move generation: `generate_rook_moves(pos: &Position, moves: &mut Vec<Move>)` using magic bitboard lookup, with unit tests |
| 3 | **Modify** | `crates/movegen/src/lib.rs` | Add `pub mod bishops;` and `pub mod rooks;` declarations; add `pub use bishops::generate_bishop_moves;` and `pub use rooks::generate_rook_moves;` re-exports |

## 2. Dependencies

- **T01 complete** — `magic::bishop_attacks(Square, Bitboard) -> Bitboard` and `magic::rook_attacks(Square, Bitboard) -> Bitboard` are implemented and tested in `crates/movegen/src/magic.rs`.
- **Phase P01 complete** — the `movegen` crate structure, module conventions, and function signature pattern `fn(pos: &Position, moves: &mut Vec<Move>)` are established.
- **Milestone M01 complete** — `chess-types` crate provides `Bitboard`, `Square`, `Move`, `MoveFlag`, `Piece`, `PieceKind`, `Color`; `chess-board` crate provides `Position` with `piece_bitboard()`, `occupied_by()`, `occupied()`, `side_to_move()`.
- **No new crate dependencies** — only uses existing `chess-types` and `chess-board` from `Cargo.toml`.

## 3. Implementation Details

### 3.1 `crates/movegen/src/bishops.rs`

**Purpose**: Generate all pseudo-legal bishop moves for the side to move.

**Exports**: `pub fn generate_bishop_moves(pos: &Position, moves: &mut Vec<Move>)`

**Key logic**:
1. Get the side to move via `pos.side_to_move()`.
2. Get all bishops for the side: `pos.piece_bitboard(Piece::new(us, PieceKind::Bishop))`.
3. Get friendly occupancy: `pos.occupied_by(us)`.
4. Get enemy occupancy: `pos.occupied_by(us.opposite())`.
5. Get total occupancy: `pos.occupied()`.
6. Iterate over each bishop square (`for from_idx in bishops`):
   - Compute `from = Square::new(from_idx as u8).expect("valid square")`.
   - Call `super::magic::bishop_attacks(from, occupancy)` to get the attack bitboard.
   - Compute `targets = attacks & !friendly` (exclude squares with own pieces).
   - Split `targets` into `captures = targets & enemies` and `quiets = targets & !enemies`.
   - Iterate `captures`, create `Move::new(from, to, MoveFlag::CAPTURE)` for each.
   - Iterate `quiets`, create `Move::new(from, to, MoveFlag::QUIET)` for each.

**Convention**: Follows the exact pattern from `knights.rs` — same import structure, same variable naming, same loop-and-push approach with separate capture/quiet iteration.

### 3.2 `crates/movegen/src/rooks.rs`

**Purpose**: Generate all pseudo-legal rook moves for the side to move.

**Exports**: `pub fn generate_rook_moves(pos: &Position, moves: &mut Vec<Move>)`

**Key logic**: Identical structure to `bishops.rs`, but:
1. Get all rooks: `pos.piece_bitboard(Piece::new(us, PieceKind::Rook))`.
2. Call `super::magic::rook_attacks(from, occupancy)` instead.

### 3.3 `crates/movegen/src/lib.rs` (modification)

**Current state**:
```rust
pub mod king;
pub mod knights;
pub mod magic;
pub mod pawns;

pub use king::generate_king_moves;
pub use knights::generate_knight_moves;
pub use pawns::generate_pawn_moves;
```

**Changes**: Add two module declarations and two re-exports:
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

Module declarations and re-exports are kept in alphabetical order, matching existing convention.

## 4. API Contracts

### `generate_bishop_moves`

```rust
pub fn generate_bishop_moves(pos: &Position, moves: &mut Vec<Move>)
```

- **Input**: A `&Position` representing the current board state; a mutable `Vec<Move>` to append moves to.
- **Output**: Appends all pseudo-legal bishop moves for `pos.side_to_move()` to `moves`. Each move has `MoveFlag::CAPTURE` if the destination contains an enemy piece, or `MoveFlag::QUIET` otherwise. Does not include moves to squares occupied by friendly pieces.
- **Example**: Position `"4k3/8/8/8/3B4/8/8/4K3 w - - 0 1"` (white bishop on d4, empty board) → 13 quiet moves along both diagonals.

### `generate_rook_moves`

```rust
pub fn generate_rook_moves(pos: &Position, moves: &mut Vec<Move>)
```

- **Input**: Same as above.
- **Output**: Appends all pseudo-legal rook moves for `pos.side_to_move()` to `moves`. Each move has `MoveFlag::CAPTURE` if the destination contains an enemy piece, or `MoveFlag::QUIET` otherwise.
- **Example**: Position `"4k3/8/8/8/3R4/8/8/4K3 w - - 0 1"` (white rook on d4, empty board) → 14 moves (7 on file + 7 on rank).

## 5. Test Plan

Tests are colocated in `#[cfg(test)] mod tests` within each source file, following project convention.

### 5.1 `bishops.rs` tests

All tests use a `has_move(moves, from, to, flags) -> bool` helper (same pattern as `knights.rs`).

| Test | Setup | Assertion |
|------|-------|-----------|
| `test_bishop_moves_empty_board` | FEN `"4k3/8/8/8/3B4/8/8/4K3 w - - 0 1"` — bishop on d4, near-empty board | 13 moves generated (all diagonals unrestricted except e1 king blocks one square). Verify all moves have `MoveFlag::QUIET`. Actually — king is on e1, not on a diagonal from d4, so 13 quiet moves (full diag reach from d4). |
| `test_bishop_excludes_friendly` | FEN `"4k3/8/8/2N5/3B4/8/8/4K3 w - - 0 1"` — bishop on d4, friendly knight on c5 | The move d4→c5 is NOT generated (neither capture nor quiet). Move d4→b6 is NOT generated (blocked by c5). |
| `test_bishop_captures` | FEN `"4k3/8/8/2n5/3B4/8/8/4K3 w - - 0 1"` — bishop on d4, enemy knight on c5 | Move d4→c5 is generated with `MoveFlag::CAPTURE`. Move d4→b6 is NOT generated (blocked behind capture). |
| `test_black_bishop_moves` | FEN `"4k3/8/8/8/3b4/8/8/4K3 b - - 0 1"` — black bishop on d4, black to move | 13 moves generated. |

### 5.2 `rooks.rs` tests

| Test | Setup | Assertion |
|------|-------|-----------|
| `test_rook_moves_empty_board` | FEN `"4k3/8/8/8/3R4/8/8/4K3 w - - 0 1"` — rook on d4, near-empty board | 14 moves generated (7 file, 7 rank). All `MoveFlag::QUIET`. Note: king on e1 is not on same file/rank as d4, so no obstruction. |
| `test_rook_excludes_friendly` | FEN `"4k3/8/8/8/3RN3/8/8/4K3 w - - 0 1"` — rook on d4, friendly knight on e4 | Move d4→e4 is NOT generated. Moves d4→f4, d4→g4, d4→h4 are NOT generated (blocked). |
| `test_rook_captures` | FEN `"4k3/8/8/8/3Rn3/8/8/4K3 w - - 0 1"` — rook on d4, enemy knight on e4 | Move d4→e4 is generated with `MoveFlag::CAPTURE`. Moves beyond e4 (f4, g4, h4) are NOT generated. |
| `test_black_rook_moves` | FEN `"4k3/8/8/8/3r4/8/8/4K3 b - - 0 1"` — black rook on d4, black to move | 14 moves generated. |

## 6. Implementation Order

1. **Create `crates/movegen/src/bishops.rs`** — implement `generate_bishop_moves` and its unit tests.
2. **Create `crates/movegen/src/rooks.rs`** — implement `generate_rook_moves` and its unit tests.
3. **Modify `crates/movegen/src/lib.rs`** — add `pub mod bishops;`, `pub mod rooks;` declarations and `pub use` re-exports.

Steps 1 and 2 are independent of each other but both depend on `magic.rs` (already complete from T01). Step 3 must come after both files exist.

## 7. Verification Commands

```bash
# Run all movegen tests (includes bishops, rooks, and existing modules)
cargo test -p chess-movegen

# Verify clippy passes with no warnings
cargo clippy -p chess-movegen -- -D warnings

# Verify formatting
cargo fmt -p chess-movegen --check
```

## 8. Design Deviations

None.