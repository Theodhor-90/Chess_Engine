# T02 — Piece-Square Tables with Tapered Evaluation

## Objective

Define separate middlegame and endgame piece-square tables for all six piece types (12 tables total). Extend the `evaluate` function to incorporate PST bonuses for each piece on the board, producing separate middlegame and endgame aggregate scores. Apply tapered evaluation to interpolate between the two scores using a game phase value.

## Deliverables

- `crates/eval/src/pst.rs` — piece-square table constant arrays for all 6 piece types (P, N, B, R, Q, K), each with separate middlegame and endgame tables (12 tables total)
- `evaluate` function extended to sum PST contributions per piece on the board
- Tapered interpolation logic between middlegame and endgame scores using formula: `score = ((mg_score * phase) + (eg_score * (max_phase - phase))) / max_phase`
- Until T03 is complete, this task may use a placeholder phase value for testing
- Unit tests confirming PST bonuses affect the evaluation (e.g., centralized knights score higher than rim knights)

## Implementation Details

- PST layout: constant `[i32; 64]` arrays indexed by `Square` using the project's little-endian rank-file mapping (a1=0, h8=63).
- Each piece type has a separate middlegame table and endgame table.
- The `evaluate` function iterates over each piece on the board, looks up its PST bonus from both middlegame and endgame tables, and accumulates separate mg/eg scores.
- Material values from T01 should be combined with PST bonuses (material counting is additive with PST).
- The tapered evaluation formula interpolates between mg and eg scores based on a phase value. The phase calculation itself is T03; this task implements the interpolation mechanism and may use a hardcoded/placeholder phase for testing.
- PST values should follow well-known chess programming conventions (e.g., PeSTO or similar proven tables). Tables should be defined from White's perspective and mirrored for Black.

## Dependencies

- Depends on T01 (Material Counting Evaluation) — this task extends the `evaluate` function and crate structure created in T01.
- Depends on M01 (`types` crate: `Square`, `PieceKind`, `Color`, `Bitboard`; `board` crate: `Position` with piece bitboard accessors, `piece_on()`).

## Verification

1. `cargo test -p eval` passes all unit tests including new PST-related tests.
2. Positions with centralized pieces score higher than positions with pieces on the rim (e.g., knight on d4 vs knight on a1).
3. The tapered interpolation correctly weights mg/eg scores (verifiable with explicit phase values in tests).
4. Material counting from T01 still works correctly — PST bonuses are additive, not replacing material values.
5. `cargo fmt --check` and `cargo clippy -- -D warnings` pass.
