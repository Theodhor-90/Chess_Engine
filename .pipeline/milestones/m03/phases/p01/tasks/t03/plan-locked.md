I now have everything needed. Here is the implementation plan:

## 1. Deliverables

| File | Action | Purpose |
|------|--------|---------|
| `crates/eval/src/phase.rs` | Create | Game phase calculation module: `compute_phase` function and `MAX_PHASE` constant |
| `crates/eval/src/lib.rs` | Modify | Add `pub mod phase;` declaration; replace placeholder `compute_phase` and `MAX_PHASE` with imports from `phase` module; update affected tests |

## 2. Dependencies

- **Internal crates (already available)**:
  - `chess-types` — provides `Bitboard`, `Color`, `Piece`, `PieceKind`
  - `chess-board` — provides `Position` with `piece_bitboard()` returning `Bitboard`, and `Bitboard::pop_count()` returning `u32`
- **T01 (completed)** — `material.rs` provides `KNIGHT_VALUE`, `BISHOP_VALUE`, `ROOK_VALUE`, `QUEEN_VALUE` constants
- **T02 (completed)** — `lib.rs` contains the tapered evaluation formula using `MAX_PHASE` and `compute_phase`, currently a placeholder returning `MAX_PHASE` always
- **No new external dependencies required**

## 3. Implementation Details

### `crates/eval/src/phase.rs`

**Purpose**: Compute a game phase score from remaining non-pawn, non-king material.

**Exports**:
- `pub const MAX_PHASE: i32 = 24;` — maximum phase value representing full starting non-pawn, non-king material. Uses a normalized scheme where N=1, B=1, R=2, Q=4. Starting material: 4×1 + 4×1 + 4×2 + 2×4 = 24.
- `pub fn compute_phase(pos: &Position) -> i32` — computes phase from the position.

**Key logic for `compute_phase`**:
1. Define phase weights per non-pawn, non-king piece type: Knight=1, Bishop=1, Rook=2, Queen=4. These are small integer weights used solely for phase calculation (distinct from material value constants, which are in centipawns). This is the standard approach (used by PeSTO/CPW) where `MAX_PHASE = 24` maps cleanly to the starting piece counts.
2. For each non-pawn, non-king piece type (Knight, Bishop, Rook, Queen), count the total number of that piece on the board (both colors combined) using `pos.piece_bitboard(Piece::new(Color::White, kind)).pop_count() + pos.piece_bitboard(Piece::new(Color::Black, kind)).pop_count()`.
3. Multiply each count by the phase weight for that piece type and sum.
4. Clamp the result to `[0, MAX_PHASE]` to handle positions with promoted pieces that could exceed starting material.
5. Return the clamped value.

**Unit tests** (in `#[cfg(test)] mod tests` within `phase.rs`):
- `startpos_phase_is_max`: Starting position yields `MAX_PHASE` (24).
- `king_and_pawns_phase_is_zero`: FEN with only kings and pawns yields phase 0.
- `missing_one_queen_intermediate`: Starting position minus one queen yields phase 20 (24 - 4).
- `promoted_pieces_clamped`: Position with extra queens from promotions, phase is clamped to `MAX_PHASE`.

### `crates/eval/src/lib.rs`

**Modifications**:
1. Add `pub mod phase;` to the module declarations (after `pub mod pst;`).
2. Remove the local `const MAX_PHASE: i32 = 24;` and the placeholder `fn compute_phase(_pos: &Position) -> i32 { MAX_PHASE }`.
3. Import `phase::compute_phase` and `phase::MAX_PHASE` (either via `use phase::{compute_phase, MAX_PHASE};` or qualified access `phase::compute_phase(pos)` / `phase::MAX_PHASE`).
4. The `evaluate` function body remains the same — it already calls `compute_phase(pos)` and uses `MAX_PHASE` in the tapered formula. The only change is these now resolve to the real implementation instead of placeholders.
5. Update the `tapered_eval_interpolation` test: with the real phase calculation, the test position `4k3/8/8/8/3N4/8/8/4K3 w - - 0 1` (one white knight, two kings) will have `phase = 1` (one knight × weight 1). The expected score changes from 333 (pure mg) to `(333 * 1 + eg_score * 23) / 24`. Compute the correct expected value:
   - mg_score = 333 (as previously computed in the existing test comment)
   - eg_score: White knight on d4 (index 27): 320 + EG_KNIGHT_TABLE[27] = 320 + 25 = 345. White king on e1 (index 4): 0 + EG_KING_TABLE[4] = 0 + (-28) = -28. Black king on e8 (index 60): mirror(60)=4, 0 + EG_KING_TABLE[4] = 0 + (-28) = -28. eg_score = (345 + (-28)) - (-28) = 345.
   - score = (333 * 1 + 345 * 23) / 24 = (333 + 7935) / 24 = 8268 / 24 = 344 (integer division: 344.5 truncates to 344).
   - Update assertion to `assert_eq!(evaluate(&pos), 344);` and update the comment to explain the real phase calculation.

## 4. API Contracts

**`phase::compute_phase(pos: &Position) -> i32`**
- **Input**: A reference to a `Position` (from `chess-board` crate).
- **Output**: An `i32` in the range `[0, MAX_PHASE]` (i.e., `[0, 24]`).
  - `MAX_PHASE` (24) = full starting material (all non-pawn, non-king pieces present) → pure middlegame weighting.
  - `0` = no non-pawn, non-king material remaining → pure endgame weighting.
  - Intermediate values = proportional interpolation.

**Examples**:
- Starting position → `24`
- `8/5k2/8/8/8/8/3K4/8 w - - 0 1` (bare kings) → `0`
- `8/5k2/8/8/8/8/3K4/4R3 w - - 0 1` (kings + one white rook) → `2`
- Starting position minus Black queen → `20`

**`phase::MAX_PHASE`**: `pub const i32 = 24`

The public `evaluate` function signature and return type are unchanged.

## 5. Test Plan

All tests use `Position::from_fen(...)` for setup and live in `#[cfg(test)] mod tests` blocks.

### Tests in `crates/eval/src/phase.rs`

| Test Name | Setup | Assertion | Traces To |
|-----------|-------|-----------|-----------|
| `startpos_phase_is_max` | Starting position FEN | `compute_phase(&pos) == MAX_PHASE` (24) | Spec: "Starting position yields full middlegame weighting (phase equals max_phase)" |
| `king_and_pawns_phase_is_zero` | `8/pppppppp/8/8/8/8/PPPPPPPP/8 w - - 0 1` (or similar kings+pawns FEN: `4k3/pppppppp/8/8/8/8/PPPPPPPP/4K3 w - - 0 1`) | `compute_phase(&pos) == 0` | Spec: "King-and-pawns-only endgame yields full endgame weighting (phase equals 0)" |
| `missing_one_queen_intermediate` | Starting position minus Black queen: `rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1` | `compute_phase(&pos) == 20` (24 - 4) | Spec: "Intermediate positions interpolate correctly (phase between 0 and max_phase)" |
| `promoted_pieces_clamped` | Position with extra queens exceeding starting material | `compute_phase(&pos) == MAX_PHASE` | Spec: "Clamp phase to [0, max_phase] to handle positions with promoted pieces" |

### Updated test in `crates/eval/src/lib.rs`

| Test Name | Change | Reason |
|-----------|--------|--------|
| `tapered_eval_interpolation` | Update expected value from `333` to `344` and update comment | Real phase calculation now returns `1` for this position instead of placeholder `24` |

## 6. Implementation Order

1. **Create `crates/eval/src/phase.rs`**: Define `MAX_PHASE` constant, phase weight constants, and `compute_phase` function with clamping. Add all four unit tests.
2. **Modify `crates/eval/src/lib.rs`**: Add `pub mod phase;`, remove placeholder `MAX_PHASE` and `compute_phase`, import from `phase` module, update the `tapered_eval_interpolation` test with the correct expected value.
3. **Verify**: Run `cargo test -p chess-eval`, `cargo fmt --check`, `cargo clippy -- -D warnings`.

## 7. Verification Commands

```bash
cargo test -p chess-eval
cargo fmt --check
cargo clippy -- -D warnings
```

## 8. Design Deviations

**Deviation: Phase weights use normalized small integers (1/1/2/4) instead of centipawn material values (320/330/500/900).**

- **What the task spec says**: "Use the material value constants defined in T01 (N=320, B=330, R=500, Q=900) to compute the phase contribution of each piece" and suggests `MAX_PHASE = 4×320 + 4×330 + 4×500 + 2×900 = 6400`.
- **Why that approach is problematic**: The existing T02 code in `lib.rs` already uses `const MAX_PHASE: i32 = 24` and the tapered formula divides by `MAX_PHASE`. The PST tables (PeSTO tables from T02) were calibrated assuming the standard `MAX_PHASE = 24` scheme (Knight=1, Bishop=1, Rook=2, Queen=4). Changing to `MAX_PHASE = 6400` would produce a mathematically equivalent formula only if the weights are exactly proportional, but Knight (320) and Bishop (330) have different centipawn values yet should contribute equally to game phase (one knight ≈ one bishop in terms of "how much material is on the board"). Using centipawn values would give bishops slightly more phase weight than knights, which is non-standard and mismatched with the PeSTO table design. Additionally, the T02 placeholder already establishes `MAX_PHASE = 24` as the contract.
- **Alternative chosen**: Use the standard PeSTO/CPW phase scheme: Knight=1, Bishop=1, Rook=2, Queen=4, `MAX_PHASE = 24`. This matches the existing placeholder, is consistent with the PeSTO tables from T02, and treats knights and bishops as equal phase contributors (which is the standard approach in chess programming).