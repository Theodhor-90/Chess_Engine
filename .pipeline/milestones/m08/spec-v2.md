# Milestone M08 — Testing, Tuning & Strength Validation

## Goal

Systematically verify engine correctness across movegen, tactics, and self-play, then optimize all evaluation and search parameters through Texel tuning and SPRT-validated self-play testing, and finally establish automated strength measurement infrastructure (CuteChess-CLI tournaments, SPRT framework, CI regression testing) to reach ~2200 Elo with the handcrafted evaluation.

## Scope

- Extended perft suite covering 100+ positions from established test suites to regression-proof the move generator
- Tactical test suites (WAC, STS, ECM) with solve-rate measurement at fixed depth/time
- Self-play framework with randomized openings to detect crashes, illegal moves, assertion failures, and time losses over at least 2,000 games
- Texel tuning infrastructure minimizing MSE between predicted win probability and actual game outcomes
- Tuning of material values, PST values, and all evaluation weights via Texel method on at least 1 million quiet positions with game outcomes sourced from the CCRL game database
- Search parameter tuning (LMR reduction table, null move reduction depth, futility margins, aspiration window sizes) validated via self-play SPRT testing
- CuteChess-CLI integration for automated tournaments against at least 3 reference engines: Stockfish at reduced depth, Ethereal, and Rustic
- SPRT testing framework for A/B testing engine changes with bounds Elo0=0, Elo1=5, alpha=0.05, beta=0.05
- Automated CI regression test suite that runs a 100-game tournament after each PR to catch strength regressions

## Phases

### Phase P01 — Test Infrastructure

Build comprehensive test harnesses that validate engine correctness across move generation, tactical problem-solving, and sustained self-play.

- **T01**: Extended perft suite — validate movegen against 100+ positions from established test suites. Regression-proof the move generator.
- **T02**: Tactical test suite — WAC (Win At Chess), STS (Strategic Test Suite), ECM (Encyclopedia of Chess Middlegames). Measure solve rate at fixed depth/time.
- **T03**: Self-play framework — engine plays itself with randomized openings. Detect crashes, illegal moves, assertion failures, and time losses over at least 2,000 games.

### Phase P02 — Parameter Tuning

Optimize evaluation and search parameters to maximize playing strength using data-driven tuning methods.

- **T01**: Texel tuning infrastructure — given a set of positions with game outcomes (win/draw/loss), optimize evaluation weights by minimizing MSE between predicted win probability and actual outcome.
- **T02**: Tune material values, PST values, and all evaluation weights using Texel method on at least 1 million quiet positions with game outcomes sourced from the CCRL game database.
- **T03**: Search parameter tuning — LMR reduction table, null move reduction depth, futility margins, aspiration window sizes. Validate via self-play SPRT testing (Elo0=0, Elo1=5, alpha=0.05, beta=0.05).

### Phase P03 — Strength Measurement

Establish automated infrastructure to measure engine Elo, A/B test changes with statistical rigor, and prevent strength regressions in CI.

- **T01**: CuteChess-CLI integration — automated tournament runner against at least 3 reference engines: Stockfish at reduced depth, Ethereal, and Rustic.
- **T02**: SPRT testing framework — Statistical Sequential Probability Ratio Test for A/B testing engine changes. Determine if a change gains or loses Elo with SPRT bounds Elo0=0, Elo1=5, alpha=0.05, beta=0.05.
- **T03**: Regression test suite — automated CI job that runs a 100-game tournament after each PR and flags a regression if Elo drops by more than 30 at p<0.05.

## Exit Criteria

1. Extended perft suite passes for 100+ positions, confirming movegen correctness
2. Tactical test suite solve rates are measured and tracked across versions
3. Self-play framework runs at least 2,000 games without crashes, illegal moves, or assertion failures
4. Texel tuning produces optimized evaluation weights that gain at least +20 Elo over untuned weights, confirmed by SPRT at Elo0=0, Elo1=10, alpha=0.05, beta=0.05
5. Search parameters are tuned and validated via SPRT testing (Elo0=0, Elo1=5, alpha=0.05, beta=0.05)
6. CuteChess-CLI tournaments produce Elo estimates with 95% confidence interval of ±30 or narrower, from at least 500 games per engine pair
7. SPRT framework can confirm whether a code change gains or loses Elo using bounds Elo0=0, Elo1=5, alpha=0.05, beta=0.05
8. CI regression tests run a 100-game tournament after each PR and flag regressions where Elo drops by more than 30 at p<0.05
9. Engine plays at ~2200 Elo level with handcrafted evaluation

## Dependencies

- **M01–M07 complete** — the engine must have core types, move generation, search, UCI protocol, transposition table, advanced search techniques, and advanced evaluation all implemented before testing, tuning, and strength validation can proceed
- **External tools**: CuteChess-CLI for tournament management, reference engines (Stockfish, Ethereal, Rustic) for Elo measurement, at least 1 million positions from the CCRL game database for Texel tuning