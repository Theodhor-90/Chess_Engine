I have all the information I need. Let me now produce the specification document.

# Milestone M08 — Testing, Tuning & Strength Validation

## Goal

Systematically verify engine correctness across movegen, tactics, and self-play, then optimize all evaluation and search parameters through Texel tuning and SPRT-validated self-play testing, and finally establish automated strength measurement infrastructure (CuteChess-CLI tournaments, SPRT framework, CI regression testing) to reach ~2200 Elo with the handcrafted evaluation.

## Scope

- Extended perft suite covering 100+ positions from established test suites to regression-proof the move generator
- Tactical test suites (WAC, STS, ECM) with solve-rate measurement at fixed depth/time
- Self-play framework with randomized openings to detect crashes, illegal moves, assertion failures, and time losses over thousands of games
- Texel tuning infrastructure minimizing MSE between predicted win probability and actual game outcomes
- Tuning of material values, PST values, and all evaluation weights via Texel method on a large dataset (CCRL or Lichess database positions)
- Search parameter tuning (LMR reduction table, null move reduction depth, futility margins, aspiration window sizes) validated via self-play SPRT testing
- CuteChess-CLI integration for automated tournaments against reference engines (Stockfish at reduced depth, Ethereal, Rustic, etc.)
- SPRT testing framework for A/B testing engine changes with statistical confidence
- Automated CI regression test suite that runs a short tournament after each PR to catch strength regressions

## Phases

### Phase P01 — Test Infrastructure

Build comprehensive test harnesses that validate engine correctness across move generation, tactical problem-solving, and sustained self-play.

- **T01**: Extended perft suite — validate movegen against 100+ positions from established test suites. Regression-proof the move generator.
- **T02**: Tactical test suite — WAC (Win At Chess), STS (Strategic Test Suite), ECM (Encyclopedia of Chess Middlegames). Measure solve rate at fixed depth/time.
- **T03**: Self-play framework — engine plays itself with randomized openings. Detect crashes, illegal moves, assertion failures, and time losses over thousands of games.

### Phase P02 — Parameter Tuning

Optimize evaluation and search parameters to maximize playing strength using data-driven tuning methods.

- **T01**: Texel tuning infrastructure — given a set of positions with game outcomes (win/draw/loss), optimize evaluation weights by minimizing MSE between predicted win probability and actual outcome.
- **T02**: Tune material values, PST values, and all evaluation weights using Texel method on a large dataset (e.g., CCRL games or Lichess database positions).
- **T03**: Search parameter tuning — LMR reduction table, null move reduction depth, futility margins, aspiration window sizes. Validate via self-play SPRT testing.

### Phase P03 — Strength Measurement

Establish automated infrastructure to measure engine Elo, A/B test changes with statistical rigor, and prevent strength regressions in CI.

- **T01**: CuteChess-CLI integration — automated tournament runner against reference engines (Stockfish at reduced depth, Ethereal, Rustic, etc.).
- **T02**: SPRT testing framework — Statistical Sequential Probability Ratio Test for A/B testing engine changes. Determine if a change gains or loses Elo with statistical confidence.
- **T03**: Regression test suite — automated CI job that runs a short tournament after each PR to catch strength regressions.

## Exit Criteria

1. Extended perft suite passes for 100+ positions, confirming movegen correctness
2. Tactical test suite solve rates are measured and tracked across versions
3. Self-play framework runs thousands of games without crashes, illegal moves, or assertion failures
4. Texel tuning produces optimized evaluation weights that measurably improve play
5. Search parameters are tuned and validated via SPRT testing
6. CuteChess-CLI tournaments produce reliable Elo estimates against reference engines
7. SPRT framework can confirm whether a code change gains or loses Elo with statistical confidence
8. CI regression tests catch strength regressions before merge
9. Engine plays at ~2200 Elo level with handcrafted evaluation

## Dependencies

- **M01–M07 complete** — the engine must have core types, move generation, search, UCI protocol, transposition table, advanced search techniques, and advanced evaluation all implemented before testing, tuning, and strength validation can proceed
- **External tools**: CuteChess-CLI for tournament management, reference engines (Stockfish, Ethereal, Rustic) for Elo measurement, a large game dataset (CCRL or Lichess) for Texel tuning positions