# Phase P03 — Strength Measurement

## Goal

Establish automated infrastructure to measure engine Elo, A/B test changes with statistical rigor, and prevent strength regressions in CI — producing CuteChess-CLI tournament automation against at least 3 reference engines (Stockfish at reduced depth, Ethereal, Rustic) with Elo estimates at 95% confidence intervals of ±30 or narrower, an SPRT testing framework for A/B testing engine changes at bounds Elo0=0, Elo1=5, alpha=0.05, beta=0.05, and an automated CI regression test suite that runs a 100-game tournament after each PR and flags regressions where Elo drops by more than 30 at p<0.05.

## Design Decisions

1. **CuteChess-CLI as the tournament manager**: CuteChess-CLI is the external tool that drives engine-vs-engine matches via UCI. The integration layer consists of scripts and configuration that invoke CuteChess-CLI with appropriate arguments (engine paths, time controls, opening books, concurrency settings), then parse its PGN and log output to extract Elo estimates and confidence intervals.

2. **SPRT implemented as a wrapper around CuteChess-CLI**: Rather than building a custom game-playing framework, the SPRT testing tool configures CuteChess-CLI to run matches between a baseline engine and a test engine, then applies the Sequential Probability Ratio Test to the game results (using the pentanomial or trinomial model) to determine whether to accept, reject, or continue testing at the specified bounds.

3. **Reference engines downloaded during CI setup**: Stockfish (at reduced depth/nodes), Ethereal, and Rustic binaries are fetched during CI workflow setup steps rather than committed to the repository. This keeps the repository lightweight while ensuring reproducible tournament conditions.

4. **CI regression job as a GitHub Actions workflow**: The regression test suite is a GitHub Actions workflow triggered on pull requests. It builds the engine, downloads reference engines and CuteChess-CLI, runs a 100-game tournament, and uses the results to flag regressions where Elo drops by more than 30 at p<0.05.

5. **Tournament and SPRT tools placed under `tools/`**: Consistent with the master plan project structure and the placement decisions in P01 and P02, the tournament runner and SPRT testing scripts live under `tools/`, keeping them separate from the engine's library crates.

## Tasks

### T01 — CuteChess-CLI Integration

Build an automated tournament runner that invokes CuteChess-CLI to play matches between the engine and at least 3 reference engines (Stockfish at reduced depth, Ethereal, Rustic). The tool must configure time controls, opening books/positions, concurrency, and game counts, then parse tournament output to produce Elo estimates with 95% confidence intervals of ±30 or narrower from at least 500 games per engine pair.

**Deliverables:**
- A script or binary under `tools/` that configures and invokes CuteChess-CLI for engine-vs-engine tournaments
- Configuration for at least 3 reference engines: Stockfish at reduced depth, Ethereal, and Rustic
- Elo estimation with 95% confidence interval output parsed from tournament results
- Demonstration of ±30 or narrower confidence intervals from at least 500 games per engine pair

### T02 — SPRT Testing Framework

Build an SPRT testing tool that uses CuteChess-CLI to run matches between a baseline engine binary and a test engine binary, applies the Sequential Probability Ratio Test to game results, and reports whether to accept (change gains Elo), reject (change loses Elo), or continue testing. The SPRT bounds are Elo0=0, Elo1=5, alpha=0.05, beta=0.05.

**Deliverables:**
- A script or binary under `tools/` that runs SPRT tests between two engine binaries via CuteChess-CLI
- SPRT logic implementing the log-likelihood ratio calculation with bounds Elo0=0, Elo1=5, alpha=0.05, beta=0.05
- Output reporting accept/reject/continue status with current LLR and game count
- Verification that the framework can distinguish between a gaining and a non-gaining change

### T03 — Regression Test Suite

Create an automated CI job (GitHub Actions workflow) that runs a 100-game tournament after each pull request and flags a regression if Elo drops by more than 30 at p<0.05 significance.

**Deliverables:**
- A GitHub Actions workflow under `.github/workflows/` triggered on pull requests
- CI setup steps that build the engine, download CuteChess-CLI and reference engine binaries
- Tournament execution of 100 games with results parsed for Elo estimation
- Regression detection logic that flags the CI run if Elo drops by more than 30 at p<0.05
- PR status check reporting pass/fail based on regression detection

## Exit Criteria

1. CuteChess-CLI tournaments produce Elo estimates with 95% confidence interval of ±30 or narrower, from at least 500 games per engine pair against at least 3 reference engines (Stockfish at reduced depth, Ethereal, Rustic)
2. SPRT framework can confirm whether a code change gains or loses Elo using bounds Elo0=0, Elo1=5, alpha=0.05, beta=0.05
3. CI regression tests run a 100-game tournament after each PR and flag regressions where Elo drops by more than 30 at p<0.05
4. Engine plays at ~2200 Elo level with handcrafted evaluation

## Dependencies

- **Phase P01 (Test Infrastructure)** must be complete: the self-play framework provides game-playing infrastructure
- **Phase P02 (Parameter Tuning)** must be complete: the tuned evaluation and search parameters are what get measured for strength
- **External tools required**: CuteChess-CLI binary, reference engine binaries (Stockfish, Ethereal, Rustic)

## Artifacts

- `tools/tournament/` — script or binary for CuteChess-CLI tournament automation and Elo estimation
- `tools/sprt/` — script or binary for SPRT A/B testing of engine changes
- `.github/workflows/regression.yml` — CI workflow for post-PR regression testing
- CuteChess-CLI and reference engine configuration files (engine paths, time controls, opening sets)