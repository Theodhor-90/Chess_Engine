Cargo isn't available in this shell but that's a local environment issue — the plan's verification is about the CI environment where Rust is explicitly installed. The YAML is valid, all awk tests pass, and the file structure is correct.

- **`.github/workflows/regression.yml`** — **Created**. GitHub Actions workflow with:
  - Trigger: `pull_request` targeting `main`
  - Runner pinned to `ubuntu-22.04` (Qt5 availability comment included)
  - Steps: checkout PR → setup Rust → cache Cargo → build PR engine → checkout main baseline → build baseline engine → build CuteChess-CLI v1.3.1 from source → run 100-game head-to-head tournament (50 rounds × 2 games, `tc=1+0.01`, openings from `tools/selfplay/openings.txt`) → parse results with inline shell/awk script (Elo + 95% CI, regression if elo_upper < -30)