Good. Now I have all the context. Here is the revised plan:

## 1. Deliverables

- **Create**: `crates/eval/src/king_centralization.rs` — New module containing the king centralization bonus function (Chebyshev distance to central 4 squares contributing only to EG score) and king-to-passed-pawn proximity bonus for pawn endgames, plus unit tests.
- **Modify**: `crates/eval/src/lib.rs` — Add `pub mod king_centralization;` declaration and integrate the king centralization EG bonus into the evaluation pipeline (adding it to `eg_score` before the tapered interpolation).

## 2. Dependencies

No new external dependencies. The implementation uses only crate-internal infrastructure:
- `chess_board::Position` — for piece bitboards
- `chess_types::{Bitboard, Color, Piece, PieceKind}` — for piece/square queries
- The `pawn` module's `forward_mask` function is not public, so passed pawn detection must be replicated locally using the same file mask / adjacent file mask approach already used elsewhere in the eval crate.

## 3. Implementation Details

### `crates/eval/src/king_centralization.rs`

**Purpose**: Compute an endgame-only king centralization bonus for each side, plus a king-to-passed-pawn proximity bonus in pure pawn endgames.

**Constants**:
- `KING_CENTER_BONUS_WEIGHT: i32 = 10` — tunable, centipawns per step closer to center
- `KING_PASSED_PAWN_PROXIMITY_WEIGHT: i32 = 5` — tunable, centipawns per step closer to passed pawn
- Central squares: d4 (index 27), d5 (35), e4 (28), e5 (36)
- File masks and adjacent file masks (reused pattern from pawn.rs) for passed pawn detection

**Exports**:
- `pub fn evaluate_king_centralization(pos: &Position) -> (i32, i32)` — returns `(mg, eg)` where `mg` is always 0 and `eg` is the centralization + proximity bonus from White's perspective (positive = White advantage).

**Key Logic — Chebyshev distance to center**:
```
fn chebyshev_distance_to_center(sq_idx: u32) -> i32 {
    let file = (sq_idx % 8) as i32;
    let rank = (sq_idx / 8) as i32;
    // Central squares: d4=(3,3), d5=(3,4), e4=(4,3), e5=(4,4)
    let dist_d4 = (file - 3).abs().max((rank - 3).abs());
    let dist_d5 = (file - 3).abs().max((rank - 4).abs());
    let dist_e4 = (file - 4).abs().max((rank - 3).abs());
    let dist_e5 = (file - 4).abs().max((rank - 4).abs());
    dist_d4.min(dist_d5).min(dist_e4).min(dist_e5)
}
```
- Bonus = `(3 - chebyshev_distance_to_center(king_sq)).max(0) * KING_CENTER_BONUS_WEIGHT`
- Contributes only to `eg` (endgame) component, `mg` = 0

**Key Logic — King proximity to passed pawns in pawn endgames**:
- Detect pure pawn endgame: only kings and pawns on the board (no knights, bishops, rooks, or queens for either side).
- If pure pawn endgame, for each passed pawn (using same forward mask logic as pawn.rs: a pawn with no enemy pawns on its file or adjacent files ahead of it), compute Chebyshev distance from the friendly king to the passed pawn, and award `(7 - distance) * KING_PASSED_PAWN_PROXIMITY_WEIGHT` as EG bonus.
- White passed pawns add to score; Black passed pawns subtract from score.

**Internal helpers** (private):
- `chebyshev_distance_to_center(sq_idx: u32) -> i32`
- `chebyshev_distance(sq1: u32, sq2: u32) -> i32`
- `is_pure_pawn_endgame(pos: &Position) -> bool`
- `is_passed_pawn(sq_idx: u32, color: Color, friendly_pawns: Bitboard, enemy_pawns: Bitboard) -> bool` — replicates the forward mask logic to detect if a pawn is passed (no enemy pawns on the pawn's file or adjacent files ahead of it)
- File mask and adjacent file mask constants (same values as in pawn.rs, local to this module)

**Unit tests** (in `#[cfg(test)] mod tests`):
1. `centralized_king_scores_higher_than_corner_king` — Compare two endgame positions: white king on e4 (centralized) vs white king on a1 (corner), both with same material. Assert the centralized king produces a higher EG score.
2. `king_proximity_to_passed_pawn_rewarded` — Pure pawn endgame with white king close to a passed pawn vs white king far from a passed pawn. Assert closer king gets higher EG bonus.

### `crates/eval/src/lib.rs`

**Modifications**:
1. Add `pub mod king_centralization;` to the module declarations (after `pub mod endgame;`).
2. In `evaluate()`, after the piece bonuses block and before the phase interpolation, call `king_centralization::evaluate_king_centralization(pos)` and add its MG/EG components to `mg_score`/`eg_score`:
```rust
let (kc_mg, kc_eg) = king_centralization::evaluate_king_centralization(pos);
mg_score += kc_mg;
eg_score += kc_eg;
```
Since `kc_mg` is always 0, this effectively adds the centralization bonus only to the endgame weight of the tapered evaluation.

## 4. API Contracts

**`king_centralization::evaluate_king_centralization(pos: &Position) -> (i32, i32)`**

- **Input**: A `&Position` reference — any valid chess position.
- **Output**: `(mg: i32, eg: i32)` — midgame score is always 0. Endgame score is positive when White's king is more centralized / closer to passed pawns, negative when Black's king is. Passed pawn proximity bonus is only active in pure pawn endgames (only kings + pawns on the board).

Example:
- Position: `8/8/8/8/4K3/8/4P3/7k w - - 0 1` (White king on e4, centralized)
  - `chebyshev_distance_to_center(e4)` = 0 → bonus = `(3 - 0) * 10 = 30`
  - `chebyshev_distance_to_center(h1)` = 4 → bonus = `(3 - 4).max(0) * 10 = 0`
  - EG = 30 - 0 = 30 (plus any passed pawn proximity if applicable)
  - Returns `(0, 30)` (approximate — passed pawn proximity may add more)

## 5. Test Plan

### Test Setup
All tests use `chess_board::Position::from_fen(...)` to construct positions. No external setup required.

### Tests in `crates/eval/src/king_centralization.rs`

**Test 1: `centralized_king_scores_higher_than_corner_king`**
- FEN 1 (centralized): `4k3/8/8/8/4K3/8/8/8 w - - 0 1` (White king e4, Black king e8)
- FEN 2 (corner): `4k3/8/8/8/8/8/8/K7 w - - 0 1` (White king a1, Black king e8)
- Call `evaluate_king_centralization` on both.
- Assert centralized `eg` > corner `eg`.
- Assert both `mg` values are 0.

**Test 2: `king_proximity_to_passed_pawn_rewarded`**
- FEN 1 (close): `8/8/8/3PK3/8/8/8/7k w - - 0 1` (White king e5 close to passed pawn d5)
- FEN 2 (far): `8/8/8/3P4/8/8/8/K6k w - - 0 1` (White king a1 far from passed pawn d5)
- Both are pure pawn endgames.
- Call `evaluate_king_centralization` on both.
- Assert close `eg` > far `eg`.

## 6. Implementation Order

1. **Create `crates/eval/src/king_centralization.rs`**:
   - Define constants (`KING_CENTER_BONUS_WEIGHT`, `KING_PASSED_PAWN_PROXIMITY_WEIGHT`, file masks, adjacent file masks)
   - Implement `chebyshev_distance_to_center(sq_idx: u32) -> i32` using chained `.min()` calls on concrete distance values
   - Implement `chebyshev_distance(sq1: u32, sq2: u32) -> i32`
   - Implement `is_pure_pawn_endgame(pos: &Position) -> bool`
   - Implement `is_passed_pawn(sq_idx: u32, color: Color, friendly_pawns: Bitboard, enemy_pawns: Bitboard) -> bool`
   - Implement `pub fn evaluate_king_centralization(pos: &Position) -> (i32, i32)`
   - Add `#[cfg(test)] mod tests` with the two unit tests

2. **Modify `crates/eval/src/lib.rs`**:
   - Add `pub mod king_centralization;` to module declarations
   - Add king centralization call in `evaluate()` after the piece bonuses block, before phase interpolation

3. **Run tests** to verify correctness

## 7. Verification Commands

```bash
# Run all eval crate tests (including new king_centralization tests)
cargo test -p chess-eval

# Run only king_centralization tests
cargo test -p chess-eval king_centralization

# Check formatting
cargo fmt --check -p chess-eval

# Run clippy lints
cargo clippy -p chess-eval -- -D warnings

# Run the full workspace test suite to verify nothing is broken
cargo test --workspace
```

## 8. Design Deviations

**Deviation 1: Passed pawn detection reimplemented locally instead of reusing pawn module function**

- **Parent spec requires**: "This should leverage the passed pawn detection implemented in P01."
- **Why this is problematic**: The `forward_mask` function in `pawn.rs` is private (`fn forward_mask`, not `pub fn`). Making it public would be an out-of-scope modification to the pawn module not required by the task spec. Additionally, the existing `evaluate_passed_pawn_extras` function in pawn.rs doesn't expose a "list of passed pawns" — it directly computes king proximity scores internally.
- **Alternative chosen**: Reimplement the passed pawn detection logic (file masks, adjacent file masks, forward checking) locally within `king_centralization.rs`, following the same algorithmic approach. This is consistent with the pattern in the codebase where `pieces.rs`, `king_safety.rs`, and `pawn.rs` all define their own local `FILE_MASKS` constants rather than sharing from a common location.