# T01 — King Safety Evaluation

## Objective

Implement king safety evaluation in the `eval` crate, producing separate midgame and endgame scores that integrate with the existing tapered evaluation framework. The king safety evaluation has three components: pawn shield scoring, open file penalties near the king, and virtual mobility of attacking pieces toward the king zone.

## Deliverables

### New File
- `crates/eval/src/king_safety.rs` — king safety evaluation module

### Modified File
- `crates/eval/src/lib.rs` — integrate king safety scoring into the `evaluate()` function

## Key Implementation Details

### King Zone Definition
The king zone comprises the 8 squares surrounding the king plus 3 squares in front of those (11 squares total, from the opponent's perspective). This captures the area where attacking pieces threaten king safety.

### Pawn Shield Evaluation
- Award bonus for pawns on the **three files around the castled king** (king file and its two adjacent files)
- Scale the bonus by **rank distance from the king** — pawns closer to the king provide better shelter, pawns further advanced provide less
- Produce separate midgame and endgame scores

### Open File Penalty Near King
- Penalize when files adjacent to the king have **no friendly pawns**
- An "open file near king" means a file within the king's file neighborhood that lacks friendly pawn coverage

### Virtual Mobility of Attacking Pieces Toward King Zone
- Evaluate the attacking potential of opponent pieces toward the king zone
- Count or weight opponent piece attacks that target squares in the king zone
- Score the aggregate attacking pressure

### Integration
- All scores must be returned as separate midgame and endgame values for use with the existing `compute_phase()` interpolation in `crates/eval/src/phase.rs`
- The function must be callable from the main `evaluate()` function in `crates/eval/src/lib.rs`

## Dependencies

- **M07 Phase P01 (Pawn Structure)** must be complete — pawn position data is needed for pawn shield and open file detection
- **Existing `eval` crate** — tapered evaluation framework from M03 (`evaluate()`, `compute_phase()`, `mg_table`/`eg_table`)
- **Existing `board` crate** — `Position` struct with piece bitboards, `is_square_attacked`
- **Existing `types` crate** — `Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`, `File`, `Rank`

## Verification Criteria

1. Unit tests verify correct scoring for positions with **strong pawn shields** (e.g., fianchetto king behind intact shield) vs **weak pawn shields** (e.g., missing g-pawn, advanced h-pawn)
2. Unit tests verify **open file penalties** near the king (e.g., king on g1 with no pawn on g-file or h-file)
3. Unit tests verify **attacking piece pressure** toward king zone (e.g., rook/queen aimed at open file near king scores higher danger)
4. King safety scores produce separate midgame and endgame values
5. `cargo test` passes with all new tests
6. `cargo clippy -- -D warnings` passes
