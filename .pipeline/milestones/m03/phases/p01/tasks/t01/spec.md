# T01 — Material Counting Evaluation

## Objective

Create the `eval` crate and implement a material counting evaluation function that sums piece values for each side and returns a score relative to the side to move.

## Deliverables

- `crates/eval/` crate added to workspace `Cargo.toml` (workspace root `members` list updated)
- `crates/eval/Cargo.toml` — crate manifest with dependencies on `types` and `board` crates
- `crates/eval/src/lib.rs` — public evaluation API and module declarations
- `crates/eval/src/material.rs` — material value constants and material counting logic
- Material value constants: P=100, N=320, B=330, R=500, Q=900
- `evaluate(&Position) -> i32` function that computes material balance relative to side to move (positive = good for side to move)
- Unit tests confirming:
  - Positive scores for material advantage
  - Negative scores for material deficit
  - Zero for equal material

## Implementation Details

- The `eval` crate lives in `crates/eval/` and depends on `types` and `board` crates from M01. No dependency on `movegen`.
- Evaluation returns `i32` relative to side to move — standard convention for negamax search compatibility.
- Iterate over the position's piece bitboards to count material for each side.
- Use `Position` accessors like `piece_bitboard()`, `occupied_by()`, `side_to_move()` from the `board` crate.
- Material values should be defined as named constants in the `eval` crate, available for reuse by later tasks (phase calculation, etc.).

## Dependencies

- No dependencies on other tasks within this phase (this is the first task).
- Depends on M01 (`types` crate: `Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`) and M01 (`board` crate: `Position` with piece bitboard accessors and `side_to_move()`).

## Verification

1. `cargo build` succeeds with the new `eval` crate in the workspace.
2. `cargo test -p eval` passes all unit tests.
3. `evaluate` returns 0 for the starting position (equal material).
4. `evaluate` returns a positive score when the side to move has extra material.
5. `evaluate` returns a negative score when the side to move is down material.
6. `cargo fmt --check` and `cargo clippy -- -D warnings` pass.
