# T01 — Specialized Endgame Evaluation

## Objective

Implement detection and scoring for mating endgames (KBN vs K, KR vs K, KQ vs K) and drawn endgames due to insufficient material, bypassing the general evaluation pipeline when a recognized pattern is detected.

## Deliverables

- **Material configuration detection** that identifies:
  - Mating patterns: KBN vs K, KR vs K, KQ vs K
  - Insufficient material (drawn): K vs K, KB vs K, KN vs K, KNN vs K
- **KBN vs K evaluator**: Drives the losing king toward the correct corner based on bishop square color. The correct corner is the one whose color matches the bishop's square color (a1/h8 for dark-square bishop, a8/h1 for light-square bishop). Uses a distance metric to the correct corner pair.
- **KR vs K evaluator**: Drives the losing king toward the edge of the board.
- **KQ vs K evaluator**: Drives the losing king toward the edge of the board.
- **Drawn endgame detection**: Returns a score of 0 (draw) for insufficient material cases.
- **General evaluation bypass**: When a specialized endgame is detected, the general `evaluate()` pipeline is bypassed and the endgame-specific score is returned directly. This avoids noise from irrelevant terms (pawn structure, mobility) in material configurations where they are meaningless.
- **Unit tests** for each endgame pattern verifying correct detection and score direction on known FEN positions.

## Files to Create or Modify

- **Create**: `crates/eval/src/endgame.rs` — new module containing specialized endgame detection, mating pattern evaluation, and insufficient material detection
- **Modify**: `crates/eval/src/lib.rs` — integrate endgame bypass logic (call specialized endgame evaluation before general evaluation)

## Key Implementation Details

- Material configuration detection should inspect piece bitboards from the `Position` struct to count pieces per side
- KBN vs K requires identifying the bishop's square color to determine the correct corner
- The winning side's score should be large and positive (material advantage + bonus for driving the losing king to the correct squares)
- Use `Bitboard`, `Square`, `PieceKind`, `Color` types from the `types` crate
- The `Position` struct and piece bitboards are in the `board` crate

## Dependencies

- Phase P01 (Pawn Structure) and P02 (King Safety & Piece Activity) must be complete
- Depends on existing `eval` crate infrastructure: `evaluate()` in `crates/eval/src/lib.rs`, tapered evaluation framework from M03
- Depends on `board` crate: `Position` struct with piece bitboards
- Depends on `types` crate: `Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`
- No dependencies on other tasks within this phase

## Verification Criteria

1. KBN vs K detection correctly identifies the pattern for both sides and drives the losing king to the correct corner based on bishop color
2. KR vs K and KQ vs K detection correctly identifies the pattern and drives the losing king to the edge
3. Insufficient material cases (K vs K, KB vs K, KN vs K, KNN vs K) are detected and return a draw score of 0
4. When a specialized endgame is detected, the general evaluation is bypassed
5. Unit tests pass for each endgame pattern on known FEN positions, verifying both detection and correct score direction