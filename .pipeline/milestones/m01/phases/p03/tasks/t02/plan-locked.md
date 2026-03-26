Now I have the complete picture. Let me produce the full revised plan.

# T02 — `unmake_move` Implementation Plan

## 1. Deliverables

- `crates/board/src/position.rs` — modified to add:
  - `Position::unmake_move(&mut self, mv: Move, undo: UndoInfo)` method that restores the position to its exact prior state
  - Unit tests verifying round-trip identity (`make_move` followed by `unmake_move`) for every move type: quiet, capture, double pawn push, castling (O-O and O-O-O for both colors), en passant, promotion (all four piece types), promotion-capture
  - FEN round-trip tests confirming `pos.to_fen()` is identical before and after a `make_move`/`unmake_move` cycle

No new files are created. No modifications to `crates/board/src/lib.rs` are needed (no new public types are introduced).

## 2. Dependencies

- **Phase P01 (Primitive Types)** — complete. Provides `Bitboard`, `Square`, `File`, `Rank`, `Color`, `PieceKind`, `Piece`, `Move`, `MoveFlag` from `chess-types`
- **Phase P02 (Position State)** — complete. Provides `Position`, `CastlingRights`, FEN parsing/serialization, Zobrist hashing infrastructure from `chess-board`
- **Task T01 (`make_move`)** within this phase — complete. Provides `UndoInfo` struct, `CASTLING_RIGHTS_MASK` const table, `pub(crate)` setter/mutation methods on `Position`, and the `Position::make_move` method
- No new crate dependencies required

## 3. Implementation Details

### `Position::unmake_move` method

Signature: `pub fn unmake_move(&mut self, mv: Move, undo: UndoInfo)`

This method is the exact inverse of `make_move`. It restores every field of `Position` to its state before the corresponding `make_move` call. The method does **not** return a value — the caller already holds the `UndoInfo` that was returned by `make_move`.

**Key design choice — direct hash restoration**: Rather than incrementally reversing each Zobrist hash toggle (which would duplicate `make_move`'s hash logic in reverse and risk divergence), `unmake_move` restores the hash directly from `UndoInfo.hash`. This is simpler, faster (one assignment vs. multiple XORs), and guaranteed correct since `make_move` saved the pre-move hash.

**Algorithm (10 steps):**

1. **Toggle side to move back**: Call `self.set_side_to_move(self.side_to_move().opposite())`. This undoes the side-to-move flip that `make_move` performed as its last logical step. After this, `self.side_to_move()` returns the color that made the move being undone.

2. **Extract move info**: `from = mv.from_sq()`, `to = mv.to_sq()`, `flags = mv.flags()`. The moving piece is now on `to` (or for promotions, the promoted piece is on `to`).

3. **Handle by move type** (branching on `flags`):

   **Quiet move** (`QUIET`):
   - Determine moving piece via `self.piece_on(to)` (it's on the destination square now)
   - Remove piece from `to`: clear bit in piece bitboard and side's occupancy bitboard
   - Place piece on `from`: set bit in piece bitboard and side's occupancy bitboard

   **Capture** (`CAPTURE`):
   - Determine moving piece via `self.piece_on(to)`
   - Remove moving piece from `to`: clear bit in piece bitboard and side's occupancy bitboard
   - Place moving piece on `from`: set bit in piece bitboard and side's occupancy bitboard
   - Restore captured piece on `to`: read from `undo.captured` (guaranteed `Some` for captures); set bit in captured piece's bitboard and opponent's occupancy bitboard

   **Double pawn push** (`DOUBLE_PAWN_PUSH`):
   - Determine moving pawn via `self.piece_on(to)`
   - Remove pawn from `to`: clear bit in pawn bitboard and side's occupancy bitboard
   - Place pawn on `from`: set bit in pawn bitboard and side's occupancy bitboard
   - (En passant square restoration is handled in step 5 via `UndoInfo`)

   **King castle** (`KING_CASTLE`):
   - Determine the castling color from `self.side_to_move()` (already restored in step 1)
   - Move king back: remove from `to` (g1 or g8), place on `from` (e1 or e8). Update king's piece bitboard and side's occupancy.
   - Move rook back: For White, remove from F1, place on H1. For Black, remove from F8, place on H8. Update rook's piece bitboard and side's occupancy. Rook squares are computed as:
     - `rook_to` (where rook is now): `Square::new(5, rank)` (f-file) where `rank = from.rank()`
     - `rook_from` (where rook started): `Square::new(7, rank)` (h-file)

   **Queen castle** (`QUEEN_CASTLE`):
   - Move king back: remove from `to` (c1 or c8), place on `from` (e1 or e8). Update king's piece bitboard and side's occupancy.
   - Move rook back: For White, remove from D1, place on A1. For Black, remove from D8, place on A8. Update rook's piece bitboard and side's occupancy. Rook squares:
     - `rook_to` (where rook is now): `Square::new(3, rank)` (d-file) where `rank = from.rank()`
     - `rook_from` (where rook started): `Square::new(0, rank)` (a-file)

   **En passant** (`EN_PASSANT`):
   - Determine capturing pawn via `self.piece_on(to)` (the moving pawn is on the EP target square)
   - Remove capturing pawn from `to`: clear bit in pawn bitboard and side's occupancy
   - Place capturing pawn on `from`: set bit in pawn bitboard and side's occupancy
   - Restore captured pawn: the captured pawn's square is on the same rank as `from` and the same file as `to`. For White undoing: captured square = `to.index() - 8`. For Black undoing: captured square = `to.index() + 8`. Read captured piece from `undo.captured` (guaranteed `Some`). Set bit in captured pawn's bitboard and opponent's occupancy.

   **Promotion** (`KNIGHT/BISHOP/ROOK/QUEEN_PROMOTION`):
   - Determine the promoted piece type from `mv.promotion_piece()` (returns the `PieceKind` that the pawn was promoted to)
   - Remove promoted piece from `to`: clear bit in the promoted piece's bitboard and side's occupancy
   - Place pawn on `from`: create `Piece::new(side_to_move, PieceKind::Pawn)`, set bit in pawn bitboard and side's occupancy

   **Promotion capture** (`*_PROMOTION_CAPTURE`):
   - Remove promoted piece from `to`: clear bit in promoted piece's bitboard and side's occupancy
   - Place pawn on `from`: set bit in pawn bitboard and side's occupancy
   - Restore captured piece on `to`: read from `undo.captured` (guaranteed `Some`); set bit in captured piece's bitboard and opponent's occupancy

4. **Update fullmove counter**: Decrement by 1 if the side whose move is being undone was Black (i.e., `self.side_to_move() == Color::Black` after step 1). This reverses the increment that `make_move` applied after Black's move.

5. **Restore saved state from `UndoInfo`**: Use the `pub(crate)` setter methods to restore:
   - `self.set_castling_rights(undo.castling_rights)`
   - `self.set_en_passant(undo.en_passant)`
   - `self.set_halfmove_clock(undo.halfmove_clock)`

6. **Restore Zobrist hash**: `self.hash = undo.hash` (direct assignment via a setter or direct field write within the crate). This is the single-assignment approach — no incremental XOR reversal needed.

7. **Recompute total occupancy**: `self.occupied = self.occupied_by[White] | self.occupied_by[Black]`. This must happen after all piece bitboard modifications are complete. Use the same pattern as `make_move` (via `occupied_by` combination or the setter method).

### Bitboard update pattern

Each piece placement/removal follows the same mechanical pattern used in `make_move`:

```rust
// Remove piece from square sq
*self.piece_bb_mut(piece) ^= Bitboard::new(1u64 << sq.index());
*self.occupied_by_mut(piece.color) ^= Bitboard::new(1u64 << sq.index());

// Place piece on square sq
*self.piece_bb_mut(piece) ^= Bitboard::new(1u64 << sq.index());
*self.occupied_by_mut(piece.color) ^= Bitboard::new(1u64 << sq.index());
```

XOR is its own inverse, so the same operation both sets and clears bits. The code reuses the `piece_bb_mut` and `occupied_by_mut` pub(crate) methods from T01.

### Hash restoration detail

The `Position` struct stores the hash as a private `hash: u64` field. T01 added toggle methods (`toggle_piece_hash`, `toggle_side_to_move_hash`, `toggle_castling_hash`, `toggle_en_passant_hash`) but `unmake_move` does not use them. Instead, it needs a way to directly assign the hash value. Two options:
- Use the existing hash toggling to XOR to the right value (complex, error-prone)
- Add a `pub(crate) fn set_hash(&mut self, h: u64)` setter method (consistent with the existing setter pattern)

The plan uses the setter approach: add `pub(crate) fn set_hash(&mut self, h: u64)` to the existing block of setter methods. This is a one-line method consistent with `set_halfmove_clock`, `set_castling_rights`, etc.

## 4. API Contracts

```rust
use chess_board::{Position, UndoInfo};
use chess_types::{Move, MoveFlag, Square};

let mut pos = Position::startpos();
let original_fen = pos.to_fen();
let original_hash = pos.hash();

// Make a move
let mv = Move::new(Square::E2, Square::E4, MoveFlag::DOUBLE_PAWN_PUSH);
let undo: UndoInfo = pos.make_move(mv);

// Position is now modified
assert_ne!(pos.to_fen(), original_fen);

// Unmake the move — position fully restored
pos.unmake_move(mv, undo);
assert_eq!(pos.to_fen(), original_fen);
assert_eq!(pos.hash(), original_hash);
```

N/A for REST/HTTP.

## 5. Test Plan

All tests are colocated in `crates/board/src/position.rs` within `#[cfg(test)] mod tests`.

Each test follows the same **round-trip identity pattern**:
1. Construct a `Position` from a specific FEN string tailored for the move type
2. Snapshot `pos.to_fen()` and `pos.hash()` before the move
3. Call `pos.make_move(mv)` to get `UndoInfo`
4. Call `pos.unmake_move(mv, undo)`
5. Assert all fields match the pre-move state

**Verification checks** applied in every test:
- `pos.to_fen() == original_fen` (FEN round-trip)
- `pos.hash() == original_hash` (Zobrist hash round-trip)
- `pos.side_to_move()` matches original
- `pos.castling_rights()` matches original
- `pos.en_passant()` matches original
- `pos.halfmove_clock()` matches original
- `pos.fullmove_counter()` matches original
- Piece bitboards match original (verified via `pos.piece_on(sq)` for key squares)
- Occupancy bitboards match original (verified via `pos.occupied()`, `pos.occupied_by(Color::White)`, `pos.occupied_by(Color::Black)`)

| Test | Setup FEN | Move | What it verifies |
|------|-----------|------|-----------------|
| `unmake_quiet` | Startpos | Ng1-f3 (`G1→F3, QUIET`) | Round-trip for a quiet non-pawn move. Knight returns to g1, f3 empty. Halfmove clock restored. |
| `unmake_capture` | `rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2` | e4xd5 (`E4→D5, CAPTURE`) | Captured black pawn restored on d5. White pawn back on e4. Halfmove clock restored. |
| `unmake_double_pawn_push` | Startpos | e2-e4 (`E2→E4, DOUBLE_PAWN_PUSH`) | Pawn returns to e2. En passant square restored to `None` (original had no EP). |
| `unmake_white_kingside_castle` | `r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1` | O-O (`E1→G1, KING_CASTLE`) | King back on e1, rook back on h1. Castling rights restored to KQkq. |
| `unmake_white_queenside_castle` | `r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1` | O-O-O (`E1→C1, QUEEN_CASTLE`) | King back on e1, rook back on a1. Castling rights restored to KQkq. |
| `unmake_black_kingside_castle` | `r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1` | O-O (`E8→G8, KING_CASTLE`) | King back on e8, rook back on h8. Castling rights restored to KQkq. |
| `unmake_black_queenside_castle` | `r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1` | O-O-O (`E8→C8, QUEEN_CASTLE`) | King back on e8, rook back on a8. Castling rights restored to KQkq. |
| `unmake_en_passant` | `rnbqkbnr/pppp1ppp/8/4pP2/8/8/PPPPP1PP/RNBQKBNR w KQkq e6 0 3` | f5xe6 (`F5→E6, EN_PASSANT`) | White pawn back on f5, black pawn restored on e5 (not e6). En passant square restored to e6. |
| `unmake_promotion_knight` | `4k3/4P3/8/8/8/8/8/4K3 w - - 0 1` | e7-e8=N (`E7→E8, KNIGHT_PROMOTION`) | Knight removed from e8, pawn restored on e7. |
| `unmake_promotion_bishop` | `4k3/4P3/8/8/8/8/8/4K3 w - - 0 1` | e7-e8=B (`E7→E8, BISHOP_PROMOTION`) | Bishop removed from e8, pawn restored on e7. |
| `unmake_promotion_rook` | `4k3/4P3/8/8/8/8/8/4K3 w - - 0 1` | e7-e8=R (`E7→E8, ROOK_PROMOTION`) | Rook removed from e8, pawn restored on e7. |
| `unmake_promotion_queen` | `4k3/4P3/8/8/8/8/8/4K3 w - - 0 1` | e7-e8=Q (`E7→E8, QUEEN_PROMOTION`) | Queen removed from e8, pawn restored on e7. |
| `unmake_promotion_capture` | `3nk3/4P3/8/8/8/8/8/4K3 w - - 0 1` | e7xd8=Q (`E7→D8, QUEEN_PROMOTION_CAPTURE`) | Queen removed from d8, pawn restored on e7, black knight restored on d8. |
| `unmake_fullmove_counter` | `rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1` | e7-e5 (`E7→E5, DOUBLE_PAWN_PUSH`) | After Black's move and unmake, fullmove counter returns to 1 (not 2). Verifies decrement logic for Black moves. |
| `unmake_preserves_ep_state` | `rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1` | Nb8-c6 (`B8→C6, QUIET`) | Position starts with en passant on e3. After make/unmake, en passant on e3 is restored. Verifies UndoInfo correctly saves and restores a non-None EP square. |
| `unmake_multiple_sequential` | Startpos | e2-e4, then d7-d5, then e4xd5 — make all three, unmake all three in reverse order | After unmaking all three moves, position equals startpos. Verifies that `unmake_move` composes correctly across multiple moves. |

**Test setup pattern**: Tests use FEN positions that isolate the specific move type. Each test stores the full FEN string and hash before `make_move`, executes `make_move` + `unmake_move`, then asserts exact equality for FEN and hash. Additionally, specific squares are checked with `piece_on` to confirm pieces are in the correct locations.

## 6. Implementation Order

1. Add `pub(crate) fn set_hash(&mut self, h: u64)` setter method to `Position` (alongside existing setters)
2. Implement `Position::unmake_move(&mut self, mv: Move, undo: UndoInfo)` with the 7-step algorithm (toggle side, extract info, handle by move type, update fullmove, restore UndoInfo fields, restore hash, recompute occupancy)
3. Write round-trip unit tests for all move types (quiet through promotion-capture)
4. Write FEN round-trip tests
5. Write the sequential multi-move test
6. Run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test -p chess-board`

## 7. Verification Commands

```bash
# Format check
cargo fmt --check --manifest-path crates/board/Cargo.toml

# Lint check
cargo clippy -p chess-board -- -D warnings

# Run all tests in the board crate
cargo test -p chess-board

# Run only the unmake_move tests (pattern match)
cargo test -p chess-board unmake
```

## 8. Design Deviations

**Direct hash restoration instead of incremental reversal**: The phase spec states that `unmake_move` "restores the position to its exact prior state using the `UndoInfo`." The `UndoInfo` struct (from T01) already stores the pre-move Zobrist hash. Rather than incrementally reversing each XOR toggle that `make_move` applied (which would require duplicating `make_move`'s hash logic in reverse order and would be a source of bugs), `unmake_move` directly assigns `undo.hash` to `self.hash`. This is:
- **Simpler**: One assignment vs. 3-8 XOR operations depending on move type
- **Faster**: Fewer operations per unmake call (relevant for search performance)
- **More robust**: Cannot diverge from `make_move`'s hash logic since it uses the saved value
- **Consistent with the spec**: The spec says "Restores Zobrist hash from `UndoInfo` (direct restoration, not incremental re-reversal)" — this approach matches the spec exactly

This requires adding a `pub(crate) fn set_hash(&mut self, h: u64)` setter method, which is consistent with the existing pattern of `pub(crate)` setters added in T01 (`set_castling_rights`, `set_en_passant`, `set_halfmove_clock`, `set_fullmove_counter`).