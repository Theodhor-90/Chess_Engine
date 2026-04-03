# T02 — Evaluation Weight Tuning

## Objective

Using the Texel tuner from T01, tune material values, PST values, and all evaluation weights on at least 1 million quiet positions sourced from the CCRL game database. Confirm that tuned weights gain at least +20 Elo over untuned weights via SPRT testing.

## Deliverables

- At least 1 million quiet positions extracted and filtered from the CCRL game database, stored in a format the tuner can consume
- Tuned material values, PST values, and all evaluation weights applied to the `eval` crate source files (replacing existing constants)
- SPRT confirmation that tuned weights gain at least +20 Elo over untuned weights (Elo0=0, Elo1=10, alpha=0.05, beta=0.05)

## Key Implementation Details

- Source data: CCRL game database (PGN format) — positions must be extracted, filtered for quiescence (no checks, no captures available, not tactically sharp), and stored with game outcomes (1.0 / 0.5 / 0.0)
- At least 1 million positions must be in the final dataset
- The dataset format must be compatible with the Texel tuner built in T01
- Run the Texel tuner on the full dataset to optimize all evaluation weights
- Tuned weights are written back into `crates/eval/` source files as Rust `const` declarations, replacing the existing untuned values
- The engine remains self-contained — no external weight files needed for the handcrafted evaluation
- SPRT validation uses the self-play framework from Phase P01/T03, comparing tuned vs untuned engine at SPRT bounds Elo0=0, Elo1=10, alpha=0.05, beta=0.05

## Files to Create or Modify

- Dataset file(s) under a data directory (e.g., `data/ccrl_positions.txt` or similar) — the filtered quiet positions with outcomes
- A position extraction/filtering script or tool to process CCRL PGN data into the tuner's input format
- `crates/eval/src/*.rs` — updated evaluation weight constants (material values, PST tables, pawn structure weights, king safety weights, mobility weights, endgame weights)

## Dependencies

- **T01 (Texel Tuning Infrastructure)** must be complete — the tuner binary is required to run optimization
- **Phase P01/T03 (Self-play framework)** must be complete — required for SPRT validation of Elo gain
- **External data**: CCRL game database (PGN files)

## Verification Criteria

1. Dataset contains at least 1 million quiet positions with game outcomes
2. Texel tuner runs successfully on the full dataset and produces optimized weights
3. Optimized weights are applied to `crates/eval/` source files
4. `cargo build`, `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt --check` all pass with the new weights
5. SPRT test confirms tuned weights gain at least +20 Elo over untuned weights (Elo0=0, Elo1=10, alpha=0.05, beta=0.05)
6. No regressions in existing test suites (perft, tactical tests)