# Phase P03 — Endgame Knowledge

## Goal

Implement specialized endgame evaluation routines that detect and score common mating patterns (KBN vs K, KR vs K, KQ vs K) and drawn endgames from insufficient material, add explicit king centralization bonuses for endgame play, and perform a final integration and weight-balancing pass to ensure all evaluation terms introduced across M07 (phases P01, P02, and P03) contribute correctly to the tapered evaluation framework via `compute_phase()` in `crates/eval/src/phase.rs`.

## Design Decisions

1. **Specialized endgame bypass**: When a recognized endgame pattern is detected (e.g., KR vs K), the specialized evaluator returns a score directly, bypassing the general `evaluate()` pipeline. This avoids noise from irrelevant terms (pawn structure, mobility) in material configurations where they are meaningless.

2. **KBN vs K corner mapping**: The KBN vs K evaluator must drive the losing king toward the correct corner — the corner whose color matches the bishop's square color. This requires identifying the bishop's square color and using a distance metric to the correct corner pair (a1/h8 for dark-square bishop, a8/h1 for light-square bishop).

3. **King centralization distance metric**: King centralization uses **Chebyshev distance** (max of file distance and rank distance) from the king's square to the nearest square in the **central 4 squares** (d4, d5, e4, e5). Chebyshev distance matches how a king actually moves (one step in any direction), making it the natural choice. The bonus equals `(3 - chebyshev_distance) * KING_CENTER_BONUS_WEIGHT`, clamped to a minimum of 0, where `KING_CENTER_BONUS_WEIGHT` is a tunable constant (initial value: 10 centipawns per step closer). This term contributes only to the endgame score component so that tapered evaluation naturally increases its weight as material decreases. An additional proximity bonus in pawn endgames rewards the king for being near passed pawns (also using Chebyshev distance).

4. **Integration task scope**: T03 is a verification and balancing pass — it does not introduce new evaluation features. It ensures all MG/EG score pairs from P01 (pawn structure, passed pawns) and P02 (king safety, mobility, piece bonuses) flow through the existing `compute_phase()` interpolation correctly. Weight balance is verified by evaluating a defined set of quiet reference positions and confirming that no single non-material evaluation term exceeds 50% of the total non-material evaluation contribution (see T03 deliverables and exit criteria for specifics).

## Tasks

### T01 — Specialized Endgame Evaluation

Implement detection and scoring for mating endgames and drawn endgames due to insufficient material.

**Deliverables:**
- Material configuration detection that identifies KBN vs K, KR vs K, KQ vs K, and insufficient material cases (K vs K, KB vs K, KN vs K, KNN vs K)
- KBN vs K evaluator that drives the losing king toward the correct corner based on bishop square color
- KR vs K and KQ vs K evaluators that drive the losing king toward the edge
- Drawn endgame detection returning a score of 0 (draw) for insufficient material
- When a specialized endgame is detected, the general evaluation is bypassed and the endgame-specific score is returned
- Unit tests for each endgame pattern verifying correct detection and score direction on known FEN positions

### T02 — King Centralization in Endgame

Add an explicit king centralization bonus that increases in weight as material decreases.

**Deliverables:**
- King centralization bonus based on **Chebyshev distance** from the king's square to the nearest of the **central 4 squares** (d4, d5, e4, e5). The bonus is `(3 - chebyshev_distance) * KING_CENTER_BONUS_WEIGHT` (clamped to minimum 0), contributing only to the endgame score component so tapered evaluation controls its weight
- Additional bonus in pure pawn endgames for king proximity to passed pawns, also using Chebyshev distance (fewer steps to the passed pawn yields a higher bonus)
- Unit tests verifying that a centralized king scores higher than a corner king in endgame positions, and that king proximity to passed pawns is rewarded in pawn endgames

### T03 — Tapered Evaluation Integration

Verify and balance all evaluation terms introduced across M07 within the tapered evaluation framework.

**Deliverables:**
- Verification that all evaluation terms from P01 (pawn hash table, doubled/isolated/backward/passed/connected pawns, passed pawn king proximity) and P02 (king safety/pawn shield/open files/attack zone, mobility per piece type, bishop pair/rook on open file/rook on seventh/knight outpost) contribute separate MG and EG scores combined via `compute_phase()` interpolation
- Verification that the phase calculation in `crates/eval/src/phase.rs` correctly weights the middlegame-to-endgame transition across all terms
- Weight balance verification: evaluate the startpos and the following 5 quiet middlegame FEN positions, and confirm that no single non-material evaluation term (king safety, mobility, pawn structure, piece bonuses) exceeds 50% of the total non-material evaluation contribution in any of these positions:
  - `r1bqkb1r/pppppppp/2n2n2/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3` (Italian-style quiet opening)
  - `r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4` (Italian Game mainline)
  - `rnbqkb1r/pp2pppp/2p2n2/3p4/2PP4/5N2/PP2PPPP/RNBQKB1R w KQkq - 0 4` (Slav Defense)
  - `rnbqk2r/ppppbppp/4pn2/8/2PP4/5N2/PP2PPPP/RNBQKB1R w KQkq - 2 4` (Queen's Gambit Declined)
  - `r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3` (Petrov-adjacent quiet)
- Additionally, in each of these quiet positions, the king safety term must contribute less than the material term in absolute value
- Weight adjustments as needed to satisfy these constraints
- Tests confirming correct score interpolation across a range of game phases (opening, middlegame, endgame material configurations) — at least one test per phase region using representative FEN positions

## Exit Criteria

1. Specialized endgame evaluation detects and scores KBN vs K (driving king to correct corner by bishop color), KR vs K (driving king to edge), and KQ vs K (driving king to edge) mating patterns, with unit tests on known FEN positions
2. Drawn endgames from insufficient material (K vs K, KB vs K, KN vs K, KNN vs K) are detected and return a draw score, with unit tests confirming detection
3. King centralization bonus is computed using Chebyshev distance to the central 4 squares (d4, d5, e4, e5), contributes to the endgame score component, and increases in weight as material decreases via tapered evaluation
4. King proximity to passed pawns is rewarded in pawn endgames using Chebyshev distance
5. All new evaluation terms from P01 and P02 contribute separate middlegame and endgame scores combined via the existing `compute_phase()` interpolation in `crates/eval/src/phase.rs`
6. In the startpos and 5 quiet middlegame reference positions (specified in T03), no single non-material evaluation term exceeds 50% of the total non-material evaluation contribution, and the king safety term contributes less than the material term in absolute value
7. Tests confirm correct tapered interpolation across opening, middlegame, and endgame phase regions using representative FEN positions

## Dependencies

- **Phase P01 (Pawn Structure)** — must be complete; T03 integrates pawn structure terms (doubled, isolated, backward, passed, connected pawns) and the pawn hash table
- **Phase P02 (King Safety & Piece Activity)** — must be complete; T03 integrates king safety, mobility, and piece bonus terms
- **`eval` crate** — `evaluate()` in `crates/eval/src/lib.rs`, `compute_phase()` in `crates/eval/src/phase.rs`, tapered evaluation framework from M03, pawn evaluation from P01, king safety/mobility/piece bonuses from P02
- **`board` crate** — `Position` struct with piece bitboards for material detection, Zobrist hashing
- **`types` crate** — `Bitboard`, `Square`, `Piece`, `PieceKind`, `Color` types with iteration and bitwise operations

## Artifacts

- `crates/eval/src/endgame.rs` — new module containing specialized endgame detection, mating pattern evaluation (KBN vs K, KR vs K, KQ vs K), and insufficient material detection
- `crates/eval/src/king_centralization.rs` — new module containing endgame king centralization bonus and king-to-passed-pawn proximity bonus
- `crates/eval/src/lib.rs` — modified to integrate endgame bypass logic (calling specialized endgame evaluation before general evaluation) and king centralization into the evaluation pipeline; weight adjustments across all M07 terms