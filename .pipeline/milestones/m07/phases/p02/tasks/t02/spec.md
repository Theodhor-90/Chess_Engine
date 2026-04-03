# T02 — Mobility Evaluation

## Objective

Implement per-piece mobility evaluation in the `eval` crate. Count pseudo-legal moves for each piece (knights, bishops, rooks, queens), excluding squares attacked by opponent pawns, and apply piece-type-dependent scaling to produce separate midgame and endgame mobility scores.

## Deliverables

### New File
- `crates/eval/src/mobility.rs` — per-piece mobility counting and scoring

### Modified File
- `crates/eval/src/lib.rs` — integrate mobility scoring into the `evaluate()` function

## Key Implementation Details

### Mobility Counting
- For each piece on the board (knights, bishops, rooks, queens), count the number of pseudo-legal moves available
- **Exclude squares attacked by opponent pawns** from the move count — this prevents inflating mobility scores with moves into danger zones
- Use existing attack generation infrastructure (magic bitboards for sliding pieces, pre-computed tables for knights)

### Piece-Type Scaling
- **Knights and bishops**: highest per-move bonus — these pieces benefit most from mobility
- **Rooks**: moderate per-move bonus
- **Queens**: light per-move bonus — to avoid the engine overvaluing early queen wandering

### Score Output
- Produce separate **midgame and endgame mobility scores** for each color
- These scores feed into the existing tapered evaluation framework via `compute_phase()` interpolation

### Integration
- The mobility evaluation function must be callable from the main `evaluate()` function in `crates/eval/src/lib.rs`
- Mobility scores are added to the overall evaluation alongside material, PST, pawn structure (from P01), and king safety (from T01)

## Dependencies

- **T01 (King Safety Evaluation)** — should be complete first since both modify `lib.rs` and king safety establishes patterns for integrating new eval terms
- **Existing `eval` crate** — tapered evaluation framework from M03
- **Existing `board` crate** — `Position` struct with piece bitboards
- **Existing `types` crate** — `Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`
- **Existing `movegen` crate** — attack generation (magic bitboards for bishops/rooks, knight attack tables)

## Verification Criteria

1. Unit tests verify **correct mobility counts** for known positions (e.g., starting position mobility for each piece, open position with high mobility)
2. Unit tests verify that **squares attacked by opponent pawns are excluded** from mobility count
3. Unit tests verify **piece-type-dependent scaling** (queen mobility score is lower per move than knight/bishop)
4. Unit tests verify **score contributions** for positions with high vs low mobility
5. Mobility scores produce separate midgame and endgame values
6. `cargo test` passes with all new tests
7. `cargo clippy -- -D warnings` passes
