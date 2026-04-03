# Phase P01 — Material Evaluation & Piece-Square Tables

## Goal

Build the static evaluation function for the chess engine in a new `eval` crate: a material-counting evaluator combined with piece-square tables and tapered evaluation that smoothly interpolates between middlegame and endgame scoring based on a game-phase score derived from remaining material. This phase produces the `evaluate` function that the search (Phase P02) will call at leaf nodes.

## Design Decisions

1. **New `eval` crate**: The evaluation logic lives in `crates/eval/`, added to the workspace. It depends on `types` and `board` crates from M01. No dependency on `movegen` (that is only needed for integration testing).

2. **Score representation**: Evaluation returns an `i32` score relative to the side to move (positive = good for side to move). This is the standard convention for negamax search compatibility.

3. **PST layout**: Piece-square tables are defined as constant `[i32; 64]` arrays indexed by `Square` using the project's little-endian rank-file mapping (a1=0, h8=63). Each piece type has a separate middlegame table and endgame table (12 tables total for 6 piece types × 2 phases).

4. **Tapered evaluation formula**: `score = ((mg_score * phase) + (eg_score * (max_phase - phase))) / max_phase`. The phase score is derived from non-pawn, non-king material remaining on the board, with a defined maximum phase value representing the starting material.

5. **Material values as constants**: Standard piece values (P=100, N=320, B=330, R=500, Q=900) are defined as named constants in the `eval` crate, available for use by the phase calculation and material counting.

## Tasks

### T01 — Material Counting Evaluation

Create the `eval` crate with workspace integration. Implement a material counting evaluation function that sums piece values for each side using standard values (P=100, N=320, B=330, R=500, Q=900) by iterating over the position's piece bitboards. Return the score relative to the side to move.

**Deliverables:**
- `crates/eval/` crate added to workspace `Cargo.toml`
- Material value constants
- `evaluate` function that computes material balance relative to side to move
- Unit tests confirming positive scores for material advantage, negative for deficit, zero for equal material

### T02 — Piece-Square Tables with Tapered Evaluation

Define separate middlegame and endgame piece-square tables for all six piece types (12 tables total). Extend the `evaluate` function to incorporate PST bonuses for each piece on the board, producing separate middlegame and endgame aggregate scores. Apply tapered evaluation to interpolate between the two scores using the game phase (implemented in T03). Until T03 is complete, this task may use a placeholder phase value for testing.

**Deliverables:**
- Middlegame and endgame PST constant arrays for P, N, B, R, Q, K
- `evaluate` function extended to sum PST contributions per piece
- Tapered interpolation logic between middlegame and endgame scores
- Unit tests confirming PST bonuses affect the evaluation (e.g., centralized knights score higher than rim knights)

### T03 — Game Phase Calculation

Implement game phase calculation that computes a phase score from the remaining non-pawn, non-king material on the board. Wire this into the tapered evaluation so that positions with full material weight toward middlegame PST values and positions with depleted material weight toward endgame PST values.

**Deliverables:**
- Phase score calculation function based on remaining material
- Maximum phase value constant representing the starting material configuration
- Integration with tapered evaluation from T02
- Unit tests confirming: starting position yields full middlegame weighting, king-and-pawns endgame yields full endgame weighting, intermediate positions interpolate correctly

## Exit Criteria

1. Evaluation function returns positive scores for material advantage and negative scores for material deficit, with magnitude proportional to the material difference.
2. Evaluation incorporates piece-square table bonuses interpolated between middlegame and endgame via tapered evaluation.
3. Game phase calculation correctly weights middlegame tables at full material and endgame tables at depleted material.

## Dependencies

- **M01 (Core Types & Board Representation)**: `types` crate provides `Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`. `board` crate provides `Position` with `piece_bitboard()`, `occupied_by()`, `side_to_move()`, and `piece_on()` accessors.
- **M02 (Move Generation)**: `movegen` crate provides `generate_legal_moves` — needed for integration testing but not for evaluation logic itself.
- No dependencies on other M03 phases.

## Artifacts

- `crates/eval/Cargo.toml` — crate manifest depending on `types` and `board`
- `crates/eval/src/lib.rs` — public evaluation API and module declarations
- `crates/eval/src/material.rs` — material value constants and material counting logic
- `crates/eval/src/pst.rs` — piece-square table constant arrays (middlegame and endgame)
- `crates/eval/src/phase.rs` — game phase calculation
- `Cargo.toml` (workspace root) — updated to include `crates/eval` in workspace members