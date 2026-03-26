Now I have everything I need. Here is the complete implementation plan:

## 1. Deliverables

- `crates/board/src/position.rs` — modified to add:
  - `UndoInfo` struct (captured piece, prior castling rights, prior en passant square, prior halfmove clock, prior Zobrist hash)
  - `CASTLING_RIGHTS_MASK` const lookup table (64 entries indexed by square index)
  - `pub(crate)` setter/mutation methods on `Position` for modifying private fields
  - `Position::make_move(&mut self, mv: Move) -> UndoInfo` method with incremental Zobrist hash update
  - Unit tests for each move type: quiet, capture, double pawn push, castling (O-O and O-O-O for both colors), en passant, promotion (all four piece types), promotion-capture
- `crates/board/src/lib.rs` — modified to export `UndoInfo` as a public type

## 2. Dependencies

- Phase P01 (Primitive Types) — complete. Provides `Bitboard`, `Square`, `File`, `Rank`, `Color`, `PieceKind`, `Piece`, `Move`, `MoveFlag` from `chess-types`
- Phase P02 (Position State) — complete. Provides `Position`, `CastlingRights`, FEN parsing/serialization, Zobrist hashing infrastructure from `chess-board`
- No new crate dependencies required

## 3. Implementation Details

### `UndoInfo` struct (in `position.rs`)

```rust
/// State saved by `make_move` that is needed by `unmake_move` to restore the position.
#[derive(Clone, Copy, Debug)]
pub struct UndoInfo {
    pub captured: Option<Piece>,
    pub castling_rights: CastlingRights,
    pub en_passant: Option<Square>,
    pub halfmove_clock: u8,
    pub hash: u64,
}
```

All fields are `pub` since `UndoInfo` is a plain data carrier consumed by `unmake_move`. The struct is `Copy` for zero-cost passing.

### `CASTLING_RIGHTS_MASK` const table (in `position.rs`)

A `const [u8; 64]` table indexed by `square.index()`. Each entry is a bitmask of castling rights to **preserve** (AND mask). When a piece moves from or to a square, `castling_rights.0 &= CASTLING_RIGHTS_MASK[sq]`.

Key entries:
- `A1` (index 0): `0b1101` — clears `WHITE_QUEENSIDE` (bit 1)
- `E1` (index 4): `0b1100` — clears both white castling rights (bits 0 and 1)
- `H1` (index 7): `0b1110` — clears `WHITE_KINGSIDE` (bit 0)
- `A8` (index 56): `0b0111` — clears `BLACK_QUEENSIDE` (bit 3)
- `E8` (index 60): `0b0011` — clears both black castling rights (bits 2 and 3)
- `H8` (index 63): `0b1011` — clears `BLACK_KINGSIDE` (bit 2)
- All other squares: `0b1111` (no change)

### `pub(crate)` mutation methods on `Position`

These keep the private-field access pattern established in P02:

```rust
pub(crate) fn piece_bb_mut(&mut self, piece: Piece) -> &mut Bitboard
pub(crate) fn occupied_by_mut(&mut self, color: Color) -> &mut Bitboard
pub(crate) fn occupied_mut(&mut self) -> &mut Bitboard
pub(crate) fn set_side_to_move(&mut self, color: Color)
pub(crate) fn set_castling_rights(&mut self, rights: CastlingRights)
pub(crate) fn set_en_passant(&mut self, sq: Option<Square>)
pub(crate) fn set_halfmove_clock(&mut self, clock: u8)
pub(crate) fn set_fullmove_counter(&mut self, counter: u16)
```

Additionally, `CastlingRights` needs a method to AND-mask its inner value. Add to `CastlingRights`:

```rust
/// Clears rights not present in the mask (AND operation on the raw bitfield).
pub fn mask(&mut self, mask: u8) {
    self.0 &= mask;
}
```

### `Position::make_move` method

Signature: `pub fn make_move(&mut self, mv: Move) -> UndoInfo`

**Algorithm:**

1. **Save undo state**: Capture current `castling_rights`, `en_passant`, `halfmove_clock`, and `hash` into `UndoInfo`. The `captured` field is set during move execution.

2. **Extract move info**: `from = mv.from_sq()`, `to = mv.to_sq()`, `flags = mv.flags()`. Determine the moving piece via `self.piece_on(from)` (will be `Some` for valid moves). Determine the captured piece via `self.piece_on(to)` for regular captures.

3. **Hash out old en passant**: If `self.en_passant` is `Some`, XOR out its file key.

4. **Hash out old castling rights**: XOR out `castling_key(self.castling_rights)`.

5. **Handle by move type** (branching on `flags`):

   **Quiet move** (`QUIET`):
   - Remove moving piece from `from`, place on `to` (piece bitboard, occupancy bitboards)
   - Hash: XOR out piece@from, XOR in piece@to

   **Capture** (`CAPTURE`):
   - Determine captured piece from `self.piece_on(to)`
   - Remove captured piece from `to` (its piece bitboard, opponent occupancy)
   - Remove moving piece from `from`, place on `to`
   - Hash: XOR out captured@to, XOR out mover@from, XOR in mover@to

   **Double pawn push** (`DOUBLE_PAWN_PUSH`):
   - Move pawn from `from` to `to`
   - Set en passant square: for White, square behind `to` (index - 8); for Black, square ahead of `to` (index + 8)
   - Hash: XOR out pawn@from, XOR in pawn@to, XOR in new EP file key

   **King castle** (`KING_CASTLE`):
   - Move king: `from` → `to` (e.g., e1→g1)
   - Move rook: h-file start square → f-file square. White: H1→F1. Black: H8→F8.
   - Hash: XOR out/in for both king and rook moves

   **Queen castle** (`QUEEN_CASTLE`):
   - Move king: `from` → `to` (e.g., e1→c1)
   - Move rook: a-file start square → d-file square. White: A1→D1. Black: A8→D8.
   - Hash: XOR out/in for both king and rook moves

   **En passant** (`EN_PASSANT`):
   - Determine captured pawn square: for White moving, `to.index() - 8`; for Black moving, `to.index() + 8`
   - Remove captured pawn from that square
   - Move capturing pawn from `from` to `to`
   - Hash: XOR out captured_pawn@capture_sq, XOR out mover@from, XOR in mover@to

   **Promotion** (`KNIGHT/BISHOP/ROOK/QUEEN_PROMOTION`):
   - Remove pawn from `from`
   - Place promoted piece (determined by `mv.promotion_piece()`) on `to`
   - Hash: XOR out pawn@from, XOR in promoted_piece@to

   **Promotion capture** (`*_PROMOTION_CAPTURE`):
   - Determine captured piece from `self.piece_on(to)`
   - Remove captured piece from `to`
   - Remove pawn from `from`
   - Place promoted piece on `to`
   - Hash: XOR out captured@to, XOR out pawn@from, XOR in promoted_piece@to

6. **Update castling rights**: Apply the mask table for both `from` and `to` squares: `self.castling_rights.mask(CASTLING_RIGHTS_MASK[from.index() as usize])` and same for `to`. This handles king moves, rook moves, and rook captures in a single branchless operation.

7. **Hash in new castling rights**: XOR in `castling_key(self.castling_rights)`.

8. **Update halfmove clock**: Reset to 0 if moving piece is a pawn or if it's a capture; otherwise increment by 1.

9. **Update fullmove counter**: Increment by 1 if side to move was Black.

10. **Toggle side to move**: Flip to opposite color. Hash: XOR in `side_to_move_key()`.

11. **Clear en passant** (if not already set by double pawn push): Set `self.en_passant = None`.

12. **Update total occupancy**: Recompute `self.occupied = self.occupied_by[0] | self.occupied_by[1]`.

13. **Return `UndoInfo`** with saved state and the captured piece.

**Note on en passant handling in hash**: The en passant square is XOR'd out at step 3 (before the move) and only XOR'd in at step 5 (double pawn push case). For non-double-pawn-push moves, en passant is simply cleared without additional hash work (the XOR-out already removed it).

### `lib.rs` modification

Add `UndoInfo` to the existing re-export line:

```rust
pub use position::{CastlingRights, Position, UndoInfo};
```

## 4. API Contracts

```rust
use chess_board::{Position, UndoInfo};
use chess_types::{Move, MoveFlag, Square};

let mut pos = Position::startpos();
let mv = Move::new(Square::E2, Square::E4, MoveFlag::DOUBLE_PAWN_PUSH);
let undo: UndoInfo = pos.make_move(mv);
// pos is now updated — pawn on e4, en passant on e3, Black to move
```

N/A for REST/HTTP.

## 5. Test Plan

All tests are colocated in `crates/board/src/position.rs` within `#[cfg(test)] mod tests`.

| Test | What it verifies |
|------|-----------------|
| `make_move_quiet` | A quiet knight move (e.g., Ng1-f3 from startpos) updates piece bitboards, occupancy, side to move, increments halfmove clock, and does not set en passant. Zobrist hash matches full recomputation. |
| `make_move_capture` | A capture removes the captured piece from its bitboard and opponent occupancy. `UndoInfo.captured` holds the captured piece. Halfmove clock resets. Zobrist hash matches recomputation. |
| `make_move_double_pawn_push` | A double pawn push (e.g., e2→e4) sets the en passant square to e3, resets halfmove clock. Zobrist hash includes EP file key. |
| `make_move_white_kingside_castle` | White O-O (e1→g1): king on g1, rook on f1 (was h1). Both white castling rights cleared. Zobrist hash matches. |
| `make_move_white_queenside_castle` | White O-O-O (e1→c1): king on c1, rook on d1 (was a1). Both white castling rights cleared. Zobrist hash matches. |
| `make_move_black_kingside_castle` | Black O-O (e8→g8): king on g8, rook on f8 (was h8). Both black castling rights cleared. Zobrist hash matches. |
| `make_move_black_queenside_castle` | Black O-O-O (e8→c8): king on c8, rook on d8 (was a8). Both black castling rights cleared. Zobrist hash matches. |
| `make_move_en_passant` | White pawn on e5 captures en passant on d6: black pawn removed from d5 (not d6), white pawn placed on d6. Zobrist hash matches. |
| `make_move_promotion_knight` | Pawn on e7→e8 with knight promotion: pawn removed, knight placed on e8. Zobrist hash matches. |
| `make_move_promotion_bishop` | Same for bishop promotion. |
| `make_move_promotion_rook` | Same for rook promotion. |
| `make_move_promotion_queen` | Same for queen promotion. |
| `make_move_promotion_capture` | Pawn on e7 captures d8 with queen promotion: captured piece removed, pawn removed, queen placed. `UndoInfo.captured` correct. Zobrist hash matches. |
| `make_move_fullmove_counter` | Fullmove counter increments after Black's move, not after White's. |
| `make_move_castling_rights_rook_capture` | When a rook is captured on its home square (e.g., a piece captures on h1), the corresponding castling right is removed. |

**Test setup pattern**: Each test constructs a `Position` from a specific FEN string tailored for that move type, calls `make_move`, then verifies:
- Piece bitboards reflect the expected post-move state (via `piece_on` / `piece_bitboard`)
- Occupancy bitboards are consistent
- Side to move toggled
- Castling rights updated correctly
- En passant square set/cleared correctly
- Halfmove clock reset/incremented correctly
- Fullmove counter updated correctly
- Zobrist hash matches `crate::zobrist::compute_hash(&pos)` (full recomputation)

## 6. Implementation Order

1. Add `CastlingRights::mask` method
2. Add `CASTLING_RIGHTS_MASK` const table
3. Add `UndoInfo` struct
4. Add `pub(crate)` setter/mutation methods to `Position`
5. Implement `Position::make_move`
6. Update `crates/board/src/lib.rs` to export `UndoInfo`
7. Write unit tests for all move types

## 7. Verification Commands

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --workspace
```

## 8. Design Deviations

None.