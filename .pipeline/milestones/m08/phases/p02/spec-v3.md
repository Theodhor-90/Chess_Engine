# Phase P02 — Parameter Tuning

## Goal

Optimize all evaluation and search parameters to maximize playing strength using data-driven methods: build a Texel tuning infrastructure that minimizes MSE between predicted win probability and actual game outcomes, tune material values, PST values, and all evaluation weights on at least 1 million quiet positions from the CCRL game database, then tune search parameters (LMR reduction table, null move reduction depth, futility margins, aspiration window sizes) with each change validated individually via self-play SPRT testing.

## Design Decisions

1. **Texel tuner as a standalone tool binary**: The tuner lives under `tools/` as a separate binary, consistent with the project structure in the master plan. It links against the `eval` crate to access evaluation functions directly (calling them in-process) rather than going through UCI, since Texel tuning requires millions of evaluations per iteration and UCI overhead would be prohibitive.

2. **CCRL game database as the position source**: The milestone spec mandates at least 1 million quiet positions sourced from the CCRL game database. Positions are filtered to exclude those in check, with captures available, or otherwise tactically sharp — this ensures the tuner optimizes positional judgment rather than tactical artifacts.

3. **Sigmoid scaling constant K tuned to the dataset**: The Texel method minimizes MSE between `sigmoid(eval * K)` and the game result (1.0, 0.5, 0.0). The constant K must be optimized for the specific dataset before tuning evaluation weights, as an incorrect K distorts the loss landscape.

4. **Search parameters validated individually via SPRT**: The milestone spec prohibits bulk untested parameter changes. Each search parameter change (LMR table, null move depth, futility margins, aspiration windows) is validated independently using the self-play framework from P01/T03 with SPRT bounds Elo0=0, Elo1=5, alpha=0.05, beta=0.05.

5. **Tuned weights written back to source code**: Optimized evaluation weights replace the existing constants in the `eval` crate source files. This keeps the engine self-contained — no external weight files needed for the handcrafted evaluation.

## Tasks

### T01 — Texel Tuning Infrastructure

Build the core Texel tuning framework: a tool binary that loads a dataset of quiet positions with game outcomes, evaluates each position using the engine's evaluation function, computes MSE between `sigmoid(eval * K)` and the actual outcome, and optimizes evaluation weights to minimize this loss. The tool must support tuning the scaling constant K to fit the dataset.

**Deliverables:**
- A binary under `tools/` that implements the Texel tuning algorithm
- Dataset loading — parse positions (FEN) with associated game outcomes (1.0 / 0.5 / 0.0)
- K-constant optimization via line search or gradient descent on the dataset
- Iterative weight optimization loop that adjusts evaluation parameters to minimize MSE
- Output of optimized weights as Rust `const` declarations (matching the existing constant names and types in the `eval` crate) printed to stdout, so they can be pasted directly into the eval crate source files

### T02 — Evaluation Weight Tuning

Using the Texel tuner from T01, tune material values, PST values, and all evaluation weights on at least 1 million quiet positions sourced from the CCRL game database. Confirm that tuned weights gain at least +20 Elo over untuned weights via SPRT testing at Elo0=0, Elo1=10, alpha=0.05, beta=0.05.

**Deliverables:**
- At least 1 million quiet positions extracted and filtered from the CCRL game database, stored in a format the tuner can consume
- Tuned material values, PST values, and all evaluation weights applied to the `eval` crate
- SPRT confirmation that tuned weights gain at least +20 Elo over untuned weights (Elo0=0, Elo1=10, alpha=0.05, beta=0.05)

### T03 — Search Parameter Tuning

Tune LMR reduction table values, null move reduction depth, futility margins, and aspiration window sizes. Each parameter change is validated individually via self-play SPRT testing with bounds Elo0=0, Elo1=5, alpha=0.05, beta=0.05. No bulk untested parameter changes.

**Deliverables:**
- Tuned LMR reduction table applied to the `search` crate
- Tuned null move reduction depth applied to the `search` crate
- Tuned futility margins applied to the `search` crate
- Tuned aspiration window sizes applied to the `search` crate
- SPRT test results for each individual parameter change confirming non-regression (Elo0=0, Elo1=5, alpha=0.05, beta=0.05)

## Exit Criteria

1. Texel tuning infrastructure produces optimized evaluation weights that gain at least +20 Elo over untuned weights, confirmed by SPRT at Elo0=0, Elo1=10, alpha=0.05, beta=0.05
2. Material values, PST values, and all evaluation weights are tuned on at least 1 million quiet positions from the CCRL game database
3. Search parameters (LMR reduction table, null move reduction depth, futility margins, aspiration window sizes) are tuned and each change validated individually via SPRT testing at Elo0=0, Elo1=5, alpha=0.05, beta=0.05

## Dependencies

- **Phase P01 (Test Infrastructure)** must be complete: the self-play framework from P01/T03 is required to run SPRT validation of search parameter changes and to confirm Elo gains from tuned evaluation weights
- **M01–M07 complete**: The engine must have all evaluation terms (material, PST, pawn structure, king safety, mobility, endgame) and all search features (LMR, null move pruning, futility pruning, aspiration windows) implemented before they can be tuned
- **External data**: At least 1 million positions from the CCRL game database for Texel tuning

## Artifacts

- `tools/texel/` — Texel tuning binary
- CCRL-derived dataset of 1 million+ quiet positions with game outcomes
- Updated evaluation weights in `crates/eval/` source files
- Updated search parameters in `crates/search/` source files
- SPRT test results documenting Elo gains from tuned weights and individual search parameter changes