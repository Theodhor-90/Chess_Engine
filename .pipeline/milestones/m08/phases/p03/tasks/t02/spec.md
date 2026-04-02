# T02 — SPRT Testing Framework

## Objective

Build an SPRT testing tool that uses CuteChess-CLI to run matches between a baseline engine binary and a test engine binary, applies the Sequential Probability Ratio Test to game results, and reports whether to accept (change gains Elo), reject (change loses Elo), or continue testing — with SPRT bounds Elo0=0, Elo1=5, alpha=0.05, beta=0.05.

## Deliverables

- A script or binary under `tools/sprt/` that runs SPRT tests between two engine binaries via CuteChess-CLI
- SPRT logic implementing the log-likelihood ratio (LLR) calculation with bounds Elo0=0, Elo1=5, alpha=0.05, beta=0.05
- Output reporting accept/reject/continue status with current LLR and game count
- Verification that the framework can distinguish between a gaining and a non-gaining change

## Key Implementation Details

- Rather than building a custom game-playing framework, the SPRT testing tool is a wrapper around CuteChess-CLI that configures it to run matches between the baseline and test engine
- Apply the Sequential Probability Ratio Test to game results using the pentanomial or trinomial model
- SPRT bounds: Elo0=0 (null hypothesis: no gain), Elo1=5 (alternative hypothesis: gains 5 Elo), alpha=0.05 (Type I error rate), beta=0.05 (Type II error rate)
- Decision thresholds: lower bound = ln(beta / (1 - alpha)), upper bound = ln((1 - beta) / alpha)
- The tool should be able to run incrementally (batches of games) and report status after each batch
- Use the CuteChess-CLI integration from T01 for match execution

## Files to Create or Modify

- `tools/sprt/` — new directory containing the SPRT testing script/binary

## Dependencies

- **T01 (CuteChess-CLI Integration)** must be complete: the SPRT tool relies on the tournament infrastructure to run matches between engine binaries
- External tools required: CuteChess-CLI binary

## Verification Criteria

1. The SPRT tool accepts two engine binary paths and runs matches between them via CuteChess-CLI
2. LLR is calculated correctly from game results using the specified bounds (Elo0=0, Elo1=5, alpha=0.05, beta=0.05)
3. The tool reports accept/reject/continue status with current LLR value and game count
4. The framework can distinguish between a gaining change and a non-gaining change (e.g., testing the current engine against itself should not trigger an accept)
5. Output is clear and machine-parseable for integration into CI workflows