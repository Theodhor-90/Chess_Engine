# T03 — Regression Test Suite

## Objective

Create an automated CI job (GitHub Actions workflow) that runs a 100-game tournament after each pull request and flags a regression if Elo drops by more than 30 at p<0.05 significance.

## Deliverables

- A GitHub Actions workflow under `.github/workflows/` triggered on pull requests
- CI setup steps that build the engine, download CuteChess-CLI and reference engine binaries
- Tournament execution of 100 games with results parsed for Elo estimation
- Regression detection logic that flags the CI run if Elo drops by more than 30 at p<0.05
- PR status check reporting pass/fail based on regression detection

## Key Implementation Details

- The CI regression job is a GitHub Actions workflow triggered on pull requests
- Build steps: (1) checkout code, (2) build the engine from the PR branch, (3) build or download the baseline engine from main branch, (4) download CuteChess-CLI binary, (5) download reference engine binaries
- Reference engines (Stockfish, Ethereal, Rustic) are fetched during CI workflow setup steps rather than committed to the repository
- Run a 100-game tournament using the CuteChess-CLI integration from T01
- Parse results and compute Elo difference between the PR engine and baseline
- Regression threshold: flag as regression if Elo drops by more than 30 at p<0.05 significance
- Report pass/fail as a GitHub PR status check
- Keep the workflow fast enough to be practical for PR review (100 games is a reasonable balance between signal and speed)

## Files to Create or Modify

- `.github/workflows/regression.yml` — new CI workflow file for post-PR regression testing
- May reference or invoke scripts from `tools/tournament/` (T01) and `tools/sprt/` (T02)

## Dependencies

- **T01 (CuteChess-CLI Integration)** must be complete: the CI job uses the tournament runner to execute matches and parse results
- **T02 (SPRT Testing Framework)** must be complete: the regression detection logic may leverage the SPRT framework for statistical significance testing
- External tools required: CuteChess-CLI binary, reference engine binaries, GitHub Actions runner

## Verification Criteria

1. The GitHub Actions workflow triggers on pull requests
2. CI successfully builds both the PR engine and the baseline (main branch) engine
3. CuteChess-CLI and reference engines are downloaded during CI setup
4. A 100-game tournament runs to completion within the CI environment
5. Results are parsed and Elo estimation is computed
6. The workflow correctly flags a regression when Elo drops by more than 30 at p<0.05
7. The workflow passes when there is no significant regression
8. PR status check reflects the pass/fail result of regression detection