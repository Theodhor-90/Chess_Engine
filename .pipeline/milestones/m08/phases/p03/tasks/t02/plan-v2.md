Now I have full context. Let me produce the revised plan that addresses the two feedback issues:

1. **Correctness**: The engine ordering bug where Baseline is listed first in CuteChess-CLI arguments, but the plan incorrectly states Candidate's wins are returned.
2. **Ambiguity**: The unclear mapping between `parse_score_line` return values and how `run_batch` / main loop interpret them.

## 1. Deliverables

| File | Purpose |
|------|---------|
| `tools/sprt/Cargo.toml` | **Modify** — remove `chess-types`, `chess-board`, `chess-movegen` dependencies (no longer needed; the tool wraps CuteChess-CLI instead of driving engines directly) |
| `tools/sprt/src/main.rs` | **Rewrite** — SPRT testing tool that invokes CuteChess-CLI to run batched matches between a baseline engine and a test engine, computes the log-likelihood ratio from game results, and reports accept/reject/continue status with current LLR and game count |

## 2. Dependencies

- **CuteChess-CLI**: External binary, must be available on `PATH` or specified via `--cutechess` argument. Not a Rust crate dependency.
- **No new Rust crate dependencies.** The existing `chess-types`, `chess-board`, and `chess-movegen` dependencies are removed since the tool no longer plays games itself — it delegates to CuteChess-CLI.
- **T01 (CuteChess-CLI Integration)** must be complete: the SPRT tool follows the same pattern for invoking CuteChess-CLI and parsing its output (score lines, Elo lines).

## 3. Implementation Details

### `tools/sprt/Cargo.toml`

Remove the three workspace dependencies (`chess-types`, `chess-board`, `chess-movegen`). The `[dependencies]` section becomes empty since the tool only uses the standard library.

```toml
[package]
name = "sprt"
version = "0.1.0"
edition.workspace = true
```

### `tools/sprt/src/main.rs`

Complete rewrite. The tool wraps CuteChess-CLI rather than playing games directly.

**Structs:**

- `Args` — CLI arguments:
  - `baseline: String` — path to baseline engine binary
  - `candidate: String` — path to candidate (test) engine binary
  - `cutechess: String` — path to CuteChess-CLI binary
  - `elo0: f64` — null hypothesis Elo bound (default 0.0)
  - `elo1: f64` — alternative hypothesis Elo bound (default 5.0)
  - `alpha: f64` — Type I error rate (default 0.05)
  - `beta: f64` — Type II error rate (default 0.05)
  - `batch_size: u32` — games per CuteChess-CLI invocation batch (default 100)
  - `max_games: u32` — maximum total games before declaring inconclusive (default 10000)
  - `tc: String` — time control string for CuteChess-CLI (default "10+0.1")
  - `openings: Option<String>` — optional openings file path
  - `concurrency: u32` — concurrent games (default 1)

**Functions:**

- `fn main()` — Parse args, compute SPRT decision thresholds, run batches of games via CuteChess-CLI in a loop. After each batch, accumulate W/D/L totals, compute LLR, print status line, and check against thresholds. Stop on accept, reject, or max games.

- `fn parse_args() -> Args` — Manual arg parsing (same pattern as T01's `tournament` tool). All SPRT-specific parameters (`--elo0`, `--elo1`, `--alpha`, `--beta`) are optional with defaults as specified.

- `fn print_usage()` — Print usage help to stderr.

- `fn run_batch(cutechess: &str, baseline: &str, candidate: &str, games: u32, tc: &str, openings: Option<&str>, concurrency: u32) -> (u32, u32, u32)` — Invoke CuteChess-CLI as a subprocess. **Candidate is listed as the first `-engine` argument and Baseline as the second**, so that CuteChess-CLI's "Score of Engine1 vs Engine2: W - L - D" line reports W/L/D from Candidate's perspective directly. The arguments are:
  - `-engine name=Candidate cmd=<candidate>`
  - `-engine name=Baseline cmd=<baseline>`
  - `-each proto=uci tc=<tc>`
  - `-games <games>` `-rounds <games/2>`
  - `-repeat`
  - `-openingfile <openings>` (if provided)
  - `-concurrency <concurrency>`
  - `-ratinginterval 0`
  
  Capture stdout, parse the final "Score of ..." line using `parse_score_line()`. The `parse_score_line()` function returns `(W, L, D)` as they appear in the score line — since Candidate is Engine1 (listed first), W is Candidate's wins and L is Candidate's losses. Return `(candidate_wins, candidate_losses, draws)`.

  **Why Candidate is listed first:** CuteChess-CLI reports "Score of Engine1 vs Engine2: W - L - D" where W is Engine1's wins. By placing Candidate as Engine1, the parsed W value directly corresponds to Candidate's wins, eliminating any need to swap values and avoiding the risk of an inverted LLR sign.

- `fn parse_score_line(line: &str) -> Option<(u32, u32, u32)>` — Parse CuteChess-CLI score output format `"Score of X vs Y: W - L - D [pct]"`. Returns `(W, L, D)` exactly as they appear in the line — W is the first engine's wins, L is the first engine's losses, D is draws. Same implementation as the tournament tool.

- `fn elo_to_score(elo: f64) -> f64` — Convert an Elo difference to an expected score: `1.0 / (1.0 + 10^(-elo/400))`.

- `fn log_likelihood_ratio(wins: u32, draws: u32, losses: u32, elo0: f64, elo1: f64) -> f64` — Compute the log-likelihood ratio using the trinomial model. The `wins` and `losses` parameters are from Candidate's perspective.
  1. Compute observed score `s = (w + d*0.5) / n`
  2. Compute observed variance `var = (w*(1-s)^2 + d*(0.5-s)^2 + l*s^2) / n`
  3. Compute expected scores under H0 and H1: `s0 = elo_to_score(elo0)`, `s1 = elo_to_score(elo1)`
  4. Return `LLR = 0.5 * n * (s1 - s0) * (2*s - s0 - s1) / var`
  5. Return 0.0 if `n == 0` or `var < 1e-12`

- `fn sprt_bounds(alpha: f64, beta: f64) -> (f64, f64)` — Compute decision thresholds:
  - `lower = ln(beta / (1 - alpha))`
  - `upper = ln((1 - beta) / alpha)`

**Main loop logic:**

```
compute (lower, upper) from (alpha, beta)
total_wins, total_draws, total_losses = 0      // all from Candidate's perspective
while total_games < max_games:
    remaining = max_games - total_games
    batch = min(batch_size, remaining)
    (w, l, d) = run_batch(...)                  // w = Candidate wins, l = Candidate losses, d = draws
    total_wins += w; total_draws += d; total_losses += l
    llr = log_likelihood_ratio(total_wins, total_draws, total_losses, elo0, elo1)
    print status line: game count, W/D/L, score%, LLR, bounds
    if llr >= upper: print "ACCEPT: H1 (candidate is stronger)"; exit 0
    if llr <= lower: print "REJECT: H0 (no significant difference)"; exit 1
print "INCONCLUSIVE: max games reached"; exit 2
```

**Engine ordering rationale and data flow summary:**

1. CuteChess-CLI is invoked with `-engine name=Candidate cmd=... -engine name=Baseline cmd=...` — Candidate is Engine1.
2. CuteChess-CLI outputs `"Score of Candidate vs Baseline: W - L - D [pct]"` — W = Candidate's wins, L = Candidate's losses.
3. `parse_score_line()` returns `(W, L, D)` as-is — no swap needed.
4. `run_batch()` returns `(W, L, D)` = `(candidate_wins, candidate_losses, draws)`.
5. Main loop passes `(total_wins, total_draws, total_losses)` to `log_likelihood_ratio()`, where `wins` = Candidate's wins.
6. A positive LLR means Candidate is outperforming Baseline → ACCEPT is correct.

**Output format (machine-parseable):**

Each batch prints a status line:
```
Games: 200/10000 | +55 =90 -55 | Score: 50.0% | LLR: 0.12 [-2.944, 2.944] | Status: continue
```

Final line:
```
Result: ACCEPT | LLR: 2.95 (>= 2.944) | Games: 1200 | +340 =540 -320 | Score: 51.7%
```
or:
```
Result: REJECT | LLR: -2.95 (<= -2.944) | Games: 800 | +180 =400 -220 | Score: 47.5%
```
or:
```
Result: INCONCLUSIVE | LLR: 1.20 | Games: 10000 | +2500 =5000 -2500 | Score: 50.0%
```

**Unit tests (in `#[cfg(test)] mod tests`):**

- `test_sprt_bounds` — verify `sprt_bounds(0.05, 0.05)` produces `lower ≈ -2.944` and `upper ≈ 2.944`
- `test_elo_to_score_zero` — `elo_to_score(0.0)` returns 0.5
- `test_elo_to_score_positive` — `elo_to_score(5.0)` returns ~0.5072
- `test_llr_equal_score` — 50/50 W/L with `elo0=0, elo1=5` gives LLR near 0
- `test_llr_positive` — candidate wins more → positive LLR
- `test_llr_negative` — candidate loses more → negative LLR
- `test_llr_zero_games` — returns 0.0
- `test_parse_score_line_valid` — parse a correctly formatted CuteChess score line
- `test_parse_score_line_invalid` — returns `None` for malformed input

## 4. API Contracts

**CLI interface:**

```
sprt --baseline <path> --candidate <path> --cutechess <path> \
     [--elo0 <f64>] [--elo1 <f64>] [--alpha <f64>] [--beta <f64>] \
     [--tc <string>] [--batch-size <N>] [--max-games <N>] \
     [--openings <path>] [--concurrency <N>]
```

**Required arguments:**
- `--baseline <path>` — path to baseline engine binary
- `--candidate <path>` — path to candidate engine binary  
- `--cutechess <path>` — path to CuteChess-CLI binary

**Optional arguments with defaults:**
- `--elo0 <f64>` — default 0.0
- `--elo1 <f64>` — default 5.0
- `--alpha <f64>` — default 0.05
- `--beta <f64>` — default 0.05
- `--tc <string>` — default "10+0.1"
- `--batch-size <N>` — default 100
- `--max-games <N>` — default 10000
- `--openings <path>` — no default (optional)
- `--concurrency <N>` — default 1

**Exit codes:**
- `0` — ACCEPT (candidate is stronger)
- `1` — REJECT (no significant difference / candidate is not stronger)
- `2` — INCONCLUSIVE (max games reached without decision)

**stdout format (per batch):**
```
Games: <total>/max_games | +W =D -L | Score: X.X% | LLR: X.XX [lower, upper] | Status: continue|accept|reject
```

**Final result line:**
```
Result: ACCEPT|REJECT|INCONCLUSIVE | LLR: X.XX | Games: N | +W =D -L | Score: X.X%
```

In all output, W = Candidate wins, L = Candidate losses, D = draws.

## 5. Test Plan

All tests are in `tools/sprt/src/main.rs` under `#[cfg(test)] mod tests`.

**Setup:** None required; all tests are pure unit tests on mathematical functions and parsing logic.

| Test | Verifies |
|------|----------|
| `test_sprt_bounds` | `sprt_bounds(0.05, 0.05)` returns `(lower ≈ -2.944, upper ≈ 2.944)` matching the decision thresholds formula `ln(β/(1-α))` and `ln((1-β)/α)` |
| `test_elo_to_score_zero` | `elo_to_score(0.0)` returns exactly `0.5` |
| `test_elo_to_score_positive` | `elo_to_score(5.0)` returns a value slightly above `0.5` (≈0.5072) |
| `test_llr_equal_score` | Equal W/D/L (e.g., 100W/200D/100L) with elo0=0, elo1=5 gives LLR ≈ 0 |
| `test_llr_positive` | More wins than losses (e.g., 150W/200D/100L) gives LLR > 0 |
| `test_llr_negative` | More losses than wins (e.g., 100W/200D/150L) gives LLR < 0 |
| `test_llr_zero_games` | `log_likelihood_ratio(0, 0, 0, ...)` returns 0.0 |
| `test_parse_score_line_valid` | Parses `"Score of Candidate vs Baseline: 120 - 100 - 280 [0.520]"` to `(120, 100, 280)` |
| `test_parse_score_line_invalid` | Returns `None` for non-matching input |

These tests cover verification criteria #2 (LLR calculated correctly) and #5 (output parseable). Criteria #1, #3, #4 require running CuteChess-CLI with actual engines and are verified via the manual verification commands below.

## 6. Implementation Order

1. **Modify `tools/sprt/Cargo.toml`** — Remove `chess-types`, `chess-board`, `chess-movegen` dependencies.

2. **Rewrite `tools/sprt/src/main.rs`** — Replace the existing direct-engine-driving implementation with the CuteChess-CLI wrapper approach:
   - `Args` struct and `parse_args()` / `print_usage()` with the new CLI parameters (`--cutechess`, `--tc`, `--batch-size`, etc.)
   - `sprt_bounds()` function (retained from existing code, same formula)
   - `elo_to_score()` function (retained from existing code, same formula)
   - `log_likelihood_ratio()` function (retained from existing code, same formula)
   - `parse_score_line()` function (new — same pattern as tournament tool's parser)
   - `run_batch()` function (new — invokes CuteChess-CLI subprocess with **Candidate as first `-engine` arg**, parses output)
   - `main()` function — batch loop with LLR checking and status output
   - `#[cfg(test)] mod tests` with all unit tests

3. **Verify** — Run `cargo build -p sprt`, `cargo test -p sprt`, `cargo clippy -p sprt`.

## 7. Verification Commands

```bash
# Build the SPRT tool
cargo build -p sprt

# Run unit tests
cargo test -p sprt

# Check clippy
cargo clippy -p sprt -- -D warnings

# Check formatting
cargo fmt -p sprt --check

# Verify the binary runs and shows usage on missing args
cargo run -p sprt -- --help 2>&1 || true

# Integration test (requires CuteChess-CLI and two engine binaries):
# Testing engine against itself should NOT trigger accept (same strength = no gain)
# cargo run -p sprt -- \
#   --baseline ./target/release/engine \
#   --candidate ./target/release/engine \
#   --cutechess cutechess-cli \
#   --elo0 0 --elo1 5 --alpha 0.05 --beta 0.05 \
#   --tc "1+0.01" --batch-size 50 --max-games 500
```

## 8. Design Deviations

**Deviation 1: CuteChess-CLI wrapper instead of direct engine driving**

- **What the existing code does:** The current `tools/sprt/src/main.rs` spawns UCI engines directly, plays games move-by-move using `chess-board` and `chess-movegen`, and manages the full game lifecycle internally.
- **What the task spec requires:** "Uses CuteChess-CLI to run matches between a baseline engine binary and a test engine binary" and the phase spec says "SPRT implemented as a wrapper around CuteChess-CLI."
- **Resolution:** The entire game-playing infrastructure is removed and replaced with CuteChess-CLI subprocess invocation. The SPRT statistical logic (LLR calculation, bounds) is retained since it is correct and matches the spec. The `chess-types`, `chess-board`, and `chess-movegen` dependencies are removed.

**Deviation 2: Batch-based incremental execution via repeated CuteChess-CLI invocations**

- **What the task spec says:** "The tool should be able to run incrementally (batches of games) and report status after each batch."
- **Why this requires deviation from a single CuteChess-CLI invocation:** CuteChess-CLI runs a fixed number of games and then exits. To support incremental SPRT checking after each batch, the tool invokes CuteChess-CLI multiple times (once per batch), accumulates the W/D/L totals across batches, and checks the LLR after each batch completes. This is the standard approach used by other SPRT frameworks that wrap CuteChess-CLI.

**Deviation 3: Default SPRT parameters instead of all-required**

- **What the existing code does:** All SPRT parameters (`--elo0`, `--elo1`, `--alpha`, `--beta`) are required with no defaults.
- **What the task spec says:** The bounds are "Elo0=0, Elo1=5, alpha=0.05, beta=0.05" — these are the canonical values.
- **Resolution:** These four parameters default to the spec-mandated values (0, 5, 0.05, 0.05) so the tool can be invoked with just `--baseline`, `--candidate`, and `--cutechess`. They remain overridable via CLI flags for flexibility. This avoids requiring users to type the canonical values every time, which would be the common case.

**Deviation 4: Candidate listed as first `-engine` argument (engine ordering)**

- **What the v1 plan specified:** Baseline was listed as the first `-engine` argument and Candidate as the second.
- **Problem:** CuteChess-CLI's score line format is `"Score of Engine1 vs Engine2: W - L - D"` where W is Engine1's wins. With Baseline as Engine1, W would be Baseline's wins, not Candidate's wins. The v1 plan incorrectly stated that `parse_score_line` returns values "from the perspective of Candidate (first engine listed in the score line)" — but Candidate was the second engine, not the first. This would have inverted the LLR sign, causing the tool to accept when it should reject and vice versa.
- **Resolution:** Candidate is now listed as the first `-engine` argument (`-engine name=Candidate cmd=<candidate> -engine name=Baseline cmd=<baseline>`). This way, the score line's W value directly represents Candidate's wins, and `parse_score_line()` returns `(candidate_wins, candidate_losses, draws)` without any swap needed. The data flow is unambiguous: `parse_score_line` → `run_batch` → main loop → `log_likelihood_ratio` all operate on Candidate's W/D/L consistently.