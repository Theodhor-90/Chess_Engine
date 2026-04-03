Now I have all the context I need. Here is the complete implementation plan:

## 1. Deliverables

- **Modify**: `crates/eval/src/pawn.rs` — Add pawn structure evaluation functions (`evaluate_pawns`) and named constants for doubled, isolated, backward, passed, and connected pawn penalties/bonuses. Each term produces separate midgame and endgame score components. The function analyzes both colors and returns `(mg_score, eg_score)` representing White's pawn structure advantage. Unit tests are added for each pawn structure pattern.

## 2. Dependencies

No new external dependencies are required. The implementation uses only existing workspace crates:

- `chess-types` (already a dependency of `chess-eval`) — `Bitboard`, `Color`, `Piece`, `PieceKind`, `Square`, `File`, `Rank`
- `chess-board` (already a dependency of `chess-eval`) — `Position` with `piece_bitboard()`, `from_fen()`

## 3. Implementation Details

### `crates/eval/src/pawn.rs` — Pawn Structure Evaluation

**New constants** (all `i32`, named for tunability):

| Constant | Midgame | Endgame | Description |
|---|---|---|---|
| `DOUBLED_PAWN_PENALTY_MG` / `_EG` | -10 | -20 | Per extra pawn on a file beyond the first |
| `ISOLATED_PAWN_PENALTY_MG` / `_EG` | -15 | -20 | Pawn with no friendly pawns on adjacent files |
| `BACKWARD_PAWN_PENALTY_MG` / `_EG` | -10 | -15 | Pawn that cannot advance safely and has no adjacent support |
| `PASSED_PAWN_BONUS_MG` | `[0, 0, 5, 10, 20, 35, 60, 0]` | Indexed by rank (0-7), rank 0/7 unused |
| `PASSED_PAWN_BONUS_EG` | `[0, 0, 10, 20, 40, 70, 120, 0]` | Indexed by rank (0-7), rank 0/7 unused |
| `CONNECTED_PAWN_BONUS_MG` / `_EG` | 5 | 7 | Pawn defended by or defending a friendly pawn |

**New helper bitboard constants** (defined as `const` within the module):

- `FILE_MASKS: [Bitboard; 8]` — one mask per file (A=0..H=7), each being `0x0101_0101_0101_0101 << file`
- `ADJACENT_FILE_MASKS: [Bitboard; 8]` — union of the file masks for files immediately left and right (A → only B; D → C|E; H → only G)

**New public function**:

```rust
pub fn evaluate_pawns(pos: &Position) -> (i32, i32)
```

**Logic**:

1. Extract `white_pawns` and `black_pawns` bitboards from the position.
2. For each color, iterate over each pawn and evaluate:
   - **Doubled pawns**: For each file, count friendly pawns on that file. If count > 1, add `(count - 1) * DOUBLED_PAWN_PENALTY` to that color's score.
   - **Isolated pawns**: A pawn is isolated if there are no friendly pawns on adjacent files. Check `friendly_pawns & ADJACENT_FILE_MASKS[file]` is empty.
   - **Backward pawns**: A pawn is backward if (a) no friendly pawn on adjacent files at the same rank or behind can support its advance, AND (b) advancing it would place it on a square attacked by an enemy pawn. For White: check that no friendly pawns exist on adjacent files at ranks ≤ this pawn's rank (use a mask of all ranks up to and including this rank on adjacent files). Then check if the advance square (rank+1) is attacked by an enemy pawn (enemy pawn on adjacent file at rank+2 for white perspective). A pawn that is already passed or isolated is not also counted as backward.
   - **Passed pawns**: A pawn is passed if there are no enemy pawns on its file or adjacent files that are at the same rank or ahead (toward promotion). For White pawn on rank R: check `enemy_pawns & passed_mask` where `passed_mask` covers the current file and adjacent files from rank R+1 to rank 8. Bonus is indexed by rank from `PASSED_PAWN_BONUS` arrays.
   - **Connected pawns**: A pawn is connected if it is defended by a friendly pawn. For White: check if there is a friendly pawn at (file-1, rank-1) or (file+1, rank-1). This is equivalent to checking if the square is in the pawn attack set of friendly pawns. Compute white pawn attacks as `((white_pawns & NOT_A_FILE) << 7) | ((white_pawns & NOT_H_FILE) << 9)`, then intersect with `white_pawns`.
3. Combine: `mg_total = white_mg - black_mg`, `eg_total = white_eg - black_eg`.
4. Return `(mg_total, eg_total)`.

**Bitboard techniques for each pattern**:

- **Doubled**: `(friendly_pawns & FILE_MASKS[f]).pop_count()` per file.
- **Isolated**: `(friendly_pawns & ADJACENT_FILE_MASKS[f]).is_empty()`.
- **Passed (White)**: For a white pawn on square `sq` (rank `r`, file `f`), build a mask of file `f` and adjacent files from rank `r+1` upward. If `enemy_pawns & mask` is empty, the pawn is passed. This mask can be precomputed or built on the fly by taking `(FILE_MASKS[f] | ADJACENT_FILE_MASKS[f])` and masking out ranks ≤ `r`.
- **Backward (White)**: The pawn has no friendly support behind or beside: `friendly_pawns & ADJACENT_FILE_MASKS[f] & ranks_up_to_r` is empty. And advancing would be attacked: check if enemy pawns exist on adjacent files at rank `r+1` (i.e., enemy pawns that could attack the advance square at rank `r+1`). Specifically, an enemy pawn on adjacent file at rank `r+2` attacks the advance square for White. Skip if the pawn is already isolated (already penalized) or passed.
- **Connected (White)**: `white_pawn_attacks = ((white_pawns & NOT_A_FILE) << 7) | ((white_pawns & NOT_H_FILE) << 9)`. A white pawn is connected if `Bitboard::new(1u64 << sq) & white_pawn_attacks` is non-empty.

For Black, all directions are mirrored (shift right instead of left, rank directions inverted).

**Helper function** for building rank masks efficiently:

```rust
const fn rank_bb(rank: u8) -> Bitboard {
    Bitboard::new(0xFFu64 << (rank * 8))
}
```

And a helper to build "forward file + adjacent" masks for passed pawn detection:

```rust
fn forward_mask(file: u8, rank: u8, color: Color) -> Bitboard
```

Returns the bitboard covering the file and adjacent files from the next rank forward (toward promotion) for the given color.

## 4. API Contracts

### `evaluate_pawns(pos: &Position) -> (i32, i32)`

**Input**: A reference to a `Position` (from `chess_board`).

**Output**: A tuple `(mg_score, eg_score)` where:
- `mg_score`: Midgame pawn structure evaluation from White's perspective (positive = White advantage)
- `eg_score`: Endgame pawn structure evaluation from White's perspective (positive = White advantage)

**Examples**:
- Starting position (symmetric pawns): returns `(0, 0)`
- Position `"4k3/8/8/8/8/8/PP1P1PPP/4K3 w - - 0 1"` (White has isolated e-pawn gap, various structure): returns negative mg/eg due to structural weaknesses
- Position with White passed pawn on d5: returns positive values reflecting the passed pawn bonus

## 5. Test Plan

All tests are added to the `#[cfg(test)] mod tests` block in `crates/eval/src/pawn.rs`, extending the existing test module.

### Test 1: `doubled_pawns_penalized`
- **Setup**: FEN with White doubled pawns on e-file (e.g., `"4k3/8/8/8/4P3/8/4P3/4K3 w - - 0 1"` — White pawns on e2 and e4) vs a symmetric Black position without doubled pawns.
- **Verify**: `evaluate_pawns()` returns negative mg and eg scores (White is penalized for doubled pawns while Black has no pawns, so the doubled penalty makes White's score negative).

### Test 2: `isolated_pawn_penalized`
- **Setup**: FEN with White isolated pawn (e.g., `"4k3/8/8/8/8/8/4P3/4K3 w - - 0 1"` — lone White pawn on e2 with no pawns on d or f files). Compare against a position where the pawn has a neighbor.
- **Verify**: The position with an isolated pawn has a lower score than one with a supported pawn.

### Test 3: `backward_pawn_penalized`
- **Setup**: FEN with a backward White pawn (e.g., `"4k3/8/8/3p4/2P5/8/3P4/4K3 w - - 0 1"` — White pawn on d2 cannot advance because d3 would be attacked by the Black pawn on c4... adjusted to create a proper backward scenario). A White pawn on d2 with friendly pawns on c4 and e4 having already advanced, and a Black pawn on e3 guarding d3.
- **Verify**: `evaluate_pawns()` reflects the backward penalty in the returned scores.

### Test 4: `passed_pawn_bonus_scales_by_rank`
- **Setup**: Two FENs — one with a White passed pawn on d4 (rank 4) and one on d6 (rank 6). No enemy pawns that could block.
- **Verify**: The d6 position's mg and eg scores are higher than the d4 position's.

### Test 5: `connected_pawns_bonus`
- **Setup**: FEN with connected White pawns (e.g., `"4k3/8/8/8/3PP3/8/8/4K3 w - - 0 1"` — White pawns on d4 and e4, mutually defending). Compare against a position with the same pawns but separated (e.g., d4 and f4).
- **Verify**: The connected position scores higher than the separated position.

### Test 6: `both_colors_evaluated`
- **Setup**: FEN where Black has a structural weakness (e.g., doubled pawns) and White does not.
- **Verify**: `evaluate_pawns()` returns positive values (White advantage), confirming both colors are evaluated and combined.

### Test 7: `startpos_pawns_symmetric`
- **Setup**: Standard starting position.
- **Verify**: `evaluate_pawns()` returns `(0, 0)` since both sides have identical pawn structures.

## 6. Implementation Order

1. **Define bitboard helper constants** — Add `FILE_MASKS`, `ADJACENT_FILE_MASKS`, `NOT_A_FILE`, `NOT_H_FILE`, and `rank_bb()` helper as private constants/functions in `pawn.rs`.
2. **Define evaluation constants** — Add all `DOUBLED_PAWN_PENALTY_MG`, `_EG`, `ISOLATED_PAWN_PENALTY_MG`, `_EG`, `BACKWARD_PAWN_PENALTY_MG`, `_EG`, `PASSED_PAWN_BONUS_MG`, `_EG` (arrays by rank), `CONNECTED_PAWN_BONUS_MG`, `_EG`.
3. **Implement `evaluate_pawns` function** — Build the core logic evaluating all five pawn structure terms for both colors, returning `(i32, i32)`.
4. **Add unit tests** — Write the seven tests specified in the test plan.
5. **Run `cargo clippy` and `cargo test`** — Verify all tests pass and no warnings.

## 7. Verification Commands

```bash
# Run all eval crate tests (including existing T01 tests and new T02 tests)
cargo test -p chess-eval

# Run clippy with warnings-as-errors for the eval crate
cargo clippy -p chess-eval -- -D warnings

# Run rustfmt check
cargo fmt --check -p chess-eval

# Run full workspace tests to ensure no regressions
cargo test --workspace
```

## 8. Design Deviations

None.