I now have a thorough understanding of the codebase. Here is the complete task plan:

## 1. Deliverables

- **Create**: `crates/eval/src/pawn.rs` — new module containing `PawnHashEntry`, `PawnHashTable`, and the `pawn_zobrist_hash` function
- **Modify**: `crates/eval/src/lib.rs` — add `pub mod pawn;` declaration to register the new module

## 2. Dependencies

- `chess-board` crate (already a dependency of `chess-eval`) — provides `Position`, `zobrist::piece_square_key`
- `chess-types` crate (already a dependency of `chess-eval`) — provides `Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`
- No new crate dependencies required.

## 3. Implementation Details

### `crates/eval/src/pawn.rs`

**Structs:**

`PawnHashEntry`:
- Fields: `key: u64` (full pawn Zobrist key for verification), `mg: i32` (midgame score), `eg: i32` (endgame score)
- Derive `Clone, Copy, Default`
- Default initializes all fields to zero (key=0, mg=0, eg=0)

`PawnHashTable`:
- Fields: `entries: Vec<PawnHashEntry>`, `mask: usize`
- `DEFAULT_SIZE: usize = 16_384` (16K entries, power-of-two)

**Functions:**

`PawnHashTable::new() -> Self`:
- Allocates `Vec<PawnHashEntry>` with `DEFAULT_SIZE` elements initialized to default
- Sets `mask = DEFAULT_SIZE - 1`

`PawnHashTable::probe(&self, key: u64) -> Option<(i32, i32)>`:
- Compute index: `(key as usize) & self.mask`
- Read entry at index
- If `entry.key == key`, return `Some((entry.mg, entry.eg))`
- Otherwise return `None`
- Note: Uses the full `u64` key for verification (not a truncated verification key like the TT), since pawn hash entries are small and a full key match avoids false positives cheaply. The pawn hash table is simpler than the TT — no depth/age-based replacement policy is needed; always overwrite on store.

`PawnHashTable::store(&mut self, key: u64, mg: i32, eg: i32)`:
- Compute index: `(key as usize) & self.mask`
- Write `PawnHashEntry { key, mg, eg }` at that index (always-replace policy)

`pawn_zobrist_hash(pos: &Position) -> u64`:
- Initialize `hash: u64 = 0`
- For each color in `[Color::White, Color::Black]`:
  - Get the pawn bitboard: `pos.piece_bitboard(Piece::new(color, PieceKind::Pawn))`
  - Iterate over the bitboard (yields `u32` square indices)
  - For each square index, construct `Square::new(sq_idx as u8).unwrap()` and XOR `chess_board::zobrist::piece_square_key(Piece::new(color, PieceKind::Pawn), sq)` into `hash`
- Return `hash`
- Note: Uses the existing `piece_square_key` public function from `crates/board/src/zobrist.rs` (already `pub`). No new random key tables are introduced.

**Unit tests (in `#[cfg(test)] mod tests`):**

1. `probe_miss_returns_none` — Create a new `PawnHashTable`, probe with an arbitrary key, assert `None`
2. `store_then_probe_returns_correct_scores` — Store `(key=0x1234, mg=50, eg=30)`, probe with same key, assert `Some((50, 30))`
3. `different_pawn_configs_produce_different_keys` — Create two positions via `Position::from_fen` with different pawn structures (e.g., startpos vs a position with a pawn moved), compute `pawn_zobrist_hash` for each, assert the keys differ

### `crates/eval/src/lib.rs`

**Modification:** Add `pub mod pawn;` to the module declarations at the top of the file (after the existing `pub mod pst;` line). No other changes to this file in T01 — the integration into `evaluate()` is deferred to T03 per the phase spec.

## 4. API Contracts

```rust
use chess_eval::pawn::{PawnHashTable, pawn_zobrist_hash};
use chess_board::Position;

// Compute pawn hash from position
let pos = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
let key = pawn_zobrist_hash(&pos);

// Create table and probe/store
let mut table = PawnHashTable::new();
assert_eq!(table.probe(key), None);       // Miss
table.store(key, 42, 37);                 // Store mg=42, eg=37
assert_eq!(table.probe(key), Some((42, 37))); // Hit
```

N/A for REST/HTTP.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| `probe_miss_returns_none` | `probe()` on a fresh table returns `None` for any key |
| `store_then_probe_returns_correct_scores` | After `store(key, mg, eg)`, `probe(key)` returns `Some((mg, eg))` with the exact stored values |
| `different_pawn_configs_produce_different_keys` | `pawn_zobrist_hash` produces distinct keys for positions with different pawn placements |

These three tests map directly to the three verification criteria in the task spec: "probe miss returns `None`", "store followed by probe returns correct scores", "different pawn configurations produce different hash keys".

## 6. Implementation Order

1. `crates/eval/src/pawn.rs` — define `PawnHashEntry` struct with `Default`
2. `crates/eval/src/pawn.rs` — define `PawnHashTable` struct with `new()`, `probe()`, `store()`
3. `crates/eval/src/pawn.rs` — implement `pawn_zobrist_hash(pos: &Position) -> u64`
4. `crates/eval/src/pawn.rs` — write `#[cfg(test)] mod tests` with the three unit tests
5. `crates/eval/src/lib.rs` — add `pub mod pawn;` declaration

## 7. Verification Commands

```bash
cargo test -p chess-eval
cargo clippy -p chess-eval -- -D warnings
cargo fmt --check -p chess-eval
```

## 8. Design Deviations

**Pawn hash entry uses full `u64` key instead of a truncated verification key:**

- The task spec says `PawnHashEntry` stores "pawn Zobrist key (or verification bits)"
- The TT in `crates/search/src/tt.rs` uses a `u16` verification key (`hash >> 48`) to save space, since TT entries are large and the table may hold millions of entries
- For the pawn hash table (16K entries, each entry only stores key + two `i32` scores), the memory savings from truncating to `u16` are negligible (~96 KB vs ~128 KB total), while a full `u64` key virtually eliminates false positive cache hits
- The task spec explicitly allows "or verification bits", making either approach valid. The full key is chosen for correctness at negligible cost.