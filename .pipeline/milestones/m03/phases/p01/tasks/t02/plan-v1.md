Good. The iterator yields `u32` square indices. I'll need to convert them to `Square` via `Square::new(idx as u8)`. Now I have all the information I need. Here is the plan:

## 1. Deliverables

| File | Purpose |
|------|---------|
| `crates/eval/src/pst.rs` (new) | Piece-square table constant arrays: 12 tables total (6 piece types × middlegame + endgame), defined as `[i32; 64]` arrays indexed by square using little-endian rank-file mapping (a1=0, h8=63). Tables defined from White's perspective. |
| `crates/eval/src/lib.rs` (modify) | Add `pub mod pst;` declaration. Extend the `evaluate` function to: (1) iterate over all pieces on the board and accumulate separate middlegame and endgame scores from material values + PST bonuses, (2) apply tapered interpolation between mg/eg scores using a phase value. Use a placeholder phase constant until T03 wires in the real phase calculation. Update existing tests and add PST-specific tests. |

## 2. Dependencies

No new external dependencies. The existing `chess-types` and `chess-board` crate dependencies are sufficient. The implementation uses:

- `chess_types::{Bitboard, Square, Piece, PieceKind, Color}` — for iterating pieces and indexing tables
- `chess_board::Position` — for `piece_bitboard()`, `side_to_move()`, and position accessors

## 3. Implementation Details

### `crates/eval/src/pst.rs`

**Purpose**: Define all 12 piece-square tables as public constants.

**Exports**:
- `pub const MG_PAWN_TABLE: [i32; 64]`
- `pub const EG_PAWN_TABLE: [i32; 64]`
- `pub const MG_KNIGHT_TABLE: [i32; 64]`
- `pub const EG_KNIGHT_TABLE: [i32; 64]`
- `pub const MG_BISHOP_TABLE: [i32; 64]`
- `pub const EG_BISHOP_TABLE: [i32; 64]`
- `pub const MG_ROOK_TABLE: [i32; 64]`
- `pub const EG_ROOK_TABLE: [i32; 64]`
- `pub const MG_QUEEN_TABLE: [i32; 64]`
- `pub const EG_QUEEN_TABLE: [i32; 64]`
- `pub const MG_KING_TABLE: [i32; 64]`
- `pub const EG_KING_TABLE: [i32; 64]`
- `pub fn mg_table(kind: PieceKind) -> &'static [i32; 64]` — returns a reference to the middlegame table for the given piece kind
- `pub fn eg_table(kind: PieceKind) -> &'static [i32; 64]` — returns a reference to the endgame table for the given piece kind
- `pub fn mirror_square(sq: u8) -> usize` — mirrors a square index vertically (for Black's perspective: `sq ^ 56`)

**Key logic**:
- Tables are defined from White's perspective using the PeSTO table values (a well-known, proven set of PST values used widely in chess programming).
- Array layout: index 0 = a1, index 63 = h8 (little-endian rank-file). The tables are written with rank 8 at the top visually but stored so that index 0 corresponds to a1.
- For Black pieces, the square index is mirrored vertically via `sq ^ 56` to flip the rank (rank 1 ↔ rank 8, rank 2 ↔ rank 7, etc.).
- The `mg_table` and `eg_table` helper functions map `PieceKind` to the corresponding constant array reference via a match statement.

### `crates/eval/src/lib.rs`

**Purpose**: Wire PST into evaluation, implement tapered interpolation.

**Changes**:
1. Add `pub mod pst;` module declaration.
2. Define a placeholder constant `const MAX_PHASE: i32 = 24;` and a temporary helper `fn compute_phase(_pos: &Position) -> i32` that returns `MAX_PHASE` (full middlegame weighting) as a placeholder until T03 replaces it.
3. Rewrite `evaluate` to:
   - Initialize `mg_score: i32 = 0` and `eg_score: i32 = 0`.
   - For each `PieceKind` (Pawn through King), for each `Color`:
     - Get the piece bitboard from the position.
     - Iterate over set bits (square indices).
     - For White: index into the PST directly with `sq_idx as usize`.
     - For Black: index into the PST with `mirror_square(sq_idx as u8)`.
     - Add material value (from `material::piece_value(kind)`) + mg PST bonus to `mg_score`.
     - Add material value + eg PST bonus to `eg_score`.
     - Apply sign: add for White, subtract for Black.
   - Call `compute_phase(pos)` to get the phase value.
   - Apply tapered evaluation: `score = ((mg_score * phase) + (eg_score * (MAX_PHASE - phase))) / MAX_PHASE`.
   - Return score relative to side to move (negate if Black to move).

**Note on placeholder phase**: The placeholder returns `MAX_PHASE` (pure middlegame), which makes the tapered formula degenerate to just `mg_score`. T03 will replace `compute_phase` with the real implementation. The tapered interpolation formula itself is fully implemented and testable with explicit phase values in unit tests.

## 4. API Contracts

The public API does not change its signature — `evaluate(pos: &Position) -> i32` remains the sole entry point. Its contract is extended:

**Input**: `&Position` — any valid chess position.

**Output**: `i32` — score relative to side to move. Positive means the side to move has an advantage. The score now includes both material values and piece-square table bonuses, interpolated between middlegame and endgame via the tapered evaluation formula.

**PST lookup helpers** (new, internal to crate):
- `pst::mg_table(PieceKind::Knight)` → `&'static [i32; 64]` — returns the middlegame knight PST.
- `pst::eg_table(PieceKind::Knight)` → `&'static [i32; 64]` — returns the endgame knight PST.
- `pst::mirror_square(sq_idx)` → `usize` — flips rank for Black perspective. `mirror_square(0)` = `56` (a1 → a8).

**Tapered interpolation example**:
- With `phase = MAX_PHASE` (24): score = mg_score (pure middlegame).
- With `phase = 0`: score = eg_score (pure endgame).
- With `phase = 12`: score = (mg_score * 12 + eg_score * 12) / 24 = average of mg and eg.

## 5. Test Plan

All tests live in `crates/eval/src/lib.rs` under `#[cfg(test)] mod tests` (colocated with source, following project convention).

### Test setup
Tests construct `Position` values via `Position::from_fen(...)` and call `evaluate()` or internal functions directly. No external test fixtures needed.

### Tests

1. **`startpos_eval_is_zero`** (existing, update): Starting position should still evaluate to 0 for material — but with PST, White has a slight positional advantage (PST bonuses are symmetric between sides in the starting position, so the total should still be 0). Verify `evaluate(&startpos) == 0`.

2. **`white_advantage_positive_for_white`** (existing, verify still passes): Position with White having extra queen. Score should be positive when White to move.

3. **`white_advantage_negative_for_black`** (existing, verify still passes): Same position but Black to move. Score should be negative.

4. **`centralized_knight_scores_higher_than_rim_knight`** (new): Construct two positions identical except for knight placement — one with a White knight on d4 (centralized) and one with a White knight on a1 (rim). The position with the centralized knight should have a higher evaluation. This confirms PST bonuses affect evaluation.

5. **`tapered_eval_interpolation`** (new): Directly test the tapered interpolation formula by calling the internal evaluation with known mg/eg scores and explicit phase values. Use a position where mg and eg PST values differ meaningfully, and verify the interpolation produces expected intermediate values. This can be achieved by testing with a position and comparing the result against manually computed expected values using the PST constants.

6. **`pst_bonuses_are_additive_with_material`** (new): Verify that a position with extra material AND better piece placement scores higher than a position with only extra material. This confirms PST and material are combined, not replacing each other.

## 6. Implementation Order

1. **Create `crates/eval/src/pst.rs`**: Define all 12 PST constant arrays, the `mg_table`/`eg_table` lookup functions, and the `mirror_square` helper.

2. **Modify `crates/eval/src/lib.rs`**: Add `pub mod pst;` declaration. Define the placeholder `MAX_PHASE` constant and `compute_phase` function. Rewrite `evaluate` to accumulate separate mg/eg scores (material + PST), apply tapered interpolation, and return relative to side to move.

3. **Add/update unit tests in `crates/eval/src/lib.rs`**: Verify existing tests still pass, add new PST-specific tests.

4. **Run verification commands** to ensure everything compiles, passes lints, and all tests pass.

## 7. Verification Commands

```bash
# Check formatting
cargo fmt --check

# Run clippy lints
cargo clippy -- -D warnings

# Run all eval crate tests
cargo test -p chess-eval

# Run entire workspace tests to ensure no regressions
cargo test
```

## 8. Design Deviations

**Deviation: Placeholder phase instead of hardcoded mid-function constant**

- **Parent spec states**: "Until T03 is complete, this task may use a placeholder phase value for testing."
- **Approach chosen**: Define `MAX_PHASE` as a constant and `compute_phase()` as a function that returns `MAX_PHASE`. This gives T03 a clean function to replace rather than a hardcoded value scattered in the evaluation logic. The tapered interpolation formula is fully implemented and testable — tests can verify interpolation behavior by calling with known positions where the phase effect is observable.
- **Why**: This follows the spec's allowance for a placeholder while structuring the code so T03 can cleanly slot in the real phase calculation. This is not extra engineering — it is the minimal structure needed for the formula `score = ((mg_score * phase) + (eg_score * (max_phase - phase))) / max_phase` to be properly implemented and tested.

No other deviations needed.