# T03 — Game Phase Calculation

## Objective

Implement game phase calculation that computes a phase score from the remaining non-pawn, non-king material on the board. Wire this into the tapered evaluation so that positions with full material weight toward middlegame PST values and positions with depleted material weight toward endgame PST values.

## Deliverables

- `crates/eval/src/phase.rs` — game phase calculation module
- Phase score calculation function based on remaining non-pawn, non-king material
- Maximum phase value constant representing the starting material configuration (all non-pawn, non-king pieces present)
- Integration with tapered evaluation from T02 — replace any placeholder phase value with the real computed phase
- Unit tests confirming:
  - Starting position yields full middlegame weighting (phase equals max_phase)
  - King-and-pawns-only endgame yields full endgame weighting (phase equals 0)
  - Intermediate positions interpolate correctly (phase between 0 and max_phase)

## Implementation Details

- The phase score is derived from non-pawn, non-king material remaining on the board.
- Use the material value constants defined in T01 (N=320, B=330, R=500, Q=900) to compute the phase contribution of each piece.
- Maximum phase value = total non-pawn, non-king material in the starting position (4×N + 4×B + 4×R + 2×Q = 4×320 + 4×330 + 4×500 + 2×900 = 6400, or whichever formula the implementation uses).
- The phase feeds into the tapered evaluation formula from T02: `score = ((mg_score * phase) + (eg_score * (max_phase - phase))) / max_phase`.
- Clamp phase to [0, max_phase] to handle positions with promoted pieces that could exceed starting material.

## Dependencies

- Depends on T01 (Material Counting Evaluation) — uses material value constants.
- Depends on T02 (Piece-Square Tables with Tapered Evaluation) — integrates with the tapered evaluation interpolation logic.
- Depends on M01 (`types` crate: `Bitboard`, `PieceKind`, `Color`; `board` crate: `Position` with piece bitboard accessors).

## Verification

1. `cargo test -p eval` passes all unit tests including phase calculation tests.
2. Starting position: phase equals max_phase, evaluation heavily weights middlegame PST values.
3. King-and-pawns endgame: phase equals 0, evaluation uses only endgame PST values.
4. Removing a queen from the starting position produces an intermediate phase value.
5. The full evaluation pipeline (material + PST + tapered via computed phase) produces coherent scores.
6. `cargo fmt --check` and `cargo clippy -- -D warnings` pass.
