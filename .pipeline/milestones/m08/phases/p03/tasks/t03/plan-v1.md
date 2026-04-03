Now I have all the context I need. Let me produce the implementation plan.

## 1. Deliverables

| File | Purpose |
|------|---------|
| `.github/workflows/regression.yml` | **Create** — GitHub Actions workflow triggered on pull requests that builds both the PR engine and the baseline (main branch) engine, downloads CuteChess-CLI and a reference engine binary, runs a 100-game tournament, computes the Elo difference, and flags the CI run as failed if Elo drops by more than 30 at p<0.05 |

## 2. Dependencies

- **CuteChess-CLI**: Downloaded during CI setup. The Linux build is available from the CuteChess GitHub releases as a pre-built binary or via `apt`/source compilation.
- **Reference engine binary**: At minimum one reference engine (Stockfish) downloaded during CI setup. Stockfish provides official Linux x86-64 binaries on GitHub releases.
- **Rust toolchain**: The `actions-rust-lang/setup-rust-toolchain` action (or `dtolnay/rust-toolchain`) to install the stable Rust toolchain in CI.
- **GitHub Actions runner**: `ubuntu-latest` provides the execution environment.
- **Existing project artifacts**: The `tools/tournament/` binary (T01) for running CuteChess-CLI tournaments, `tools/tournament/engines.toml` for reference engine configuration, and `tools/selfplay/openings.txt` for opening positions.

## 3. Implementation Details

### `.github/workflows/regression.yml`

**Purpose**: Automated CI job that detects strength regressions on pull requests by running a 100-game tournament between the PR engine and a baseline engine built from the `main` branch.

**Trigger**: `pull_request` event targeting the `main` branch.

**Jobs**:

A single job `regression-test` running on `ubuntu-latest` with these steps:

1. **Checkout PR code** — `actions/checkout@v4` to get the PR branch code.

2. **Setup Rust toolchain** — `dtolnay/rust-toolchain@stable` to install stable Rust.

3. **Build PR engine** — Run `cargo build --release -p engine` to build the PR branch engine binary. The binary lands at `target/release/engine`.

4. **Checkout main branch baseline** — Use `actions/checkout@v4` with `ref: main` into a separate path (e.g., `baseline-src`) to get the main branch source.

5. **Build baseline engine** — Run `cargo build --release -p engine --manifest-path baseline-src/Cargo.toml --target-dir baseline-target` to build the main branch engine. The binary lands at `baseline-target/release/engine`.

6. **Download CuteChess-CLI** — Fetch the CuteChess-CLI Linux binary from the CuteChess GitHub releases (or build from source via `apt-get install cutechess`). Place it at a known path, e.g., `./cutechess-cli`.

7. **Download Stockfish** — Fetch a Stockfish Linux binary from the official Stockfish GitHub releases. Place it at a known path, e.g., `./stockfish`. Make it executable.

8. **Run 100-game tournament** — Execute two 100-game tournaments using the T01 tournament runner or directly invoking `cutechess-cli`:
   - PR engine vs Stockfish (100 games)
   - Baseline engine vs Stockfish (100 games)
   
   Use CuteChess-CLI directly (rather than through the tournament tool) for simpler CI integration, with flags: `-engine name=PR cmd=target/release/engine -engine name=Stockfish cmd=./stockfish option.Threads=1 option.Hash=64 option."Skill Level"=5 -each proto=uci tc=5+0.05 -games 100 -rounds 50 -repeat -concurrency 1 -ratinginterval 0 -openingfile tools/selfplay/openings.txt`.
   
   Alternatively, run a single head-to-head tournament: PR engine vs Baseline engine (100 games, same time control). This is the more direct measurement — it tells us whether the PR is weaker than main. This is the preferred approach.

9. **Parse results and detect regression** — A shell script block that:
   - Parses the CuteChess-CLI `Score of` line to extract W-L-D counts
   - Computes the score percentage and Elo difference using the standard logistic formula: `Elo = -400 * log10(1/score - 1)`
   - Computes the standard error and 95% confidence interval
   - Checks whether the lower bound of the 95% CI is below -30 Elo (i.e., Elo drop > 30 at p<0.05)
   - If regression detected: prints a clear message and exits with non-zero status, causing the CI check to fail
   - If no regression: prints the results and exits with zero status

**Key logic for regression detection**:
- From W/D/L: `score = (W + D*0.5) / N`
- `elo = -400 * log10(1/score - 1)`
- `se = sqrt(score * (1 - score) / N)`
- Convert SE to Elo error margin for 95% CI: compute `elo_lower` from `score - 1.96*se`
- Regression flag: `elo_lower < -30` (i.e., the PR engine is statistically significantly weaker by more than 30 Elo)

**Note on the approach**: Running PR-vs-Baseline directly (rather than each vs a third engine) is cleaner — it eliminates confounding variance from the reference engine. However, if the two builds are very similar, 100 games will mostly produce draws with a near-zero Elo difference and a wide CI, so regressions of less than ~60 Elo won't be detectable with 100 games. This is expected per the task spec (the task says "flags a regression if Elo drops by more than 30 at p<0.05", which naturally requires a large enough signal to be detectable in 100 games).

## 4. API Contracts

N/A — This deliverable is a CI workflow file, not a library or binary with an API surface.

## 5. Test Plan

**What to test**: The workflow YAML is validated by GitHub Actions when it runs. The regression detection logic (shell arithmetic/inline script) is the critical component.

**Test 1 — Workflow triggers on pull requests**: Create a PR to verify the workflow appears in the PR's "Checks" tab. Verification: the workflow is listed and starts executing.

**Test 2 — Engines build successfully in CI**: Both the PR engine (`target/release/engine`) and the baseline engine (`baseline-target/release/engine`) must compile. Verification: the build steps succeed without errors.

**Test 3 — External tools download successfully**: CuteChess-CLI and Stockfish binaries are downloaded and made executable. Verification: the download steps succeed and the binaries are runnable (`./cutechess-cli --version`, `./stockfish quit` exit cleanly).

**Test 4 — Tournament runs to completion**: The 100-game tournament completes and produces output with a `Score of` line. Verification: the tournament step produces parseable output.

**Test 5 — Regression detection reports pass for non-regressing PR**: When the PR engine is identical or similar to baseline, the Elo difference should be near zero with a wide CI, and the workflow should pass. Verification: exit code 0.

**Test 6 — Regression detection reports fail for regressing engine**: If the PR engine were significantly weaker (e.g., by artificially handicapping it), the workflow should flag the regression and fail. Verification: exit code non-zero. (This is a conceptual verification — in practice, confirmed by the p<0.05 threshold logic being correct in the shell script.)

**Test 7 — PR status check reflects result**: The GitHub Actions workflow result (pass/fail) appears as a status check on the PR. Verification: the check is visible in the PR UI.

## 6. Implementation Order

1. **Create `.github/workflows/` directory** — Ensure the directory structure exists.

2. **Write `.github/workflows/regression.yml`** — The complete workflow file with all steps:
   - Workflow trigger configuration (`on: pull_request`)
   - Job definition (`regression-test` on `ubuntu-latest`)
   - Checkout steps (PR code and main branch baseline)
   - Rust toolchain setup
   - Engine build steps (PR and baseline)
   - CuteChess-CLI download step
   - Stockfish download step (used as a time control reference / fallback if head-to-head is insufficient)
   - Tournament execution step (PR engine vs baseline engine, 100 games)
   - Results parsing and regression detection step (inline shell script with Elo calculation and p<0.05 threshold check)

## 7. Verification Commands

```bash
# Verify the workflow file is valid YAML
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/regression.yml'))"

# Verify the workflow file exists in the expected location
ls -la .github/workflows/regression.yml

# Verify the workflow triggers on pull_request (grep for the trigger)
grep -A2 'on:' .github/workflows/regression.yml

# Verify the engine builds (this already works as part of the project)
cargo build --release -p engine

# Verify the tournament tool builds (dependency from T01)
cargo build --release -p tournament

# Dry-run: verify the inline regression detection script logic
# (Extract the shell script from the workflow and test with known W/D/L values)
# Example: 50 wins, 0 draws, 50 losses (score=0.5, Elo=0, should pass)
bash -c '
W=50; D=0; L=50
N=$((W + D + L))
SCORE=$(echo "scale=6; ($W + $D * 0.5) / $N" | bc)
echo "Score: $SCORE"
ELO=$(echo "scale=2; -400 * l(1/$SCORE - 1) / l(10)" | bc -l)
echo "Elo: $ELO"
'
```

## 8. Design Deviations

**Deviation 1: Head-to-head PR-vs-Baseline instead of PR-vs-reference-engines**

- **Parent spec requires**: "runs a 100-game tournament after each PR" and references the CuteChess-CLI integration from T01 which runs against reference engines (Stockfish, Ethereal, Rustic).
- **Why that approach is problematic**: Running against 3 reference engines with only 100 total games means ~33 games per engine pair, which gives extremely wide confidence intervals (±100+ Elo) making regression detection at 30 Elo impossible. Additionally, downloading 3 separate reference engines in CI adds complexity and failure points. The task spec says to "flag a regression if Elo drops by more than 30 at p<0.05" — this requires comparing the PR engine against the baseline engine directly.
- **Alternative chosen**: Run a single 100-game head-to-head tournament: PR engine vs Baseline engine (built from main branch). This directly measures whether the PR is weaker than the current baseline, which is exactly what regression testing requires. Stockfish is still downloaded as a fallback reference engine in case head-to-head produces inconclusive results, but the primary test is PR-vs-Baseline.

**Deviation 2: Inline shell regression detection instead of invoking the SPRT tool**

- **Parent spec requires**: "regression detection logic may leverage the SPRT framework" (T02).
- **Why that approach is problematic**: The SPRT tool (T02) runs iterative batched matches with accept/reject/continue semantics — it is designed for open-ended testing that may run thousands of games. The CI regression test runs exactly 100 games and checks a simple Elo threshold. Using SPRT with Elo0=0, Elo1=30 bounds on only 100 games would almost always return "inconclusive" since 100 games is too few for SPRT to reach a decision at these bounds. The task spec requires "flags a regression if Elo drops by more than 30 at p<0.05" — this is a one-shot frequentist test, not a sequential test.
- **Alternative chosen**: Inline shell script that computes the Elo difference and 95% confidence interval from the 100-game W/D/L results using the standard logistic Elo formula and checks whether the lower bound of the CI drops below -30. This directly implements the "Elo drops by more than 30 at p<0.05" criterion.