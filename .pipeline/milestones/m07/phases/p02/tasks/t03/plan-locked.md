Now I have all the context I need. Let me produce the implementation plan.

## 1. Deliverables

### New File
- **`crates/eval/src/pieces.rs`** — Piece-specific positional bonus evaluation module. Contains `evaluate_piece_bonuses()` function that detects and scores: bishop pair bonus, rook on open file bonus, rook on semi-open file bonus, rook on seventh rank bonus, and knight outpost bonus. Returns separate midgame and endgame `(i32, i32)` scores.

### Modified File
- **`crates/eval/src/lib.rs`** — Add `pub mod pieces;` declaration, import and call `pieces::evaluate_piece_bonuses()` for both colors, add the resulting midgame/endgame scores to the accumulators before tapered interpolation. Update the `tapered_eval_interpolation` test's expected value to account for the new piece bonus terms.

## 2. Dependencies

No new external dependencies. The module uses the same crate-internal dependencies already available:
- `chess-types` (for `Bitboard`, `Color`, `Piece`, `PieceKind`)
- `chess-board` (for `Position`)

The `chess-movegen` crate is already a dependency in `crates/eval/Cargo.toml` (added by T01) but is not needed by this task.

## 3. Implementation Details

### `crates/eval/src/pieces.rs`

**Purpose**: Evaluate piece-specific positional bonuses for a single color.

**Exports**: `pub fn evaluate_piece_bonuses(pos: &Position, color: Color) -> (i32, i32)`

**Constants**:

```rust
// Bishop pair bonus
const BISHOP_PAIR_BONUS_MG: i32 = 30;
const BISHOP_PAIR_BONUS_EG: i32 = 50;

// Rook on open file (no pawns at all on file)
const ROOK_OPEN_FILE_BONUS_MG: i32 = 20;
const ROOK_OPEN_FILE_BONUS_EG: i32 = 10;

// Rook on semi-open file (no friendly pawns, opponent pawns remain)
const ROOK_SEMI_OPEN_FILE_BONUS_MG: i32 = 10;
const ROOK_SEMI_OPEN_FILE_BONUS_EG: i32 = 5;

// Rook on seventh rank
const ROOK_SEVENTH_RANK_BONUS_MG: i32 = 20;
const ROOK_SEVENTH_RANK_BONUS_EG: i32 = 30;

// Knight outpost
const KNIGHT_OUTPOST_BONUS_MG: i32 = 20;
const KNIGHT_OUTPOST_BONUS_EG: i32 = 15;
```

**Shared bitboard constants** (same pattern as `king_safety.rs` and `pawn.rs`):

```rust
const FILE_MASKS: [Bitboard; 8] = [
    Bitboard::new(0x0101_0101_0101_0101),
    Bitboard::new(0x0101_0101_0101_0101 << 1),
    // ... through file H
];

const NOT_A_FILE: Bitboard = Bitboard::new(0xFEFE_FEFE_FEFE_FEFE);
const NOT_H_FILE: Bitboard = Bitboard::new(0x7F7F_7F7F_7F7F_7F7F);
```

**Key Logic for `evaluate_piece_bonuses(pos, color)`**:

1. **Bishop pair**: Count bishops for `color` via `pos.piece_bitboard(Piece::new(color, PieceKind::Bishop)).pop_count()`. If `>= 2`, add `BISHOP_PAIR_BONUS_MG` / `BISHOP_PAIR_BONUS_EG`.

2. **Rook on open/semi-open file**: For each rook of `color`, get the file index (`sq_idx % 8`). Retrieve friendly pawns and enemy pawns. Check `FILE_MASKS[file]`:
   - If `(friendly_pawns & file_mask).is_empty() && (enemy_pawns & file_mask).is_empty()` → open file bonus.
   - Else if `(friendly_pawns & file_mask).is_empty()` (but enemy pawns exist) → semi-open file bonus.

3. **Rook on seventh rank**: For each rook of `color`, compute rank (`sq_idx / 8`). The seventh rank is rank 6 (index) for White and rank 1 (index) for Black. If the rook is on its seventh rank, add the bonus.

4. **Knight outpost**: For each knight of `color`, compute file (`sq_idx % 8`) and rank (`sq_idx / 8`). Check all three conditions:
   - **Ranks 4–6 on opponent's half**: For White, rank indices 4, 5, 6 (ranks 5–7 human). For Black, rank indices 1, 2, 3 (ranks 2–4 human). Actually, re-examining: "ranks 4–6" in the spec refers to ranks 4, 5, 6 in chess notation (1-indexed). In our 0-indexed system: rank 3, 4, 5. "Opponent's half" for White is ranks 5–8 (indices 4–7), for Black is ranks 1–4 (indices 0–3). The intersection of ranks 4–6 (indices 3–5) with opponent's half: White → indices 4, 5; Black → index 3. However, the spec says "ranks 4–6 on the opponent's half of the board" which should be interpreted as: the knight is on rank 4, 5, or 6 (1-indexed) AND on the opponent's half. For White, opponent's half is ranks 5–8 (indices 4–7). So for White: rank indices 3, 4, 5 intersected with 4–7 = indices 4, 5 (ranks 5–6). For Black, opponent's half is ranks 1–4 (indices 0–3). Rank indices 2, 3, 4 (ranks 3–5, mirrored: ranks 4–6 from black's perspective become indices 4, 3, 2). For Black: rank indices 2, 3 (ranks 3–4). To keep it simple and symmetric: for White, valid outpost squares are rank indices 4 and 5; for Black, rank indices 2 and 3.
   - **Supported by a friendly pawn**: Check if a friendly pawn is diagonally behind the knight. For White knight on `sq`, check if there is a friendly pawn on `(sq - 9)` (if not on A-file) or `(sq - 7)` (if not on H-file). For Black knight on `sq`, check if there is a friendly pawn on `(sq + 7)` (if not on A-file) or `(sq + 9)` (if not on H-file).
   - **Cannot be attacked by any opponent pawn**: No opponent pawn on the adjacent files at ranks in front of (from the opponent's perspective) the knight's rank. Specifically, check that no opponent pawn exists on the adjacent files at ranks that could advance to attack this square. For White's knight at rank `r` file `f`: opponent (Black) pawns attack by advancing down, so check if there are any black pawns on adjacent files (`f-1`, `f+1`) at ranks `r+1` through `6` (since a black pawn at rank `r+1` on an adjacent file directly attacks rank `r`; a black pawn further away can advance to attack). For Black's knight at rank `r` file `f`: check if white pawns exist on adjacent files at ranks `1` through `r-1`.

5. Sum all bonuses into `(mg, eg)` and return.

### `crates/eval/src/lib.rs`

**Changes**:
1. Add `pub mod pieces;` after the existing `pub mod mobility;` line.
2. After the mobility evaluation block (lines 68–71), add:
   ```rust
   let (w_pc_mg, w_pc_eg) = pieces::evaluate_piece_bonuses(pos, Color::White);
   let (b_pc_mg, b_pc_eg) = pieces::evaluate_piece_bonuses(pos, Color::Black);
   mg_score += w_pc_mg - b_pc_mg;
   eg_score += w_pc_eg - b_pc_eg;
   ```
3. Update the `tapered_eval_interpolation` test assertion. The test position `"4k3/8/8/8/3N4/8/8/4K3 w - - 0 1"` has a single White knight and no pawns, bishops, or rooks for either side — so no piece bonuses will fire (bishop pair needs ≥2, rook bonuses need rooks, knight outpost needs pawn support). The expected value should remain `378`.

## 4. API Contracts

**`pieces::evaluate_piece_bonuses(pos: &Position, color: Color) -> (i32, i32)`**

- **Input**: A reference to a `Position` and a `Color` indicating which side to evaluate.
- **Output**: A tuple `(mg_score, eg_score)` representing the total piece-specific positional bonus for that color. Both values are non-negative (bonuses only, no penalties).
- **Example**: For a position where White has two bishops and a rook on an open file: `(50, 60)` — the sum of bishop pair bonus `(30, 50)` and rook open file bonus `(20, 10)`.

## 5. Test Plan

All tests are colocated in `crates/eval/src/pieces.rs` under `#[cfg(test)] mod tests`.

### Test 1: `bishop_pair_bonus_awarded`
- **Setup**: Position with White having two bishops: `"4k3/8/8/8/8/8/8/2B1KB2 w - - 0 1"`
- **Verify**: `evaluate_piece_bonuses(&pos, Color::White)` returns `(mg, eg)` where `mg >= BISHOP_PAIR_BONUS_MG` and `eg >= BISHOP_PAIR_BONUS_EG`.

### Test 2: `single_bishop_no_pair_bonus`
- **Setup**: Position with White having one bishop: `"4k3/8/8/8/8/8/8/2B1K3 w - - 0 1"`
- **Verify**: `evaluate_piece_bonuses(&pos, Color::White)` returns `(0, 0)` (no bonuses triggered).

### Test 3: `rook_on_open_file`
- **Setup**: Position with White rook on a file with no pawns: `"4k3/8/8/8/8/8/PP3PPP/4K2R w - - 0 1"` (rook on h-file, no pawns on h-file)
- **Verify**: The mg component includes the open file bonus, i.e., `mg >= ROOK_OPEN_FILE_BONUS_MG`.

### Test 4: `rook_on_semi_open_file`
- **Setup**: Position with White rook on a file with only opponent pawns: `"4k3/7p/8/8/8/8/PP3PP1/4K2R w - - 0 1"` (rook on h-file, black pawn on h7, no white pawn on h-file)
- **Verify**: The mg component includes the semi-open file bonus. It is less than the open file bonus test.

### Test 5: `rook_on_seventh_rank`
- **Setup**: White rook on rank 7: `"4k3/R7/8/8/8/8/8/4K3 w - - 0 1"`
- **Verify**: `mg >= ROOK_SEVENTH_RANK_BONUS_MG` and `eg >= ROOK_SEVENTH_RANK_BONUS_EG`.

### Test 6: `knight_outpost_awarded`
- **Setup**: White knight on e5 (index 36, rank 4) with White pawn on d4 (index 27) supporting it, no Black pawns on d or f files that could attack e5: `"4k3/8/8/4N3/3P4/8/8/4K3 w - - 0 1"`
- **Verify**: `mg >= KNIGHT_OUTPOST_BONUS_MG` and `eg >= KNIGHT_OUTPOST_BONUS_EG`.

### Test 7: `knight_without_pawn_support_no_outpost`
- **Setup**: White knight on e5 but no supporting pawn: `"4k3/8/8/4N3/8/8/8/4K3 w - - 0 1"`
- **Verify**: The returned scores do not include the outpost bonus (mg == 0, eg == 0 since no other bonuses apply).

### Test 8: `all_bonuses_produce_midgame_and_endgame`
- **Setup**: Position where bishop pair and rook on open file both fire: `"4k3/8/8/8/8/8/8/R1B1KB2 w - - 0 1"` (two bishops + rook on a-file open)
- **Verify**: Both mg and eg are positive and greater than any single bonus alone.

## 6. Implementation Order

1. **Create `crates/eval/src/pieces.rs`**: Define all constants (`BISHOP_PAIR_BONUS_MG/EG`, `ROOK_OPEN_FILE_BONUS_MG/EG`, `ROOK_SEMI_OPEN_FILE_BONUS_MG/EG`, `ROOK_SEVENTH_RANK_BONUS_MG/EG`, `KNIGHT_OUTPOST_BONUS_MG/EG`), the `FILE_MASKS` array, `NOT_A_FILE` / `NOT_H_FILE` constants, and the `evaluate_piece_bonuses` function implementing all five bonus types.
2. **Add unit tests in `pieces.rs`**: Implement all 8 tests specified in the test plan inside a `#[cfg(test)] mod tests` block.
3. **Modify `crates/eval/src/lib.rs`**: Add `pub mod pieces;` declaration and integrate `pieces::evaluate_piece_bonuses()` calls for both colors into the `evaluate()` function. Verify/update the `tapered_eval_interpolation` test expected value if needed.
4. **Run `cargo test -p chess-eval`** to confirm all tests pass.
5. **Run `cargo clippy -p chess-eval -- -D warnings`** to confirm no lint warnings.

## 7. Verification Commands

```bash
cargo test -p chess-eval
cargo clippy -p chess-eval -- -D warnings
cargo fmt -p chess-eval -- --check
cargo test --workspace
```

## 8. Design Deviations

None.