# T02 — King Centralization in Endgame

## Objective

Add an explicit king centralization bonus that increases in weight as material decreases, plus a king-to-passed-pawn proximity bonus for pawn endgames.

## Deliverables

- **King centralization bonus** based on **Chebyshev distance** (max of file distance and rank distance) from the king's square to the nearest of the **central 4 squares** (d4, d5, e4, e5). The bonus formula is `(3 - chebyshev_distance) * KING_CENTER_BONUS_WEIGHT`, clamped to a minimum of 0. `KING_CENTER_BONUS_WEIGHT` is a tunable constant with initial value of 10 centipawns per step closer. This bonus contributes **only to the endgame score component** so that tapered evaluation naturally increases its weight as material decreases.
- **King proximity to passed pawns bonus** in pure pawn endgames, also using Chebyshev distance. Fewer steps from the king to the passed pawn yields a higher bonus.
- **Unit tests** verifying:
  - A centralized king scores higher than a corner king in endgame positions
  - King proximity to passed pawns is rewarded in pawn endgames

## Files to Create or Modify

- **Create**: `crates/eval/src/king_centralization.rs` — new module containing endgame king centralization bonus and king-to-passed-pawn proximity bonus
- **Modify**: `crates/eval/src/lib.rs` — integrate king centralization into the evaluation pipeline

## Key Implementation Details

- **Chebyshev distance** = `max(|file_a - file_b|, |rank_a - rank_b|)`. This matches how a king actually moves (one step in any direction).
- The central 4 squares are d4, d5, e4, e5. Compute Chebyshev distance to the nearest of these four.
- The bonus contributes only to the EG (endgame) score component — it should have zero contribution to the MG (middlegame) score.
- The king-to-passed-pawn proximity bonus requires detecting passed pawns; this should leverage the passed pawn detection implemented in P01.
- Pure pawn endgame detection: only kings and pawns remain on the board.
- Use `Square`, `File`, `Rank`, `Color` types from the `types` crate.

## Dependencies

- **T01 (Specialized Endgame Evaluation)** must be complete — T01 establishes the endgame module and bypass logic; T02 adds king centralization for positions that don't trigger specialized endgame bypass
- Phase P01 (Pawn Structure) — passed pawn detection is needed for the king-to-passed-pawn proximity bonus
- Existing `eval` crate infrastructure: tapered evaluation framework, `compute_phase()`

## Verification Criteria

1. King centralization bonus is correctly computed using Chebyshev distance to the central 4 squares
2. The bonus contributes only to the endgame score component (not middlegame)
3. A centralized king produces a higher evaluation than a corner king in endgame positions
4. King proximity to passed pawns is rewarded in pawn endgames
5. The `KING_CENTER_BONUS_WEIGHT` constant is tunable (initial value: 10 centipawns per step)
6. Unit tests pass for centralization comparison and passed pawn proximity