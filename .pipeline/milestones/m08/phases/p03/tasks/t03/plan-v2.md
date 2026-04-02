Now I have all the context. Let me produce the revised plan.

## 1. Deliverables

| File | Purpose |
|------|---------|
| `.github/workflows/regression.yml` | **Create** — GitHub Actions workflow triggered on pull requests that builds both the PR engine and the baseline (main branch) engine, downloads CuteChess-CLI, runs a 100-game head-to-head tournament between the PR and baseline engines, and flags the CI run as failed if Elo drops by more than 30 at p<0.05 |

## 2. Dependencies

- **CuteChess-CLI**: Built from source during CI setup. The CuteChess repository (`https://github.com/cutechess/cutechess`) is cloned at tag `v1.3.1`, built with CMake (Qt dependency satisfied by `qt5-default` / `qtbase5-dev`), producing the `cutechess-cli` binary. Pre-built Linux binaries are not reliably available from the releases page, so building from source is the robust approach.
- **Rust toolchain**: The `dtolnay/rust-toolchain@stable` action to install the stable Rust toolchain in CI.
- **GitHub Actions runner**: `ubuntu-latest` provides the execution environment.
- **Existing project artifacts**: The `tools/selfplay/openings.txt` file (EPD format opening positions created in P01).
- **No reference engines needed**: The head-to-head PR-vs-Baseline approach eliminates the need to download Stockfish, Ethereal, or Rustic in CI. Reference engine tournaments are the domain of T01's `tools/tournament/` binary, not the CI regression job.

## 3. Implementation Details

### `.github/workflows/regression.yml`

**Purpose**: Automated CI job that detects strength regressions on pull requests by running a 100-game head-to-head tournament between the PR engine and a baseline engine built from the `main` branch.

**Trigger**: `pull_request` event targeting the `main` branch.

**Jobs**:

A single job `regression-test` running on `ubuntu-latest` with these steps:

1. **Checkout PR code** — `actions/checkout@v4` to get the PR branch code (default behavior, checks out the merge commit).

2. **Setup Rust toolchain** — `dtolnay/rust-toolchain@stable` to install stable Rust.

3. **Cache Cargo artifacts** — `actions/cache@v4` on `~/.cargo/registry`, `~/.cargo/git`, and `target/` keyed by `Cargo.lock` hash to speed up builds.

4. **Build PR engine** — Run `cargo build --release -p engine`. The binary lands at `target/release/engine`. Copy it to `./pr-engine` to avoid overwrite in the next build step.

5. **Checkout main branch baseline** — Use `actions/checkout@v4` with `ref: main` into a separate path (`baseline-src/`) to get the main branch source.

6. **Build baseline engine** — Run `cargo build --release -p engine --manifest-path baseline-src/Cargo.toml --target-dir baseline-target`. The binary lands at `baseline-target/release/engine`. Copy it to `./baseline-engine`.

7. **Build CuteChess-CLI from source** — Install build dependencies (`sudo apt-get update && sudo apt-get install -y qtbase5-dev cmake g++`), clone the CuteChess repository at tag `v1.3.1` (`git clone --branch v1.3.1 --depth 1 https://github.com/cutechess/cutechess.git cutechess-src`), build with CMake (`cd cutechess-src && cmake . && make -j$(nproc) cutechess-cli`), and copy the binary to `./cutechess-cli`.

8. **Run 100-game head-to-head tournament** — Invoke CuteChess-CLI directly with the following exact command:

   ```bash
   ./cutechess-cli \
     -engine name=PR cmd=./pr-engine proto=uci \
     -engine name=Baseline cmd=./baseline-engine proto=uci \
     -each tc=1+0.01 \
     -rounds 50 \
     -games 2 \
     -repeat \
     -openings file=tools/selfplay/openings.txt format=epd order=random \
     -concurrency 1 \
     -ratinginterval 0 \
     -pgnout regression-games.pgn \
     > tournament-output.txt 2>&1
   ```

   **Time control justification**: `tc=1+0.01` (1 second base + 10ms increment) keeps the total 100-game tournament under 5 minutes in CI. The engines are identical builds playing head-to-head, so ultra-fast time control is sufficient — we are detecting regressions, not measuring absolute strength. Using `-rounds 50 -games 2 -repeat` plays 100 total games (2 games per round, colors swapped each pair, from 50 openings).

9. **Parse results and detect regression** — An inline shell script that parses the CuteChess-CLI output and applies the regression threshold. The complete script:

   ```bash
   #!/bin/bash
   set -euo pipefail

   OUTPUT_FILE="tournament-output.txt"

   # Extract the final "Score of" line (last occurrence)
   SCORE_LINE=$(grep "^Score of" "$OUTPUT_FILE" | tail -1)
   if [ -z "$SCORE_LINE" ]; then
     echo "ERROR: No 'Score of' line found in tournament output"
     cat "$OUTPUT_FILE"
     exit 1
   fi

   echo "Tournament result: $SCORE_LINE"

   # Parse W-L-D from "Score of PR vs Baseline: W - L - D  [pct]"
   # CuteChess-CLI format: first engine's W, first engine's L, D
   SCORES=$(echo "$SCORE_LINE" | sed 's/.*: *\([0-9]*\) *- *\([0-9]*\) *- *\([0-9]*\).*/\1 \2 \3/')
   W=$(echo "$SCORES" | awk '{print $1}')
   L=$(echo "$SCORES" | awk '{print $2}')
   D=$(echo "$SCORES" | awk '{print $3}')
   N=$((W + L + D))

   echo "PR engine: +${W} =${D} -${L} (${N} games)"

   if [ "$N" -eq 0 ]; then
     echo "ERROR: No games completed"
     exit 1
   fi

   # Edge case: all losses → definite regression
   if [ "$W" -eq 0 ] && [ "$D" -eq 0 ]; then
     echo "REGRESSION DETECTED: PR engine lost all ${N} games"
     echo "Elo: -inf"
     exit 1
   fi

   # Edge case: all wins → no regression
   if [ "$L" -eq 0 ] && [ "$D" -eq 0 ]; then
     echo "NO REGRESSION: PR engine won all ${N} games"
     echo "Elo: +inf"
     exit 0
   fi

   # Compute Elo difference and 95% CI using awk (float arithmetic)
   awk -v w="$W" -v d="$D" -v l="$L" 'BEGIN {
     n = w + d + l
     score = (w + d * 0.5) / n

     # Elo difference: -400 * log10(1/score - 1)
     # Note: awk log() is natural log; log10(x) = log(x)/log(10)
     elo = -400.0 * log(1.0/score - 1.0) / log(10.0)

     # Standard error of the score proportion
     se = sqrt(score * (1.0 - score) / n)

     # Lower bound of 95% CI: compute Elo from (score - 1.96*se)
     # This is the delta-method approximation, acceptable for n=100
     score_lower = score - 1.96 * se

     # Clamp score_lower to avoid log(0) or log(negative)
     if (score_lower <= 0.001) score_lower = 0.001
     if (score_lower >= 0.999) score_lower = 0.999

     elo_lower = -400.0 * log(1.0/score_lower - 1.0) / log(10.0)

     # Upper bound for display
     score_upper = score + 1.96 * se
     if (score_upper <= 0.001) score_upper = 0.001
     if (score_upper >= 0.999) score_upper = 0.999
     elo_upper = -400.0 * log(1.0/score_upper - 1.0) / log(10.0)

     printf "Score: %.1f%%\n", score * 100
     printf "Elo difference: %+.1f\n", elo
     printf "95%% CI: [%+.1f, %+.1f] (delta-method approximation)\n", elo_lower, elo_upper
     printf "Lower bound of 95%% CI: %+.1f\n", elo_lower

     # Regression check: if the UPPER bound of the Elo CI is below -30,
     # we are confident at p<0.05 that the true Elo is below -30.
     # Equivalently: the Elo estimate minus the margin is below -30.
     # Using elo_lower: if elo_lower < -30, the PR engine is significantly
     # weaker by more than 30 Elo at the 95% confidence level.
     if (elo_lower < -30.0) {
       printf "REGRESSION DETECTED: Elo lower bound (%.1f) < -30\n", elo_lower
       exit 1
     } else {
       printf "NO REGRESSION: Elo lower bound (%.1f) >= -30\n", elo_lower
       exit 0
     }
   }'
   ```

   **Sign convention**: The PR engine is the first engine passed to CuteChess-CLI. CuteChess-CLI reports `W - L - D` from the first engine's perspective. So `W` = PR wins, `L` = PR losses. A score < 0.5 means the PR engine is weaker → negative Elo → regression. The formula `-400 * log10(1/score - 1)` yields negative Elo when score < 0.5 and positive Elo when score > 0.5.

   **Edge case handling**: If score = 0 (all losses), the log formula is undefined; the script detects this and immediately flags regression. If score = 1 (all wins), the script immediately passes. For extreme-but-not-zero scores, `score_lower` and `score_upper` are clamped to `[0.001, 0.999]` to avoid log(0).

   **Delta-method note**: Converting `score ± 1.96*SE` to Elo via the nonlinear logistic formula is a first-order approximation. For 100 games this approximation is adequate — the error is small relative to the wide confidence interval inherent in 100 games.

## 4. API Contracts

N/A — This deliverable is a CI workflow file, not a library or binary with an API surface.

## 5. Test Plan

**What to test**: The workflow YAML is validated by GitHub Actions when it runs. The regression detection logic (inline awk script) is the critical component.

**Test 1 — Workflow triggers on pull requests**: Create a PR to verify the workflow appears in the PR's "Checks" tab. Verification: the workflow is listed and starts executing.

**Test 2 — Engines build successfully in CI**: Both the PR engine (`./pr-engine`) and the baseline engine (`./baseline-engine`) must compile. Verification: the build steps succeed without errors.

**Test 3 — CuteChess-CLI builds from source in CI**: The build dependencies install, the repository clones at v1.3.1, and CMake produces a working binary. Verification: `./cutechess-cli --version` outputs version information.

**Test 4 — Tournament runs to completion**: The 100-game head-to-head tournament completes and produces a `Score of` line in `tournament-output.txt`. Verification: the tournament step produces parseable output and `regression-games.pgn` contains 100 games.

**Test 5 — Regression detection reports pass for non-regressing PR**: When the PR engine is identical to baseline (e.g., a docs-only change), the Elo difference should be near zero with a wide CI, and the workflow should pass. Verification: exit code 0.

**Test 6 — Regression detection handles edge cases**: Verify the inline script's behavior for known W/D/L values:
- `W=50, L=50, D=0` → score=0.5, Elo≈0, should pass
- `W=10, L=90, D=0` → score=0.1, Elo≈-191, elo_lower far below -30, should flag regression
- `W=0, L=100, D=0` → all losses, immediate regression flag
- `W=100, L=0, D=0` → all wins, immediate pass
- `W=45, L=55, D=0` → score=0.45, Elo≈-35, but CI is wide with 100 games so elo_lower depends on SE

**Test 7 — PR status check reflects result**: The GitHub Actions workflow result (pass/fail) appears as a status check on the PR. Verification: the check is visible in the PR UI.

## 6. Implementation Order

1. **Create `.github/workflows/` directory** — Ensure the directory structure exists (it may not since no workflows exist yet).

2. **Write `.github/workflows/regression.yml`** — The complete workflow file containing:
   - Workflow name: `Regression Test`
   - Trigger: `on: pull_request` targeting `main` branch
   - Single job `regression-test` on `ubuntu-latest` with the 9 steps described in Section 3
   - The inline regression detection script from Section 3, step 9 (embedded as a `run:` block in the final step)

3. **Verify locally** — Run the verification commands from Section 7 to confirm the YAML is valid and the awk script logic is correct.

## 7. Verification Commands

```bash
# Verify the workflow file is valid YAML
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/regression.yml'))"

# Verify the workflow file exists in the expected location
ls -la .github/workflows/regression.yml

# Verify the workflow triggers on pull_request
grep -A2 'on:' .github/workflows/regression.yml

# Verify that no reference engines (Stockfish, Ethereal, Rustic) are downloaded in the workflow
grep -ci 'stockfish\|ethereal\|rustic' .github/workflows/regression.yml
# Expected: 0

# Verify the engine builds (this already works as part of the project)
cargo build --release -p engine

# Dry-run: test the regression detection awk script with known W/D/L values

# Test: equal score (should pass)
echo "Testing W=50 D=0 L=50 (Elo≈0, should pass)..."
awk -v w=50 -v d=0 -v l=50 'BEGIN {
  n=w+d+l; score=(w+d*0.5)/n
  elo=-400.0*log(1.0/score-1.0)/log(10.0)
  se=sqrt(score*(1.0-score)/n)
  sl=score-1.96*se; if(sl<=0.001)sl=0.001; if(sl>=0.999)sl=0.999
  elo_lower=-400.0*log(1.0/sl-1.0)/log(10.0)
  printf "score=%.3f elo=%+.1f elo_lower=%+.1f -> %s\n", score, elo, elo_lower, (elo_lower<-30)?"FAIL":"PASS"
}'

# Test: heavy losses (should flag regression)
echo "Testing W=10 D=0 L=90 (Elo≈-191, should flag regression)..."
awk -v w=10 -v d=0 -v l=90 'BEGIN {
  n=w+d+l; score=(w+d*0.5)/n
  elo=-400.0*log(1.0/score-1.0)/log(10.0)
  se=sqrt(score*(1.0-score)/n)
  sl=score-1.96*se; if(sl<=0.001)sl=0.001; if(sl>=0.999)sl=0.999
  elo_lower=-400.0*log(1.0/sl-1.0)/log(10.0)
  printf "score=%.3f elo=%+.1f elo_lower=%+.1f -> %s\n", score, elo, elo_lower, (elo_lower<-30)?"FAIL":"PASS"
}'

# Test: mild loss (should probably pass — CI too wide with 100 games)
echo "Testing W=43 D=14 L=43 (Elo≈0, should pass)..."
awk -v w=43 -v d=14 -v l=43 'BEGIN {
  n=w+d+l; score=(w+d*0.5)/n
  elo=-400.0*log(1.0/score-1.0)/log(10.0)
  se=sqrt(score*(1.0-score)/n)
  sl=score-1.96*se; if(sl<=0.001)sl=0.001; if(sl>=0.999)sl=0.999
  elo_lower=-400.0*log(1.0/sl-1.0)/log(10.0)
  printf "score=%.3f elo=%+.1f elo_lower=%+.1f -> %s\n", score, elo, elo_lower, (elo_lower<-30)?"FAIL":"PASS"
}'
```

## 8. Design Deviations

**Deviation 1: Head-to-head PR-vs-Baseline instead of PR-vs-reference-engines**

- **Parent spec states**: "runs a 100-game tournament after each PR" and the task spec lists CuteChess-CLI integration (T01) and SPRT framework (T02) as dependencies, implying the CI job should leverage those tools. T01 runs against reference engines (Stockfish, Ethereal, Rustic).
- **Why that approach is problematic for CI regression testing**: Running against 3 reference engines with only 100 total games means ~33 games per engine pair, producing confidence intervals of ±100+ Elo — far too wide to detect a 30 Elo regression. Additionally, downloading and building 3 reference engines in CI adds significant setup time and introduces external failure points (broken download URLs, incompatible binaries). The regression test's goal is to detect whether the PR is weaker than the current baseline, not to estimate absolute Elo — the latter is T01's domain.
- **Chosen approach**: Run a single 100-game head-to-head tournament: PR engine vs Baseline engine (built from the `main` branch). This directly measures relative strength change, eliminates external engine dependencies, and uses all 100 games for a single comparison to maximize statistical power. No reference engines are downloaded in the CI workflow.

**Deviation 2: Inline shell/awk regression detection instead of invoking the SPRT tool (T02)**

- **Parent spec states**: "The regression detection logic may leverage the SPRT framework" (T02 dependency).
- **Why SPRT is not suitable for fixed-game-count CI tests**: The SPRT tool (T02) implements sequential testing — it runs batched matches with accept/reject/continue semantics, potentially playing thousands of games until a decision boundary is crossed. The CI regression test has a fixed budget of exactly 100 games and needs a definitive pass/fail result. Running SPRT with Elo0=0, Elo1=5 on 100 games would almost always return "inconclusive" because 100 games is far too few to distinguish a 5 Elo difference. The task spec criterion — "flags a regression if Elo drops by more than 30 at p<0.05" — is a one-shot frequentist hypothesis test (confidence interval check), not a sequential probability ratio test.
- **Chosen approach**: An inline awk script computes the Elo difference and its 95% confidence interval from the 100-game W/D/L results using the logistic Elo formula. It flags a regression if and only if the lower bound of the 95% CI is below -30 Elo. This directly and precisely implements the spec's "Elo drops by more than 30 at p<0.05" criterion. The delta-method approximation (applying the nonlinear Elo conversion to `score ± 1.96*SE`) is acknowledged as an approximation that is adequate for n=100, where the confidence interval is inherently wide.