Now I have full context. The feedback is clear: the plan introduces `PASSED_PAWN_ADVANCE_BONUS_MG/EG` constants but T02 already handles the rank-based advancement bonus via `PASSED_PAWN_BONUS_MG/EG` in `evaluate_color()`. The new constants are dead code. I need to remove them and clarify that T03 only adds king proximity and blockade — no new advancement bonus arrays.

## 1. Deliverables

- **Modify**: `crates/eval/src/pawn.rs` — Add passed pawn king proximity bonus function, blockade penalty function, and a `chebyshev_distance` helper. Add unit tests for blockade penalty and king proximity scoring.
- **Modify**: `crates/eval/src/lib.rs` — Integrate pawn evaluation into `evaluate()` by adding a `&mut PawnHashTable` parameter, probing/storing the pawn hash table, calling `evaluate_pawns()` for cacheable pawn structure scores, and calling the new passed pawn king proximity/blockade function for position-dependent adjustments. Combine pawn mg/eg scores into the tapered evaluation. Update existing tests to pass a `PawnHashTable`.
- **Modify**: `crates/search/src/lib.rs` — Add a `PawnHashTable` field to `SearchContext`, initialize it in `search()`, and pass `&mut ctx.pawn_table` to `chess_eval::evaluate()` at both call sites (in `quiescence` and `negamax`). Update test helper `test_ctx()` and affected tests.
- **Modify**: `crates/engine/src/main.rs` — No changes expected; the `PawnHashTable` is internal to the search context and `search()` signature does not change.

## 2. Dependencies

No new external crates or packages are required. All functionality uses existing dependencies:

- `chess_board::Position` — for piece bitboards and king square access
- `chess_types::{Bitboard, Square, Color, Piece, PieceKind}` — for square distance calculations
- `crates/eval/src/pawn.rs` (T01/T02) — the existing `PawnHashTable`, `pawn_zobrist_hash`, and `evaluate_pawns` functions

## 3. Implementation Details

### `crates/eval/src/pawn.rs`

**Purpose**: Add passed pawn king proximity and blockade evaluation.

**Clarification on advancement bonus**: The rank-based passed pawn advancement bonus is already fully handled by T02 via the `PASSED_PAWN_BONUS_MG/EG` arrays in the `evaluate_color()` function (lines 11–12, used at lines 117–118). T03 does NOT introduce any new advancement bonus constants or arrays. T03 adds only two position-dependent adjustments — king proximity and blockade — that are computed outside the pawn hash table cache because they depend on king positions and all pieces, not just pawns.

**New constants**:
- `KING_PROXIMITY_OWN_BONUS: i32` — per-distance-unit bonus for own king being close to a passed pawn (endgame-weighted, applied to `eg` score only). Suggested value: `5` per unit of proximity (i.e., `5 * (7 - distance)` so closer kings get more bonus).
- `KING_PROXIMITY_OPP_BONUS: i32` — per-distance-unit bonus for opponent king being far from a passed pawn (endgame-weighted, applied to `eg` score only). Suggested value: `3` per unit of distance.
- `BLOCKED_PASSED_PAWN_PENALTY_MG: i32` — penalty when a piece occupies the advance square. Suggested value: `-5`.
- `BLOCKED_PASSED_PAWN_PENALTY_EG: i32` — penalty when a piece occupies the advance square. Suggested value: `-10`.

**New public function**:
- `pub fn evaluate_passed_pawn_extras(pos: &Position) -> (i32, i32)` — iterates over all passed pawns for both colors (using the same `forward_mask` detection logic from `evaluate_color`), and for each passed pawn:
  1. Computes Chebyshev distance from the friendly king to the pawn square
  2. Computes Chebyshev distance from the opponent king to the pawn square
  3. Checks if the advance square (one rank forward toward promotion) is occupied by any piece (using `pos.occupied()`)
  4. Awards eg king proximity bonuses: `KING_PROXIMITY_OWN_BONUS * (7 - own_distance)` and `KING_PROXIMITY_OPP_BONUS * opp_distance`
  5. Applies blockade penalty (mg and eg) if the advance square is occupied
  6. Returns `(white_extras_mg - black_extras_mg, white_extras_eg - black_extras_eg)`

**Helper function** (private):
- `fn chebyshev_distance(sq1: u32, sq2: u32) -> i32` — computes `max(|file1 - file2|, |rank1 - rank2|)` using raw square indices (file = idx % 8, rank = idx / 8)

**New tests**:
- `blocked_passed_pawn_lower_than_unblocked` — White passed pawn on d5 with a piece on d6 vs White passed pawn on d5 with d6 empty; blocked version has lower extra score from `evaluate_passed_pawn_extras`.
- `king_proximity_own_king_close_scores_higher` — White passed pawn on d5, White king on e5 (close) vs White king on a1 (far). Call `evaluate_passed_pawn_extras`. Assert close-king position has higher extra score.
- `king_proximity_opp_king_far_scores_higher` — White passed pawn on d5, Black king on h8 (far) vs Black king on e6 (close). Assert far-opponent-king gives higher score.
- `pawn_hash_end_to_end` — Create a position, create a `PawnHashTable`, call `evaluate()` twice. Assert both calls return the same score (verifying cache consistency).

### `crates/eval/src/lib.rs`

**Purpose**: Integrate pawn evaluation into the tapered evaluation pipeline.

**Changes to `evaluate()`**:
- Add parameter `pawn_table: &mut PawnHashTable`
- After computing material + PST scores, compute pawn Zobrist hash via `pawn::pawn_zobrist_hash(pos)`
- Probe `pawn_table.probe(pawn_key)`:
  - On hit: use cached `(pawn_mg, pawn_eg)`
  - On miss: call `pawn::evaluate_pawns(pos)` to get `(pawn_mg, pawn_eg)`, then `pawn_table.store(pawn_key, pawn_mg, pawn_eg)`
- Call `pawn::evaluate_passed_pawn_extras(pos)` to get position-dependent `(pp_extra_mg, pp_extra_eg)` (these are NOT cached because they depend on king positions and all pieces, not just pawn structure)
- Add `pawn_mg + pp_extra_mg` to `mg_score` and `pawn_eg + pp_extra_eg` to `eg_score`
- The tapered interpolation formula remains unchanged

**Updated exports**: `pub use pawn::PawnHashTable;` added for the search crate to reference.

### `crates/search/src/lib.rs`

**Purpose**: Thread the `PawnHashTable` through search to `evaluate()`.

**Changes**:
- Add `pawn_table: chess_eval::PawnHashTable` field to `SearchContext`
- Initialize with `pawn_table: chess_eval::PawnHashTable::new()` in `search()` function
- In `quiescence`: change `chess_eval::evaluate(pos)` to `chess_eval::evaluate(pos, &mut ctx.pawn_table)`
- In `negamax`: change `chess_eval::evaluate(pos)` to `chess_eval::evaluate(pos, &mut ctx.pawn_table)`
- Update `test_ctx()` helper to include `pawn_table: chess_eval::PawnHashTable::new()`
- Update test call sites that call `chess_eval::evaluate(&pos)` directly to pass a `&mut PawnHashTable`

## 4. API Contracts

**`evaluate` (modified signature)**:
```rust
pub fn evaluate(pos: &Position, pawn_table: &mut PawnHashTable) -> i32
```
- Input: `&Position` (the current board state), `&mut PawnHashTable` (pawn hash table for caching)
- Output: `i32` — evaluation score from the perspective of the side to move (positive = good for side to move)

**`evaluate_passed_pawn_extras` (new)**:
```rust
pub fn evaluate_passed_pawn_extras(pos: &Position) -> (i32, i32)
```
- Input: `&Position`
- Output: `(mg_score, eg_score)` — king proximity and blockade adjustments for passed pawns, from White's perspective

**`chebyshev_distance` (private helper)**:
```rust
fn chebyshev_distance(sq1: u32, sq2: u32) -> i32
```
- Input: two square indices (0–63)
- Output: Chebyshev distance (max of file distance, rank distance)

## 5. Test Plan

All tests go in `#[cfg(test)] mod tests` sections of their respective files.

### `crates/eval/src/pawn.rs` — New Tests

1. **`blocked_passed_pawn_lower_than_unblocked`**: FEN with White passed pawn on d5 and a Black knight on d6 (blocked) vs FEN with d6 empty. Call `evaluate_passed_pawn_extras` on both positions. Assert blocked version yields lower extra score (the mg component should include the blockade penalty).

2. **`king_proximity_own_king_close_scores_higher`**: FEN with White passed pawn on d5, White king on e5 vs White king on a1. Black king on h8, no other pieces. Call `evaluate_passed_pawn_extras`. Assert close-king position has higher extra score (in eg component).

3. **`king_proximity_opp_king_far_scores_higher`**: FEN with White passed pawn on d5, White king on e1. Black king on h8 (far) vs Black king on e6 (close). Assert far-opponent-king gives higher extra score (in eg component).

### `crates/eval/src/lib.rs` — Updated/New Tests

4. **`pawn_hash_end_to_end`**: Create a position, create a `PawnHashTable`, call `evaluate()` twice. Assert both calls return the same score (verifying cache consistency).

5. **Existing tests updated**: `startpos_eval_is_zero`, `white_advantage_positive_for_white`, `white_advantage_negative_for_black`, `centralized_knight_scores_higher_than_rim_knight`, `tapered_eval_interpolation`, `pst_bonuses_are_additive_with_material` — all updated to pass `&mut PawnHashTable::new()` as second argument. The `tapered_eval_interpolation` test uses a position with no pawns, so pawn eval contributes 0 and the expected value remains 344.

## 6. Implementation Order

1. **Add `chebyshev_distance` helper** to `crates/eval/src/pawn.rs`
2. **Add king proximity/blockade constants** to `crates/eval/src/pawn.rs` (`KING_PROXIMITY_OWN_BONUS`, `KING_PROXIMITY_OPP_BONUS`, `BLOCKED_PASSED_PAWN_PENALTY_MG`, `BLOCKED_PASSED_PAWN_PENALTY_EG`)
3. **Implement `evaluate_passed_pawn_extras`** in `crates/eval/src/pawn.rs` — uses passed pawn detection logic (same `forward_mask` approach as `evaluate_color`), king proximity, and blockade detection
4. **Add unit tests** for `evaluate_passed_pawn_extras` in `crates/eval/src/pawn.rs`
5. **Modify `evaluate()` signature** in `crates/eval/src/lib.rs` to accept `&mut PawnHashTable`; add pawn hash table probe/store logic and passed pawn extras integration
6. **Add `pub use pawn::PawnHashTable;`** to `crates/eval/src/lib.rs`
7. **Update existing tests** in `crates/eval/src/lib.rs` to pass `&mut PawnHashTable::new()`
8. **Add `pawn_hash_end_to_end` test** in `crates/eval/src/lib.rs`
9. **Add `pawn_table` field** to `SearchContext` in `crates/search/src/lib.rs`
10. **Update `search()` function** to initialize `pawn_table` in `SearchContext`
11. **Update `evaluate()` call sites** in `quiescence` and `negamax` in `crates/search/src/lib.rs`
12. **Update `test_ctx()` and search tests** in `crates/search/src/lib.rs`

## 7. Verification Commands

```bash
# Run all eval crate tests (including existing T01/T02 tests and new T03 tests)
cargo test -p chess-eval

# Run all search crate tests (verify no regressions from signature change)
cargo test -p chess-search

# Run clippy with warnings-as-errors for the eval crate
cargo clippy -p chess-eval -- -D warnings

# Run clippy for the search crate
cargo clippy -p chess-search -- -D warnings

# Run rustfmt check
cargo fmt --check -p chess-eval
cargo fmt --check -p chess-search

# Run full workspace tests to ensure no regressions
cargo test --workspace
```

## 8. Design Deviations

**Deviation 1: `evaluate()` takes `&mut PawnHashTable` parameter instead of global/shared storage**

- The task spec says "it can be passed as a `&mut PawnHashTable` parameter or stored in a shared location accessible during search"
- Passing as a parameter is chosen because it avoids global mutable state, is consistent with Rust ownership conventions, and the `SearchContext` already owns other search-persistent data structures (TT, killer table, history table). The pawn hash table fits naturally as another `SearchContext` field.
- This requires changing `evaluate()`'s signature from `fn evaluate(pos: &Position) -> i32` to `fn evaluate(pos: &Position, pawn_table: &mut PawnHashTable) -> i32`, which propagates to the 2 production call sites in the search crate and existing test call sites.

No other deviations from the task spec are needed.