Now I have all the context. Let me produce the revised plan addressing every feedback issue.

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
- Starting position (`"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"`): Returns `(0, 0)` because both sides have identical, symmetric pawn structures with no structural weaknesses.
- Position `"4k3/8/8/8/8/8/PP1P1PPP/4K3 w - - 0 1"` (White pawns on a2, b2, d2, f2, g2, h2; Black has no pawns): The d2 pawn is **isolated** (no friendly pawns on c-file or e-file), a2-b2 are **connected** (b2 defends a2 and vice-versa via pawn attacks), and f2-g2-h2 form a connected chain. The returned score is net negative (both mg and eg < 0) because the isolated penalty on d2 combines with the absence of any Black pawns to penalize (so White's raw score includes penalties with no offsetting Black penalties), even though there are connected bonuses for the other pawns. The exact values depend on the penalty/bonus constants.
- Position with White passed pawn on d5 (`"4k3/8/8/3P4/8/8/8/4K3 w - - 0 1"`): Returns positive mg and eg values reflecting the passed pawn bonus at rank 5 (index 4 for White: `PASSED_PAWN_BONUS_MG[4] = 20`, `PASSED_PAWN_BONUS_EG[4] = 40`), plus the isolated penalty since the d-pawn has no neighbors, resulting in `(20 + (-15), 40 + (-20)) = (5, 20)`.

## 5. Test Plan

All tests are added to the `#[cfg(test)] mod tests` block in `crates/eval/src/pawn.rs`, extending the existing test module.

### Test 1: `doubled_pawns_penalized`
- **Setup**: FEN `"4k3/8/8/8/4P3/8/4P3/4K3 w - - 0 1"` — White pawns on e2 and e4. Black has no pawns.
- **Verify**: `evaluate_pawns()` returns negative mg and eg scores. White has two pawns on the e-file (doubled penalty of -10 mg, -20 eg for the extra pawn). Both pawns are also isolated (no pawns on d or f files: -15 mg, -20 eg each). The total should be strongly negative.

### Test 2: `isolated_pawn_penalized`
- **Setup**: Two positions compared:
  - Isolated: FEN `"4k3/8/8/8/8/8/4P3/4K3 w - - 0 1"` — lone White pawn on e2 with no pawns on d or f files.
  - Supported: FEN `"4k3/8/8/8/8/8/3PP3/4K3 w - - 0 1"` — White pawns on d2 and e2 (adjacent, so neither is isolated).
- **Verify**: The isolated position has a lower score than the supported position.

### Test 3: `backward_pawn_penalized`
- **Setup**: FEN `"4k3/8/8/4p3/2P1P3/8/3P4/4K3 w - - 0 1"` — White pawns on c4, d2, and e4. Black pawn on e5. The d2 pawn is backward because: (a) no friendly pawns on adjacent files (c and e) at rank 2 or below — the c4 and e4 pawns are both above d2, so `friendly_pawns & ADJACENT_FILE_MASKS[d] & ranks_up_to_rank2` is empty; (b) advancing to d3 would place it on a square attacked by the Black pawn on e4... wait — actually the Black pawn is on e5. So: the d2 pawn cannot advance to d3 because d3 is attacked by the enemy pawn on e4? No — Black pawn on e5 attacks d4. Let me correct: With Black pawn on e5, advancing d2→d3 is not directly attacked; but d2 still cannot catch up to c4/e4 for support. Actually let me use a cleaner backward pawn position.
- **Revised Setup**: FEN `"4k3/8/8/8/2P1P3/4p3/3P4/4K3 w - - 0 1"` — White pawns on c4, d2, e4. Black pawn on e3. The d2 pawn is backward because: (a) friendly pawns on adjacent files c and e are both on rank 4, well ahead — no friendly pawn on c-file or e-file at rank 2 or below exists to support the d-pawn's advance; (b) advancing d2→d3 places it on a square attacked by the Black pawn on e3 (a Black pawn on e3 attacks d2... wait — a Black pawn on e3 attacks d2 and f2, not d3). Let me reconsider.

A Black pawn on e3 attacks squares d2 and f2 (Black pawns attack downward diagonally). That doesn't attack d3. For d3 to be attacked by a Black pawn, we need a Black pawn on c4 or e4 — but those are White pawns.

The correct setup: FEN `"4k3/8/8/8/2P1P3/8/3P4/4K3 w - - 0 1"` with an added Black pawn that attacks d3. A Black pawn on c4 attacks d3, but c4 has a White pawn. A Black pawn on e4 attacks d3, but e4 has a White pawn. So we need: FEN `"4k3/3p4/8/8/8/2P1P3/3P4/4K3 w - - 0 1"` — White pawns on c3, d2, e3. Black pawn on d7. The d2 pawn is behind c3 and e3. Advancing d2→d3 doesn't get attacked by Black d7 pawn. This still isn't a clean backward example.

Let me use the textbook backward pawn definition correctly:

**Final Setup**: FEN `"4k3/8/3p4/8/8/2P1P3/3P4/4K3 w - - 0 1"` — White pawns on c3, d2, e3. Black pawn on d6. The White d2 pawn is backward: its adjacent-file friendly pawns (c3 and e3) have already advanced past it, so no friendly pawn can support the d-pawn from behind or beside. And the d-pawn cannot safely advance because pushing d2→d3 results in a pawn that is still behind c3/e3, and the advance square d4 will eventually be controlled. However the immediate backward condition is: no support behind + advance is attacked.

Actually, the simplest textbook backward pawn: FEN `"4k3/8/8/2pp4/8/2P1P3/3P4/4K3 w - - 0 1"` — White pawns on c3, d2, e3. Black pawns on c5, d5. The White d2 pawn is backward because (a) friendly pawns c3 and e3 are ahead of it on adjacent files so they cannot support d2's advance, and (b) pushing d2→d3 creates a pawn that would face capture after d3→d4 being attacked by the c5 pawn. In the standard backward definition, d3 is attacked by the Black pawn on c4... we don't have c4.

I need to simplify. Let me use the most direct backward pawn scenario:

**Final Setup**: FEN `"4k3/8/8/8/3p4/4P3/3P4/4K3 w - - 0 1"` — White pawns on d2 and e3. Black pawn on d4. The d2 pawn is backward: the only adjacent-file friendly pawn is e3, which is already on rank 3 (ahead of d2 on rank 2). Advancing d2→d3 would be attacked by the Black pawn on d4 — wait, a pawn on d4 attacks c3 and e3, not d3. Pawns attack diagonally.

For d3 to be attacked by a Black pawn, Black needs a pawn on c4 or e4. Let me use: FEN `"4k3/8/8/8/4p3/2P5/3P4/4K3 w - - 0 1"` — White pawns on c3 and d2. Black pawn on e4. Advancing d2→d3 places it on a square attacked by the Black pawn on e4 (e4 attacks d3 and f3). Meanwhile, the only adjacent friendly pawn is on c3 (rank 3) which is ahead of d2 — no friendly pawn at rank ≤ 2 on adjacent files c or e. So d2 is backward. The d-pawn is not isolated (c3 is on an adjacent file) and not passed (Black pawn on e4 can reach d-file adjacent ranks).
- **Verify**: `evaluate_pawns()` reflects the backward penalty. Specifically compare against a position where the d-pawn is not backward (e.g., `"4k3/8/8/8/4p3/2PP4/8/4K3 w - - 0 1"` with White pawns on c3, d3 — the d3 pawn is no longer behind its neighbors). The backward position should score lower.

### Test 4: `passed_pawn_bonus_scales_by_rank`
- **Setup**: Two FENs — one with a White passed pawn on d4 (rank 4, index 3) and one on d6 (rank 6, index 5). No enemy pawns.
  - Rank 4: `"4k3/8/8/8/3P4/8/8/4K3 w - - 0 1"`
  - Rank 6: `"4k3/8/3P4/8/8/8/8/4K3 w - - 0 1"`
- **Verify**: The d6 position returns higher mg and eg scores than the d4 position (both should be positive since the pawn is passed, though also isolated in both cases — the relative difference comes from the passed bonus scaling).

### Test 5: `connected_pawns_bonus`
- **Setup**: Two FENs comparing connected vs separated pawns:
  - Connected: `"4k3/8/8/8/3PP3/8/8/4K3 w - - 0 1"` — White pawns on d4 and e4 (e4 defends d4 and d4 defends e4 via pawn attacks).
  - Separated: `"4k3/8/8/8/3P1P2/8/8/4K3 w - - 0 1"` — White pawns on d4 and f4 (not adjacent, cannot defend each other).
- **Verify**: The connected position scores higher than the separated position.

### Test 6: `both_colors_evaluated`
- **Setup**: FEN `"4k3/pp1pp3/8/8/8/8/PPPPPP2/4K3 w - - 0 1"` — White has 6 pawns (a2-f2) with no structural weaknesses (all connected in chain), Black has 4 pawns on a7, b7, d7, e7 with doubled-free but isolated d7/e7 (gap on c-file isolates a7-b7 from d7-e7 — actually b7 and d7 have a gap on c-file, so b7 has neighbor a7 only, d7 has neighbor e7 only; neither b7 nor d7 is isolated since each has at least one neighbor on adjacent file).
- **Revised Setup**: FEN `"4k3/pp1p4/8/8/8/8/PPP5/4K3 w - - 0 1"` — White pawns on a2, b2, c2 (connected chain). Black pawns on a7, b7, d7 (the d7 pawn is isolated — no Black pawns on c-file or e-file).
- **Verify**: `evaluate_pawns()` returns positive mg and eg values (White advantage), confirming Black's isolated d7 pawn penalty is detected and White's clean structure is credited.

### Test 7: `startpos_pawns_symmetric`
- **Setup**: Standard starting position `"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"`.
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

---

Wait — I realize the output above contains my working-through-the-problem reasoning in Test 3 which shouldn't be in the final document. Let me produce the clean final version.

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
- **Backward (White)**: The pawn has no friendly support behind or beside: `friendly_pawns & ADJACENT_FILE_MASKS[f] & ranks_up_to_r` is empty. And advancing would be attacked: check if enemy pawns exist on adjacent files at rank `r+2` (i.e., enemy pawns that attack the advance square at rank `r+1`). Skip if the pawn is already isolated (already penalized) or passed.
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
- Starting position (`"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"`): Returns `(0, 0)` because both sides have identical, symmetric pawn structures with no structural weaknesses.
- Position `"4k3/8/8/8/8/8/PP1P1PPP/4K3 w - - 0 1"` (White pawns on a2, b2, d2, f2, g2, h2; Black has no pawns): The d2 pawn is **isolated** (no friendly pawns on c-file or e-file) receiving `ISOLATED_PAWN_PENALTY` (-15 mg, -20 eg). The a2-b2 pair are **connected** (each defends the other). The f2-g2-h2 group forms a connected chain. White's raw score includes the isolated penalty on d2 plus connected bonuses on the supported pawns. Since Black has no pawns, there is no offsetting Black penalty. The net result is negative because the isolated penalty (-15 mg, -20 eg) outweighs the connected bonuses.
- Position `"4k3/8/8/3P4/8/8/8/4K3 w - - 0 1"` (White passed pawn on d5): Returns `(5, 20)` — the passed pawn bonus at rank 5 (`PASSED_PAWN_BONUS_MG[4] = 20`, `PASSED_PAWN_BONUS_EG[4] = 40`) combined with the isolated penalty (`ISOLATED_PAWN_PENALTY_MG = -15`, `_EG = -20`) yields `(20 + (-15), 40 + (-20)) = (5, 20)`.

## 5. Test Plan

All tests are added to the `#[cfg(test)] mod tests` block in `crates/eval/src/pawn.rs`, extending the existing test module.

### Test 1: `doubled_pawns_penalized`
- **Setup**: FEN `"4k3/8/8/8/4P3/8/4P3/4K3 w - - 0 1"` — White pawns on e2 and e4. Black has no pawns.
- **Verify**: `evaluate_pawns()` returns negative mg and eg scores. White has two pawns on the e-file (doubled penalty for the extra pawn) and both pawns are isolated (no pawns on d or f files). The total is strongly negative.

### Test 2: `isolated_pawn_penalized`
- **Setup**: Two positions compared:
  - Isolated: FEN `"4k3/8/8/8/8/8/4P3/4K3 w - - 0 1"` — lone White pawn on e2 with no pawns on d or f files.
  - Supported: FEN `"4k3/8/8/8/8/8/3PP3/4K3 w - - 0 1"` — White pawns on d2 and e2 (adjacent, so neither is isolated).
- **Verify**: The isolated position has a lower score (more negative or less positive) than the supported position.

### Test 3: `backward_pawn_penalized`
- **Setup**: FEN `"4k3/8/8/8/4p3/2P5/3P4/4K3 w - - 0 1"` — White pawns on c3 and d2. Black pawn on e4. The White d2 pawn is backward because: (a) the only adjacent-file friendly pawn is c3, which is on rank 3 — ahead of d2 on rank 2 — so no friendly pawn exists on adjacent files (c or e) at rank ≤ 2 to support the d-pawn's advance; (b) advancing d2→d3 places it on a square attacked by the Black pawn on e4 (a Black pawn on e4 attacks d3 and f3). The d2 pawn is not isolated (c3 is on adjacent c-file) and not passed (Black pawn on e4 covers d-file adjacent ranks ahead).
- **Comparison**: FEN `"4k3/8/8/8/4p3/2PP4/8/4K3 w - - 0 1"` — White pawns on c3 and d3 (d-pawn has caught up to its neighbor; no longer backward).
- **Verify**: The first position (with backward d2) scores lower than the second position (d3, no backward pawn).

### Test 4: `passed_pawn_bonus_scales_by_rank`
- **Setup**: Two FENs — one with a White passed pawn on d4 (rank 4, index 3) and one on d6 (rank 6, index 5). No enemy pawns that could block or contest.
  - Rank 4: `"4k3/8/8/8/3P4/8/8/4K3 w - - 0 1"`
  - Rank 6: `"4k3/8/3P4/8/8/8/8/4K3 w - - 0 1"`
- **Verify**: The d6 position returns higher mg and eg scores than the d4 position. Both pawns are also isolated, but since the isolated penalty is the same in both cases, the relative difference comes entirely from the passed pawn bonus scaling by rank.

### Test 5: `connected_pawns_bonus`
- **Setup**: Two FENs comparing connected vs separated pawns:
  - Connected: `"4k3/8/8/8/3PP3/8/8/4K3 w - - 0 1"` — White pawns on d4 and e4 (mutually defending via pawn attacks).
  - Separated: `"4k3/8/8/8/3P1P2/8/8/4K3 w - - 0 1"` — White pawns on d4 and f4 (not on adjacent files, cannot defend each other).
- **Verify**: The connected position scores higher than the separated position.

### Test 6: `both_colors_evaluated`
- **Setup**: FEN `"4k3/3p4/8/8/8/8/PPP5/4K3 w - - 0 1"` — White pawns on a2, b2, c2 (connected chain, no structural weaknesses). Black pawn on d7 (isolated — no Black pawns on c-file or e-file).
- **Verify**: `evaluate_pawns()` returns positive mg and eg values (White advantage), confirming that Black's isolated d7 pawn penalty is detected and subtracted, while White's connected chain receives bonuses.

### Test 7: `startpos_pawns_symmetric`
- **Setup**: Standard starting position `"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"`.
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