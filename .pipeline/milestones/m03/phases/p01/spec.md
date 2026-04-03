## Phase P01 — Material Evaluation & Piece-Square Tables

### Goal

Build the evaluation foundation for the chess engine: a material-counting evaluator combined with piece-square tables and tapered evaluation that smoothly interpolates between middlegame and endgame scoring. This phase produces the static evaluation function that the search (Phase P02) will call at leaf nodes.

### Deliverables

- **T01 — Material counting evaluation**: Sum piece values for each side using standard values (P=100, N=320, B=330, R=500, Q=900). Return the score relative to the side to move.
- **T02 — Piece-square tables with tapered evaluation**: Implement separate middlegame and endgame piece-square tables for all six piece types. Tapered evaluation interpolates between the two table sets based on a game-phase score derived from remaining material.
- **T03 — Game phase calculation**: Compute a phase score from the remaining non-pawn, non-king material on the board. Use this score to weight the middlegame vs endgame PST contributions in the tapered evaluation.

### Technical Constraints

- The `eval` crate should depend only on `types` and `board` crates from M01.
- Score must be returned relative to the side to move (positive = good for side to move).
- Standard piece values: P=100, N=320, B=330, R=500, Q=900.
- PST values should be defined as constant arrays indexed by `Square` (a1=0, h8=63 little-endian rank-file mapping per project conventions).
- Tapered evaluation formula: `score = ((mg_score * phase) + (eg_score * (max_phase - phase))) / max_phase`.
- No `unwrap()` in library code; use `Result`/`Option` propagation.
- All public items must have `///` doc comments.

### Dependencies

- **M01 (Core Types & Board Representation)**: Requires `types` crate (`Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`, `Move`, `File`, `Rank`) and `board` crate (`Position` with piece bitboards, side-to-move access).
- **M02 (Move Generation)**: Requires `movegen` crate (`generate_legal_moves`) — needed for integration testing but not for the evaluation logic itself.
- No dependencies on other M03 phases.