# T03 — Tapered Evaluation Integration

## Objective

Verify and balance all evaluation terms introduced across M07 (phases P01, P02, and P03) within the tapered evaluation framework. This is a verification and balancing pass — it does not introduce new evaluation features.

## Deliverables

- **MG/EG score verification**: Confirm that all evaluation terms from P01 (pawn hash table, doubled/isolated/backward/passed/connected pawns, passed pawn king proximity) and P02 (king safety/pawn shield/open files/attack zone, mobility per piece type, bishop pair/rook on open file/rook on seventh/knight outpost) contribute separate MG and EG scores combined via `compute_phase()` interpolation.
- **Phase calculation verification**: Verify that the phase calculation in `crates/eval/src/phase.rs` correctly weights the middlegame-to-endgame transition across all terms.
- **Weight balance verification**: Evaluate the startpos and the following 5 quiet middlegame FEN positions, and confirm that no single non-material evaluation term (king safety, mobility, pawn structure, piece bonuses) exceeds 50% of the total non-material evaluation contribution in any of these positions:
  - `r1bqkb1r/pppppppp/2n2n2/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3` (Italian-style quiet opening)
  - `r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4` (Italian Game mainline)
  - `rnbqkb1r/pp2pppp/2p2n2/3p4/2PP4/5N2/PP2PPPP/RNBQKB1R w KQkq - 0 4` (Slav Defense)
  - `rnbqk2r/ppppbppp/4pn2/8/2PP4/5N2/PP2PPPP/RNBQKB1R w KQkq - 2 4` (Queen's Gambit Declined)
  - `r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3` (Petrov-adjacent quiet)
- **King safety constraint**: In each of the quiet positions above, the king safety term must contribute less than the material term in absolute value.
- **Weight adjustments** as needed to satisfy the above constraints.
- **Phase interpolation tests**: Tests confirming correct score interpolation across a range of game phases (opening, middlegame, endgame material configurations) — at least one test per phase region using representative FEN positions.

## Files to Modify

- **Modify**: `crates/eval/src/lib.rs` — weight adjustments across all M07 evaluation terms as needed for balance
- **Modify**: `crates/eval/src/phase.rs` — if phase calculation adjustments are needed
- Potentially modify weight constants in any eval module files from P01 and P02

## Key Implementation Details

- This task is a **verification and balancing pass**, not a feature implementation task
- The 50% dominance threshold means: for each of the 6 reference positions (startpos + 5 FENs), compute the absolute contribution of each non-material eval term, sum them, and verify no single term exceeds 50% of that sum
- King safety < material (in absolute value) is checked separately for each reference position
- Weight adjustments should be conservative — only change values that violate the constraints
- The `compute_phase()` function in `crates/eval/src/phase.rs` performs MG/EG interpolation using remaining material to compute a phase value

## Dependencies

- **T01 (Specialized Endgame Evaluation)** must be complete
- **T02 (King Centralization in Endgame)** must be complete
- **Phase P01 (Pawn Structure)** must be complete — its terms are being verified
- **Phase P02 (King Safety & Piece Activity)** must be complete — its terms are being verified

## Verification Criteria

1. All M07 evaluation terms contribute separate MG and EG scores combined via `compute_phase()` interpolation
2. Phase calculation in `crates/eval/src/phase.rs` correctly weights the MG-to-EG transition
3. In startpos and all 5 quiet middlegame reference positions, no single non-material evaluation term exceeds 50% of total non-material evaluation contribution
4. In all reference positions, king safety term contributes less than material term in absolute value
5. Tests confirm correct tapered interpolation across opening, middlegame, and endgame phase regions
6. All existing tests continue to pass after any weight adjustments