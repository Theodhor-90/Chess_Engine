# T01 — Texel Tuning Infrastructure

## Objective

Build the core Texel tuning framework as a standalone tool binary that loads a dataset of quiet positions with game outcomes, evaluates each position using the engine's evaluation function, computes MSE between `sigmoid(eval * K)` and the actual outcome, and optimizes evaluation weights to minimize this loss.

## Deliverables

- A binary under `tools/texel/` that implements the Texel tuning algorithm
- Dataset loading — parse positions (FEN) with associated game outcomes (1.0 / 0.5 / 0.0)
- K-constant optimization via line search or gradient descent on the dataset before tuning begins
- Iterative weight optimization loop that adjusts evaluation parameters to minimize MSE
- Output of optimized weights as Rust `const` declarations (matching the existing constant names and types in the `eval` crate) printed to stdout, so they can be pasted directly into the eval crate source files

## Key Implementation Details

- The tuner lives under `tools/` as a separate binary, consistent with the project structure (`tools/` for utility binaries)
- Links against the `eval` crate to access evaluation functions directly (in-process), not through UCI — Texel tuning requires millions of evaluations per iteration and UCI overhead would be prohibitive
- The Texel method minimizes MSE between `sigmoid(eval * K)` and the game result (1.0 for white win, 0.5 for draw, 0.0 for white loss)
- The scaling constant K must be optimized for the specific dataset before tuning evaluation weights, as an incorrect K distorts the loss landscape
- Positions should be filtered to exclude those in check, with captures available, or otherwise tactically sharp — to optimize positional judgment rather than tactical artifacts

## Files to Create or Modify

- `tools/texel/` — new directory for the Texel tuning binary
- `tools/texel/main.rs` (or `tools/texel/src/main.rs`) — the tuner binary entry point
- `Cargo.toml` — workspace root updated to include the new binary/crate

## Dependencies

- No dependencies on other tasks in this phase (this is the foundation for T02 and T03)
- Requires the `eval` crate from M07 to be complete with all evaluation terms (material, PST, pawn structure, king safety, mobility, endgame)
- Requires the `board` crate for FEN parsing (`Position::from_fen`)

## Verification Criteria

1. `cargo build` succeeds with the new `tools/texel` binary included in the workspace
2. The binary can load a sample dataset of FEN positions with game outcomes
3. K-constant optimization runs and converges to a stable value on the sample dataset
4. The optimization loop iterates and reduces MSE measurably over multiple iterations on the sample dataset
5. Output format matches existing `eval` crate constant declarations (valid Rust `const` syntax with correct names and types)
6. `cargo clippy -- -D warnings` and `cargo fmt --check` pass
7. `cargo test` passes (all existing tests remain green)