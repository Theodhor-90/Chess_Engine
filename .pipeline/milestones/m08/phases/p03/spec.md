## Phase P03 — Strength Measurement

### Goal

Establish automated infrastructure to measure engine Elo, A/B test changes with statistical rigor, and prevent strength regressions in CI. This phase produces the tournament and testing framework that validates the engine has reached ~2200 Elo with handcrafted evaluation.

### Deliverables

- **CuteChess-CLI integration** (T01): Automated tournament runner against at least 3 reference engines: Stockfish at reduced depth, Ethereal, and Rustic. Must produce Elo estimates with 95% confidence interval of ±30 or narrower from at least 500 games per engine pair.
- **SPRT testing framework** (T02): Statistical Sequential Probability Ratio Test for A/B testing engine changes. Determines if a change gains or loses Elo with SPRT bounds Elo0=0, Elo1=5, alpha=0.05, beta=0.05.
- **Regression test suite** (T03): Automated CI job (GitHub Actions) that runs a 100-game tournament after each PR and flags a regression if Elo drops by more than 30 at p<0.05.

### Technical Constraints

- CuteChess-CLI is the external tournament manager — the engine communicates with it via UCI protocol.
- Reference engines (Stockfish at reduced depth, Ethereal, Rustic) must be available as binaries in the CI environment or downloadable during CI setup.
- SPRT bounds are fixed at Elo0=0, Elo1=5, alpha=0.05, beta=0.05 for parameter validation; these provide ~95% statistical confidence.
- CI regression tests must complete within a reasonable time — 100-game tournaments with appropriate time controls.
- The regression threshold is Elo drop >30 at p<0.05 significance.
- CI runs on GitHub Actions — tournament scripts and configuration must integrate with `.github/workflows/`.
- The final exit criterion is engine playing at ~2200 Elo level with handcrafted evaluation.

### Dependencies

- **Phase P01 (Test Infrastructure)** must be complete: the self-play framework provides the game-playing infrastructure.
- **Phase P02 (Parameter Tuning)** must be complete: the tuned evaluation and search parameters are what get measured for strength.
- **External tools required**: CuteChess-CLI binary, reference engine binaries (Stockfish, Ethereal, Rustic), at least 1 million CCRL positions (carried from P02).