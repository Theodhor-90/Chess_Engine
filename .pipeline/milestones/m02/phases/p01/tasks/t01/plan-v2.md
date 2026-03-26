Now I have all the context needed. Let me produce the revised plan.

## 1. Deliverables

- `crates/movegen/Cargo.toml` — New crate manifest declaring the `chess-movegen` package with dependencies on `chess-types` and `chess-board`
- `crates/movegen/src/lib.rs` — Crate root: declares modules and re-exports the public pawn move generation function
- `crates/movegen/src/pawns.rs` — Pawn pseudo-legal move generation for both white and black, with colocated unit tests
- `Cargo.toml` (workspace root) — Updated `members` list to include `crates/movegen`

## 2. Dependencies

- `chess-types` (local path `../types`) — provides `Bitboard`, `Square`, `Move`, `MoveFlag`, `Color`, `Piece`, `PieceKind`, `File`, `Rank`
- `chess-board` (local path `../board`) — provides `Position` (accessors: `piece_bitboard`, `occupied`, `occupied_by`, `side_to_move`, `en_passant`, `from_fen`)
- No new external crates required

## 3. Implementation Details

### `crates/movegen/Cargo.toml`

```toml
[package]
name = "chess-movegen"
version = "0.1.0"
edition.workspace = true

[dependencies]
chess-types = { path = "../types" }
chess-board = { path = "../board" }
```

### `Cargo.toml` (workspace root)

Add `"crates/movegen"` to the `members` array:

```toml
members = ["crates/types", "crates/board", "crates/movegen"]
```

### `crates/movegen/src/lib.rs`

- Declares `pub mod pawns;`
- Re-exports the public function: `pub use pawns::generate_pawn_moves;`

### `crates/movegen/src/pawns.rs`

**Constants (file-private):**

- `NOT_A_FILE: Bitboard` — `Bitboard::new(0xFEFE_FEFE_FEFE_FEFE)` — masks out the A-file to prevent wrap-around on capture shifts
- `NOT_H_FILE: Bitboard` — `Bitboard::new(0x7F7F_7F7F_7F7F_7F7F)` — masks out the H-file to prevent wrap-around on capture shifts
- `RANK_3: Bitboard` — `Bitboard::new(0x0000_0000_00FF_0000)` — intermediate rank mask for white double push
- `RANK_6: Bitboard` — `Bitboard::new(0x0000_FF00_0000_0000)` — intermediate rank mask for black double push
- `RANK_8: Bitboard` — `Bitboard::new(0xFF00_0000_0000_0000)` — promotion rank for white
- `RANK_1: Bitboard` — `Bitboard::new(0x0000_0000_0000_00FF)` — promotion rank for black

No other rank constants are needed. The double-push approach filters through `RANK_3`/`RANK_6` (see below), so `RANK_4` and `RANK_5` are not required.

**Public function:**

```rust
pub fn generate_pawn_moves(pos: &Position, moves: &mut Vec<Move>)
```

Takes a `&Position` reference and appends all pseudo-legal pawn moves for `pos.side_to_move()` to the provided `Vec<Move>`.

**Internal logic — dispatches by color:**

The function reads `pos.side_to_move()` and calls an internal helper parameterized by direction constants. The core logic for each color:

1. **Retrieve bitboards**: `pawns = pos.piece_bitboard(Piece::new(us, PieceKind::Pawn))`, `empty = !pos.occupied()`, `enemies = pos.occupied_by(them)`

2. **Single push**:
   - White: `single = (pawns << 8) & empty`
   - Black: `single = (pawns >> 8) & empty`
   - Split into non-promotion (`single & !promo_rank`) and promotion (`single & promo_rank`)
   - Non-promotion: iterate target bits, compute `from = Square::new(to_idx - 8)` (white) or `Square::new(to_idx + 8)` (black), emit `Move::new(from, to, MoveFlag::QUIET)`
   - Promotion: iterate target bits, compute `from` as above, emit 4 moves with `KNIGHT_PROMOTION`, `BISHOP_PROMOTION`, `ROOK_PROMOTION`, `QUEEN_PROMOTION`

3. **Double push**:
   - White: `double = (((pawns << 8) & empty & RANK_3) << 8) & empty` — first shift finds pawns that successfully pushed from rank 2 to rank 3 (intermediate clear), second shift pushes to rank 4 (target clear)
   - Black: `double = (((pawns >> 8) & empty & RANK_6) >> 8) & empty` — first shift finds pawns that pushed from rank 7 to rank 6, second shift pushes to rank 5
   - Iterate target bits, compute `from = Square::new(to_idx - 16)` (white) or `Square::new(to_idx + 16)` (black), emit `Move::new(from, to, MoveFlag::DOUBLE_PAWN_PUSH)`

4. **Captures** (two directions per color):

   Using LERF mapping (a1 = 0), the shift values and required file masks are:

   | Color | Direction | Shift | File mask | Example |
   |-------|-----------|-------|-----------|---------|
   | White | Up-left | `<< 7` | `NOT_A_FILE` | d2(11) → c3(18) |
   | White | Up-right | `<< 9` | `NOT_H_FILE` | d2(11) → e3(20) |
   | Black | Down-right | `>> 7` | `NOT_H_FILE` | d7(51) → e6(44) |
   | Black | Down-left | `>> 9` | `NOT_A_FILE` | d7(51) → c6(42) |

   For each of the two capture directions:
   - Compute capture targets: `cap = (shifted_pawns) & enemies` using the shift and file mask from the table above
   - Split into non-promotion (`cap & !promo_rank`) and promotion captures (`cap & promo_rank`)
   - Non-promotion captures: iterate target bits, compute `from` by reversing the shift (subtract for white, add for black), emit `Move::new(from, to, MoveFlag::CAPTURE)`
   - Promotion captures: iterate target bits, emit 4 moves with `KNIGHT_PROMOTION_CAPTURE`, `BISHOP_PROMOTION_CAPTURE`, `ROOK_PROMOTION_CAPTURE`, `QUEEN_PROMOTION_CAPTURE`

5. **En passant**:
   - Check `pos.en_passant()`. If `Some(ep_sq)`:
     - Compute `ep_bb = Bitboard::new(1u64 << ep_sq.index())`
     - For each of the two capture directions (same shift/mask table as above), check if `(shifted_pawns) & ep_bb` is non-empty
     - If so, compute `from` by reversing the shift, emit `Move::new(from, ep_sq, MoveFlag::EN_PASSANT)`

**`#[cfg(test)] mod tests`** at the bottom of `pawns.rs`:

- Tests use `Position::from_fen(...)` to set up specific board states, call `generate_pawn_moves`, then assert the expected moves are present in the output.
- Helper: a function that checks whether a specific `Move` (by from, to, flags) is in the generated moves list.

**Test cases** (mapped directly to the task spec's verification criteria):

| Test | Setup (FEN) | What it verifies |
|------|------------|-----------------|
| `test_white_single_push` | Startpos | White pawns generate 8 single pushes (a2→a3 through h2→h3) with `QUIET` flag |
| `test_white_single_push_blocked` | Pawn on e2, opponent piece on e3 | No push generated for blocked pawn |
| `test_black_single_push` | FEN with black to move, pawns on rank 7 | Black pawns generate single pushes with `QUIET` flag |
| `test_white_double_push` | Startpos | White pawns on rank 2 generate 8 double pushes with `DOUBLE_PAWN_PUSH` flag |
| `test_white_double_push_blocked_intermediate` | Pawn on e2, piece on e3 | No double push when intermediate square blocked |
| `test_white_double_push_blocked_target` | Pawn on e2, e3 empty, piece on e4 | No double push when target square blocked |
| `test_black_double_push` | FEN with black to move, pawns on rank 7 | Black double pushes with `DOUBLE_PAWN_PUSH` flag |
| `test_white_captures` | White pawn on d4, black pieces on c5 and e5 | Two captures with `CAPTURE` flag |
| `test_black_captures` | Black pawn on e5, white pieces on d4 and f4, black to move | Two captures with `CAPTURE` flag |
| `test_no_capture_on_friendly` | White pawn on d4, white piece on e5 | No capture onto own piece |
| `test_capture_no_file_wrap` | White pawn on a4 with black piece on h5 | No wrap-around capture |
| `test_white_en_passant` | FEN with white pawn on e5, ep square d6 | En passant capture with `EN_PASSANT` flag |
| `test_black_en_passant` | FEN with black pawn on d4, ep square e3 | En passant from black side |
| `test_no_en_passant_when_unavailable` | Position with no ep square | No en passant moves generated |
| `test_white_promotion` | White pawn on e7, e8 empty | 4 non-capture promotion moves (`KNIGHT_PROMOTION`, `BISHOP_PROMOTION`, `ROOK_PROMOTION`, `QUEEN_PROMOTION`) |
| `test_black_promotion` | Black pawn on e2, e1 empty, black to move | 4 non-capture promotion moves |
| `test_white_promotion_capture` | White pawn on e7, black piece on d8 | 4 capture-promotion moves (`KNIGHT_PROMOTION_CAPTURE` through `QUEEN_PROMOTION_CAPTURE`) |
| `test_black_promotion_capture` | Black pawn on e2, white piece on d1, black to move | 4 capture-promotion moves |
| `test_no_moves_for_opponent_pawns` | Startpos (white to move) | No moves generated for black pawns |

## 4. API Contracts

```rust
use chess_board::Position;
use chess_types::Move;
use chess_movegen::generate_pawn_moves;

let pos = Position::startpos();
let mut moves = Vec::new();
generate_pawn_moves(&pos, &mut moves);
// moves now contains 16 pawn moves (8 single push + 8 double push)
```

N/A for REST/HTTP.

## 5. Test Plan

All tests are colocated in `crates/movegen/src/pawns.rs` within `#[cfg(test)] mod tests`.

| Test | What it verifies |
|------|-----------------|
| `test_white_single_push` | White pawns produce single push `QUIET` moves |
| `test_white_single_push_blocked` | No single push when target square occupied |
| `test_black_single_push` | Black pawns produce single push `QUIET` moves |
| `test_white_double_push` | White pawns on rank 2 produce `DOUBLE_PAWN_PUSH` moves |
| `test_white_double_push_blocked_intermediate` | No double push when intermediate square blocked |
| `test_white_double_push_blocked_target` | No double push when target square blocked |
| `test_black_double_push` | Black pawns on rank 7 produce `DOUBLE_PAWN_PUSH` moves |
| `test_white_captures` | Diagonal captures produce `CAPTURE` moves onto enemy squares |
| `test_black_captures` | Black diagonal captures produce `CAPTURE` moves |
| `test_no_capture_on_friendly` | No capture moves onto squares occupied by friendly pieces |
| `test_capture_no_file_wrap` | Captures do not wrap across file A/H boundary |
| `test_white_en_passant` | En passant capture with `EN_PASSANT` flag when ep square available |
| `test_black_en_passant` | Black en passant capture |
| `test_no_en_passant_when_unavailable` | No en passant moves when `pos.en_passant()` is `None` |
| `test_white_promotion` | Non-capture promotion generates 4 moves with correct flags |
| `test_black_promotion` | Black non-capture promotion generates 4 moves |
| `test_white_promotion_capture` | Capture-promotion generates 4 moves with correct capture-promotion flags |
| `test_black_promotion_capture` | Black capture-promotion generates 4 moves |
| `test_no_moves_for_opponent_pawns` | Only side-to-move pawns generate moves |

## 6. Implementation Order

1. `Cargo.toml` (workspace root) — add `crates/movegen` to members
2. `crates/movegen/Cargo.toml` — create crate manifest
3. `crates/movegen/src/lib.rs` — create crate root with module declaration and re-export
4. `crates/movegen/src/pawns.rs` — implement `generate_pawn_moves` with all move types (single push, double push, captures, en passant, promotions, capture-promotions) and all colocated unit tests

## 7. Verification Commands

```bash
cargo test -p chess-movegen
cargo clippy -p chess-movegen -- -D warnings
cargo fmt --check
```

## 8. Design Deviations

None.