Now I have the complete picture. Let me produce the revised plan.

## 1. Deliverables

| # | File | Action | Purpose |
|---|------|--------|---------|
| 1 | `crates/board/src/zobrist.rs` | Create | Zobrist key tables (compile-time constants), `compute_hash` function for full hash from position state, and key accessor helpers |
| 2 | `crates/board/src/lib.rs` | Modify | Add `pub mod zobrist;` declaration |
| 3 | `crates/board/src/position.rs` | Modify | Add `hash: u64` field to `Position`, add `set_hash()` and `hash()` accessors, add `inner()` to `CastlingRights`, add incremental update methods |
| 4 | `crates/board/src/fen.rs` | Modify | Update `parse_fen` to compute and store the initial Zobrist hash after constructing the `Position` |

## 2. Dependencies

- **T01 (Position Struct)** — complete; provides `Position`, `CastlingRights`, field accessors
- **T02 (FEN Parser)** — complete; provides `parse_fen`, `format_fen`, `Position::new()`
- **`chess-types` crate** — provides `Bitboard`, `Color`, `Piece`, `PieceKind`, `Square`, `File`
- **No new external crates required** — Zobrist keys will be hardcoded constants generated from a fixed-seed PRNG computed offline

## 3. Implementation Details

### 3.1 `crates/board/src/zobrist.rs`

**Purpose**: Contains all Zobrist key tables and the full hash computation function.

**Constants**:
- `PIECE_SQUARE_KEYS: [[u64; 64]; 12]` — 768 pre-generated random `u64` values. Indexed by `piece.index()` (0..12) and `square.index()` (0..64).
- `SIDE_TO_MOVE_KEY: u64` — single key XOR'd in when it is Black's turn to move.
- `CASTLING_KEYS: [u64; 16]` — 16 keys, one per combination of the 4 castling flags. Indexed by `CastlingRights` inner `u8` value (0..16).
- `EN_PASSANT_KEYS: [u64; 8]` — 8 keys, one per file. Indexed by `File as u8` (0..7).

All keys are generated offline using a fixed-seed PRNG (xorshift64) and hardcoded as `const` arrays. This ensures deterministic hashing across runs with zero runtime cost.

**Key generation approach**: Use a simple xorshift64 PRNG with a known seed (e.g., `0x29A4_D52E_9652_73AB`) to produce all 793 keys (768 + 1 + 16 + 8). The keys will be produced by a `const fn` so the arrays are compile-time constants.

**Public function**:
- `pub fn compute_hash(pos: &Position) -> u64` — Compute the full Zobrist hash from scratch by iterating over all 12 piece bitboards, XOR-ing in piece-square keys for each set bit, then XOR-ing in the side-to-move key (if Black), the castling key for the current castling rights, and the en passant file key (if en passant is active).

**Public inline helpers** (used by `Position` for incremental updates):
- `pub fn piece_square_key(piece: Piece, square: Square) -> u64` — returns `PIECE_SQUARE_KEYS[piece.index()][square.index() as usize]`
- `pub fn side_to_move_key() -> u64` — returns `SIDE_TO_MOVE_KEY`
- `pub fn castling_key(rights: CastlingRights) -> u64` — returns `CASTLING_KEYS[rights.inner() as usize]`
- `pub fn en_passant_key(file: File) -> u64` — returns `EN_PASSANT_KEYS[file as usize]`

**`CastlingRights` needs an `inner()` accessor**: The `zobrist` module needs to index into `CASTLING_KEYS` by the raw `u8` value. `CastlingRights` currently has no public method to retrieve the raw value — an `inner()` method must be added to `CastlingRights` in `position.rs`.

### 3.2 `crates/board/src/lib.rs`

Add `pub mod zobrist;` to the module declarations. No items need to be re-exported since the Zobrist module's public API is consumed internally by `Position`.

### 3.3 `crates/board/src/position.rs`

**Changes**:

1. **Add `hash: u64` field** to the `Position` struct.

2. **Add `inner()` method to `CastlingRights`**:
   ```rust
   pub fn inner(self) -> u8 { self.0 }
   ```

3. **`Position::new()` signature is NOT changed.** The existing `pub(crate) fn new(...)` remains as-is (no `hash` parameter). This avoids inflating an already long parameter list and avoids a breaking change to the T02 constructor.

4. **Add `pub(crate) fn set_hash(&mut self, hash: u64)`** method on `Position`, which sets `self.hash = hash`.

5. **Update `Position::startpos()`** to compute the hash: construct the `Position` with `hash: 0` (via the struct literal it already uses), then call `zobrist::compute_hash(&pos)` and set it via `pos.hash = ...` (direct field access is possible since `startpos()` is inside `position.rs`).

6. **Add `hash()` accessor**:
   ```rust
   pub fn hash(&self) -> u64 { self.hash }
   ```

7. **Add incremental update methods** on `Position`:
   - `pub fn toggle_piece_hash(&mut self, piece: Piece, square: Square)` — XOR the piece-square key into `self.hash`
   - `pub fn toggle_side_to_move_hash(&mut self)` — XOR the side-to-move key into `self.hash`
   - `pub fn toggle_castling_hash(&mut self, rights: CastlingRights)` — XOR the castling key for the given rights into `self.hash`
   - `pub fn toggle_en_passant_hash(&mut self, file: File)` — XOR the en passant file key into `self.hash`

   These methods only update the hash field — they do not change any other position state. They are the building blocks that `make_move`/`unmake_move` (Phase P03) will use for incremental hash maintenance.

### 3.4 `crates/board/src/fen.rs`

**Change**: After constructing the `Position` via `Position::new(...)` at the end of `parse_fen`, compute the hash and set it using `set_hash()`.

**Chosen approach**: Use `set_hash()` (not a new parameter to `Position::new()`). This is consistent with section 3.3's decision to leave `new()` unchanged. The updated code at the end of `parse_fen` will be:

```rust
let mut pos = Position::new(
    piece_bb,
    occupied_by,
    occupied,
    side_to_move,
    castling_rights,
    en_passant,
    halfmove_clock,
    fullmove_counter,
);
pos.set_hash(crate::zobrist::compute_hash(&pos));
Ok(pos)
```

The `Position` struct's `hash` field will be initialized to `0` in `Position::new()` (hardcoded in the constructor body, not a parameter), and then overwritten by `set_hash()`.

## 4. API Contracts

**`zobrist::compute_hash(pos: &Position) -> u64`**
- Input: A reference to a fully constructed `Position`
- Output: A `u64` Zobrist hash computed from scratch by XOR-ing all applicable keys
- Example: `compute_hash(&Position::startpos())` returns a fixed non-zero `u64`

**`Position::hash(&self) -> u64`**
- Input: `&self`
- Output: The stored Zobrist hash value

**`Position::set_hash(&mut self, hash: u64)`**
- Visibility: `pub(crate)`
- Side effect: `self.hash = hash`

**`Position::toggle_piece_hash(&mut self, piece: Piece, square: Square)`**
- Side effect: `self.hash ^= zobrist::piece_square_key(piece, square)`

**`Position::toggle_side_to_move_hash(&mut self)`**
- Side effect: `self.hash ^= zobrist::side_to_move_key()`

**`Position::toggle_castling_hash(&mut self, rights: CastlingRights)`**
- Side effect: `self.hash ^= zobrist::castling_key(rights)`

**`Position::toggle_en_passant_hash(&mut self, file: File)`**
- Side effect: `self.hash ^= zobrist::en_passant_key(file)`

**`CastlingRights::inner(self) -> u8`**
- Returns the raw `u8` bitfield value

## 5. Test Plan

All tests are colocated in `crates/board/src/zobrist.rs` under `#[cfg(test)] mod tests`.

### Test 1: `startpos_hash_is_nonzero_and_deterministic`
- Call `Position::startpos()` twice
- Assert `pos.hash() != 0`
- Assert both hashes are equal

### Test 2: `same_fen_produces_same_hash`
- Construct two positions from the same FEN string via `Position::from_fen`
- Assert their hashes are equal
- Repeat for the starting position FEN to verify `from_fen` and `startpos` produce the same hash

### Test 3: `different_positions_produce_different_hashes`
- Construct positions from at least 5 distinct FEN strings (starting position, 3 middlegame, 1 endgame — reuse the FENs from existing tests in `fen.rs`)
- Assert all pairwise hashes are distinct

### Test 4: `incremental_update_matches_full_recomputation`

Construct two positions from FEN strings representing a "before" and "after" state, then verify that applying incremental XOR updates to the before-hash produces the after-hash.

Specifically:
- Construct `pos_before` from `"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"` (starting position)
- Construct `pos_after` from `"rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"` (after 1.e4)
- Starting from `pos_before.hash()`, apply the XOR incremental updates that correspond to the move 1.e4:
  1. XOR out white pawn on e2: `hash ^= piece_square_key(WhitePawn, E2)`
  2. XOR in white pawn on e4: `hash ^= piece_square_key(WhitePawn, E4)`
  3. XOR side-to-move: `hash ^= side_to_move_key()`
  4. XOR in en passant file e: `hash ^= en_passant_key(File::E)`
- Assert that the incrementally computed hash equals `pos_after.hash()` (which was computed from scratch by `from_fen`)

This approach avoids mutating private `Position` fields — it only reads the `hash()` of two independently-constructed positions and verifies the XOR arithmetic.

### Test 5: `from_fen_and_startpos_same_hash`
- `Position::startpos().hash()` must equal `Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap().hash()`

## 6. Implementation Order

1. **Add `inner()` to `CastlingRights`** in `position.rs` — needed by `zobrist.rs` for key indexing
2. **Add `hash: u64` field to `Position`** in `position.rs` — add the field (initialized to `0` in `new()`), add `hash()` accessor, add `set_hash()`, add incremental toggle methods
3. **Create `crates/board/src/zobrist.rs`** — Zobrist key tables (const arrays generated via const-fn xorshift64), `compute_hash` function, key accessor helpers
4. **Add `mod zobrist;`** to `crates/board/src/lib.rs`
5. **Update `Position::startpos()`** in `position.rs` — compute and set initial hash via `zobrist::compute_hash` after construction
6. **Update `parse_fen`** in `fen.rs` — call `pos.set_hash(zobrist::compute_hash(&pos))` before returning
7. **Add unit tests** to `zobrist.rs`
8. **Run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`** to verify everything passes

## 7. Verification Commands

```bash
cd /Users/theodhor/Desktop/Portfolio/Chess_Engine
cargo fmt --check
cargo clippy -- -D warnings
cargo test -p chess-board
cargo test -p chess-board -- zobrist
```

## 8. Design Deviations

**Deviation 1: `CastlingRights::inner()` accessor**

- **Parent spec requires**: Zobrist castling keys indexed by the 16 combinations of the 4 castling flags, accessed via `CastlingRights`
- **Problem**: `CastlingRights` wraps a `u8` but has no public accessor to retrieve the raw value. The `zobrist` module needs to index `CASTLING_KEYS` by this raw value.
- **Alternative chosen**: Add `pub fn inner(self) -> u8` to `CastlingRights`. This is the minimal change needed to support Zobrist key indexing. It is consistent with the pattern used by `Bitboard::inner()`.

**Deviation 2: `set_hash()` method instead of adding `hash` parameter to `Position::new()`**

- **Parent spec requires**: `Position::from_fen` and `Position::startpos` compute and store the initial hash
- **Problem**: `Position::new()` (a `pub(crate)` constructor from T02) does not accept a `hash` parameter. Computing the hash requires a fully constructed `Position` (to iterate piece bitboards), so we cannot compute it before calling `new()`.
- **Alternative chosen**: Keep `Position::new()` unchanged. Add `hash: u64` to the `Position` struct initialized to `0` in `new()`, and add a `pub(crate) fn set_hash(&mut self, hash: u64)` method. Both `startpos()` and `parse_fen` construct the position first, then call `set_hash(compute_hash(&pos))`. This avoids adding yet another parameter to the already long `new()` signature and avoids a breaking change to the T02 constructor.