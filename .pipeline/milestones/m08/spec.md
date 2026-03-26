## Milestone M08 — Testing, Tuning & Strength Validation

### Goal

Systematic verification and parameter optimization. Target: ~2200 Elo (classical evaluation ceiling).

### Phases

#### Phase P01 — Test Infrastructure
- **T01**: Extended perft suite — validate movegen against 100+ positions from established test suites. Regression-proof the move generator.
- **T02**: Tactical test suite — WAC (Win At Chess), STS (Strategic Test Suite), ECM (Encyclopedia of Chess Middlegames). Measure solve rate at fixed depth/time.
- **T03**: Self-play framework — engine plays itself with randomized openings. Detect crashes, illegal moves, assertion failures, and time losses over thousands of games.

#### Phase P02 — Parameter Tuning
- **T01**: Texel tuning infrastructure — given a set of positions with game outcomes (win/draw/loss), optimize evaluation weights by minimizing MSE between predicted win probability and actual outcome.
- **T02**: Tune material values, PST values, and all evaluation weights using Texel method on a large dataset (e.g., CCRL games or Lichess database positions).
- **T03**: Search parameter tuning — LMR reduction table, null move reduction depth, futility margins, aspiration window sizes. Validate via self-play SPRT testing.

#### Phase P03 — Strength Measurement
- **T01**: CuteChess-CLI integration — automated tournament runner against reference engines (Stockfish at reduced depth, Ethereal, Rustic, etc.).
- **T02**: SPRT testing framework — Statistical Sequential Probability Ratio Test for A/B testing engine changes. Determine if a change gains or loses Elo with statistical confidence.
- **T03**: Regression test suite — automated CI job that runs a short tournament after each PR to catch strength regressions.

### Exit Criteria
- Extended perft suite passes for 100+ positions, confirming movegen correctness
- Tactical test suite solve rates are measured and tracked across versions
- Self-play framework runs thousands of games without crashes, illegal moves, or assertion failures
- Texel tuning produces optimized evaluation weights that measurably improve play
- Search parameters are tuned and validated via SPRT testing
- CuteChess-CLI tournaments produce reliable Elo estimates against reference engines
- SPRT framework can confirm whether a code change gains or loses Elo with statistical confidence
- CI regression tests catch strength regressions before merge
- Engine plays at ~2200 Elo level with handcrafted evaluation
- All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`