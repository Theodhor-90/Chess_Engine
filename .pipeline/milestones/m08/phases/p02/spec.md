## Phase P02 — Parameter Tuning

### Goal

Optimize evaluation and search parameters to maximize playing strength using data-driven tuning methods. This phase takes the verified-correct engine from P01 and systematically improves its evaluation weights and search parameters through Texel tuning and SPRT-validated self-play.

### Deliverables

- **Texel tuning infrastructure** (T01): Given a set of positions with game outcomes (win/draw/loss), optimize evaluation weights by minimizing MSE between predicted win probability and actual outcome. This is the core optimization framework.
- **Evaluation weight tuning** (T02): Tune material values, PST values, and all evaluation weights using the Texel method on at least 1 million quiet positions with game outcomes sourced from the CCRL game database.
- **Search parameter tuning** (T03): Tune LMR reduction table, null move reduction depth, futility margins, and aspiration window sizes. Validate each parameter change via self-play SPRT testing with bounds Elo0=0, Elo1=5, alpha=0.05, beta=0.05.

### Technical Constraints

- Texel tuning requires at least 1 million quiet positions — positions must be filtered to exclude positions in check, captures available, or otherwise tactically sharp.
- Position data must be sourced from the CCRL game database with associated game outcomes (1-0, 0.5-0.5, 0-1).
- The Texel optimizer minimizes MSE between sigmoid(eval * K) and the game result, where K is a scaling constant tuned to the dataset.
- Search parameter changes must be validated individually via SPRT self-play testing (Elo0=0, Elo1=5, alpha=0.05, beta=0.05) — no bulk untested parameter changes.
- Tuned evaluation weights must gain at least +20 Elo over untuned weights, confirmed by SPRT at Elo0=0, Elo1=10, alpha=0.05, beta=0.05.
- The tuner should be a separate tool binary under `tools/` per the project structure.
- Rust conventions apply: minimize external dependencies, `thiserror`/`anyhow` for errors, no `unwrap()` in library crates.

### Dependencies

- **Phase P01 (Test Infrastructure)** must be complete: the self-play framework from P01/T03 is needed to run SPRT validation of search parameter changes, and the tactical test suites provide complementary strength signals.