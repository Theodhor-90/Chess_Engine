# T03 — Piece-Specific Positional Bonuses

## Objective

Implement piece-specific positional bonuses in the `eval` crate: bishop pair, rook on open/semi-open files, rook on seventh rank, and knight outpost detection. Each bonus produces separate midgame and endgame scores integrated into the tapered evaluation framework.

## Deliverables

### New File
- `crates/eval/src/pieces.rs` — piece-specific positional bonus evaluation

### Modified File
- `crates/eval/src/lib.rs` — integrate piece bonus scoring into the `evaluate()` function

## Key Implementation Details

### Bishop Pair Bonus
- Award a bonus when a side has **two or more bishops**
- The bishop pair is especially strong in open positions; produce separate midgame/endgame values

### Rook on Open File Bonus
- Award a bonus when a rook sits on a file with **no pawns at all** (neither friendly nor opponent)
- An open file gives the rook maximum vertical influence

### Rook on Semi-Open File Bonus
- Award a (smaller) bonus when a rook sits on a file with **no friendly pawns** but opponent pawns remain
- A semi-open file still provides good rook activity

### Rook on Seventh Rank Bonus
- Award a bonus when a rook reaches the **seventh rank** (rank 7 for white, rank 2 for black)
- Rooks on the seventh rank attack the opponent's unadvanced pawns and restrict the king

### Knight Outpost Detection
- A knight qualifies as an outpost when ALL of these conditions are met:
  1. The knight is on a square in **ranks 4–6** on the **opponent's half** of the board
  2. The knight is **supported by a friendly pawn** (diagonally behind it)
  3. The knight **cannot be attacked by any opponent pawn** (no opponent pawn can advance to attack the square)
- Award a bonus for each knight that meets all outpost criteria

### Integration
- All bonuses produce separate midgame and endgame scores
- Integrate into the `evaluate()` function in `crates/eval/src/lib.rs` alongside material, PST, pawn structure, king safety, and mobility terms

## Dependencies

- **T01 (King Safety)** and **T02 (Mobility)** — should be complete first; all three tasks modify `lib.rs` and this task finalizes the P02 integration
- **M07 Phase P01 (Pawn Structure)** — pawn position data is needed for rook open/semi-open file detection and knight outpost safety from opponent pawns
- **Existing `eval` crate** — tapered evaluation framework from M03
- **Existing `board` crate** — `Position` struct with piece bitboards
- **Existing `types` crate** — `Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`, `File`, `Rank`

## Verification Criteria

1. Unit test: **bishop pair** — position with two bishops gets bonus; position with one bishop does not
2. Unit test: **rook on open file** — rook on a file with no pawns gets bonus; rook on file with pawns does not
3. Unit test: **rook on semi-open file** — rook on a file with only opponent pawns gets the semi-open bonus
4. Unit test: **rook on seventh rank** — rook on rank 7 (white) / rank 2 (black) gets bonus
5. Unit test: **knight outpost** — knight on e5 supported by d4 pawn with no opponent pawn able to attack e5 gets outpost bonus; knight without pawn support does not
6. All bonuses produce separate midgame and endgame values
7. All new evaluation terms are properly integrated into `evaluate()` and combined via tapered eval
8. `cargo test` passes with all new tests
9. `cargo clippy -- -D warnings` passes
