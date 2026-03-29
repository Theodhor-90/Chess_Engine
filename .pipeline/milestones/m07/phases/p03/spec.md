## Phase P03 — Endgame Knowledge

### Goal

Implement specialized endgame evaluation, king centralization bonuses, and ensure all new evaluation terms from P01 and P02 integrate cleanly into the tapered evaluation framework. This final phase completes the M07 advanced evaluation milestone.

### Deliverables

- **Specialized endgame evaluation** (T01): Detect and evaluate KBN vs K (drive king to correct corner based on bishop color), KR vs K (drive king to edge), and KQ vs K (drive king to edge) mating patterns. Detect drawn endgames due to insufficient material (K vs K, KB vs K, KN vs K, KNN vs K). When a specialized endgame is detected, bypass the general evaluation and return the endgame-specific score.
- **King centralization in endgame** (T02): Bonus for king advancing toward center squares as material decreases. Use the endgame component of tapered evaluation so the bonus is weighted toward low-material positions. Additional bonus in pure pawn endgames for king proximity to passed pawns.
- **Tapered evaluation integration** (T03): Ensure all new evaluation terms from P01 (pawn structure, passed pawns) and P02 (king safety, mobility, piece bonuses) contribute separate middlegame and endgame scores that are combined via the existing `compute_phase()` interpolation in `crates/eval/src/phase.rs`. Verify correct phase weighting across the transition. Adjust evaluation term weights for balance — no single term should dominate; king safety should not outweigh material for quiet positions.

### Technical Decisions & Constraints

- Specialized endgame evaluations bypass the general evaluation function entirely when the endgame pattern is detected.
- KBN vs K must drive the king toward the correct corner based on bishop color (a complex mating pattern).
- Insufficient material detection covers: K vs K, KB vs K, KN vs K, KNN vs K.
- King centralization uses the endgame component of tapered eval so it naturally increases in weight as material decreases.
- The integration task (T03) is a verification and balancing pass across all evaluation terms — it must ensure no regression in existing evaluation quality while properly weighting new terms.
- The overall milestone exit criterion is at least 1950 Elo measured by a 500+ game gauntlet against reference engines using CuteChess-CLI at 10+0.1 time control.

### Dependencies

- **Phase P01 (Pawn Structure)** and **Phase P02 (King Safety & Piece Activity)** must both be complete — T03 of this phase performs the final integration and balancing of all terms introduced in those phases.
- Depends on the existing `eval` crate (`compute_phase()` in `crates/eval/src/phase.rs`, tapered evaluation framework from M03).
- Depends on the `board` crate (`Position` struct, piece bitboards for material detection).
- Depends on the `types` crate for bitboard operations.