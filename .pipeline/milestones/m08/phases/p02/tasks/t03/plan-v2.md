Good. Now I have all context. Let me produce the revised plan.

## 1. Deliverables

| # | File | Purpose |
|---|------|---------|
| 1 | `tools/sprt/Cargo.toml` | Crate manifest for the SPRT testing tool binary |
| 2 | `tools/sprt/src/main.rs` | SPRT A/B testing tool: runs two engine binaries (baseline vs candidate) head-to-head via UCI, computes SPRT log-likelihood ratio to accept/reject a change, reports results |
| 3 | `Cargo.toml` (workspace root) | Updated `members` to include `tools/sprt` |
| 4 | `crates/search/src/lib.rs` | Updated LMR reduction table divisor, null move reduction depths, futility margins, reverse futility margins, and aspiration window initial delta + widening logic |

## 2. Dependencies

- **Internal crates** (for `tools/sprt`): `chess-types`, `chess-board`, `chess-movegen` — same pattern as `tools/selfplay`
- **Rust standard library only** — no external crates beyond workspace dependencies; SPRT math (log-likelihood ratio, Elo-to-win-probability) uses `f64` operations from `std`
- **Phase P01/T03 complete**: The `selfplay` tool exists and provides the game-playing mechanism; the SPRT tool reuses its pattern of UCI engine communication
- **Phase P02/T01–T02 complete**: Texel-tuned evaluation weights are already applied to the `eval` crate

## 3. Implementation Details

### 3.1 `tools/sprt/Cargo.toml`

Standard tool binary manifest following the pattern of `tools/selfplay/Cargo.toml`:
- Package name: `sprt`
- Edition: `workspace`
- Dependencies: `chess-types`, `chess-board`, `chess-movegen` (relative paths `../../crates/*`)

### 3.2 `tools/sprt/src/main.rs`

**Purpose**: Run a head-to-head match between two engine binaries (baseline and candidate) and apply the Sequential Probability Ratio Test to determine whether the candidate is stronger, weaker, or statistically indistinguishable.

**CLI interface**:
```
sprt --baseline <path> --candidate <path> --openings <path> \
     --elo0 <f64> --elo1 <f64> --alpha <f64> --beta <f64> \
     --movetime <ms> [--maxgames <N>] [--maxmoves <N>]
```

- `--baseline`: path to baseline engine binary
- `--candidate`: path to candidate engine binary
- `--openings`: path to openings file (same format as selfplay: one FEN per line)
- `--elo0`: null hypothesis Elo difference (typically 0)
- `--elo1`: alternative hypothesis Elo difference (typically 5)
- `--alpha`: Type I error rate (typically 0.05)
- `--beta`: Type II error rate (typically 0.05)
- `--movetime`: milliseconds per move
- `--maxgames`: maximum total games before declaring inconclusive (default: 10000)
- `--maxmoves`: max plies per game (default: 300)

**Key functions**:

- `parse_args()` → `Args`: Manual argument parsing (same pattern as selfplay/texel)
- `load_openings(path: &str)` → `Vec<String>`: Load FEN lines, skip comments/blank lines
- `spawn_engine(path: &str)` → `EngineHandle`: Spawn engine process, UCI handshake, return writer + receiver (identical pattern to selfplay)
- `send_line()`, `wait_for_line()`, `wait_for_bestmove()`: UCI communication helpers (same as selfplay)
- `validate_move()`: Validate engine output against legal moves (same as selfplay)
- `play_game(baseline: &str, candidate: &str, opening_fen: &str, movetime: u64, max_moves: u32, baseline_is_white: bool)` → `GameResult`: Play a single game between two different engines. In even-numbered games baseline is white; in odd-numbered games candidate is white.
- `elo_to_score(elo: f64)` → `f64`: Convert Elo difference to expected score: `1.0 / (1.0 + 10.0_f64.powf(-elo / 400.0))`
- `log_likelihood_ratio(wins: u32, draws: u32, losses: u32, elo0: f64, elo1: f64)` → `f64`: Compute the SPRT LLR using the Gaussian approximation (details below).
- `sprt_bounds(alpha: f64, beta: f64)` → `(f64, f64)`: Compute lower bound `ln(beta / (1 - alpha))` and upper bound `ln((1 - beta) / alpha)`.
- `main()`: Game loop that plays games in pairs (alternating colors), computes LLR after each game pair, and terminates when LLR crosses either SPRT bound or max games reached. Prints running W/D/L, LLR, and bounds. Final output: H0 accepted (no gain), H1 accepted (candidate stronger), or inconclusive.

**SPRT math details — Gaussian approximation**:

The LLR is computed using the Gaussian approximation, the standard approach used by most chess SPRT implementations (fishtest, cutechess-cli):

1. Convert `elo0` and `elo1` to expected scores `s0` and `s1` via `elo_to_score()`
2. Compute observed score: `observed_score = (W + D/2) / N` where `N = W + D + L`
3. Compute variance: `variance = (W * (1.0 - observed_score)^2 + D * (0.5 - observed_score)^2 + L * (0.0 - observed_score)^2) / N`
4. Compute LLR: `LLR = 0.5 * N * (s1 - s0) * (2.0 * observed_score - s0 - s1) / variance`
5. Compute SPRT bounds: `lower = ln(beta / (1 - alpha))`, `upper = ln((1 - beta) / alpha)`
6. Decision: if `LLR >= upper`, accept H1 (candidate stronger); if `LLR <= lower`, accept H0 (no gain); otherwise continue playing

**Output format** (printed to stdout):
```
SPRT: elo0=0.0 elo1=5.0 alpha=0.05 beta=0.05
Bounds: lower=-2.944 upper=2.944
Game 2/10000: +1 =0 -0 (score=100.0%) LLR=0.42 [-2.944, 2.944]
...
Result: H1 accepted (candidate is stronger)
Final: +52 =30 -18 (score=67.0%) LLR=3.02 after 200 games
```

### 3.3 `Cargo.toml` (workspace root)

Add `"tools/sprt"` to the `members` list, after `"tools/pgn-extract"`.

### 3.4 `crates/search/src/lib.rs` — Tuned search parameters

Each parameter is tuned via the SPRT tool against the current best. The tuning process is: build baseline engine, modify a parameter, build candidate engine, run SPRT. If H1 accepted, keep the change; if H0 accepted, revert. The following parameters are updated:

**LMR reduction table** (line 96):
- Current: divisor `1.75` in formula `floor(ln(d) * ln(m) / 1.75)`
- Candidate values to test: `1.50`, `1.60`, `1.70`, `1.80`, `1.90`, `2.00`
- The SPRT-validated winner replaces `1.75`

**Null move reduction depth** (line 302):
- Current: `R = 2` for `depth <= 6`, `R = 3` for `depth > 6`
- Candidates: adjust the depth threshold (try `depth <= 5`, `depth <= 7`, `depth <= 8`) and reduction values (try `R = 3` for shallow, `R = 4` for deep)
- The SPRT-validated winner replaces the current values

**Futility margins** (line 27):
- Current: `[0, 200, 400, 600]`
- Candidates: scale by factors (e.g., `[0, 150, 300, 450]`, `[0, 180, 360, 540]`, `[0, 250, 450, 700]`)
- The SPRT-validated winner replaces the current array

**Reverse futility margins** (line 28):
- Current: `[0, 100, 300, 500]`
- Candidates: scale by factors (e.g., `[0, 80, 250, 400]`, `[0, 120, 350, 550]`, `[0, 150, 350, 600]`)
- The SPRT-validated winner replaces the current array

**Aspiration windows** (lines 670–674):
- Current: full window `(-INFINITY, INFINITY)` at every depth — no aspiration windows implemented
- Implementation: Add aspiration window logic to the `search()` function's iterative deepening loop. Starting from depth 5+, use an initial window of `(prev_score - ASPIRATION_DELTA, prev_score + ASPIRATION_DELTA)`. On fail-low or fail-high, widen the window by multiplying delta by `ASPIRATION_WIDEN_FACTOR` and re-search. After two consecutive failures on the same side, fall back to full window.
- New constants: `const ASPIRATION_DELTA: i32 = 25;` and `const ASPIRATION_WIDEN_FACTOR: i32 = 4;`
- Candidates for ASPIRATION_DELTA: `15`, `20`, `25`, `30`, `40`, `50`
- Candidates for ASPIRATION_WIDEN_FACTOR: `2`, `3`, `4`, `5`
- The SPRT-validated winners become the final values

**Test updates**: The existing `futility_margins_match_spec` and `lmr_table_values_correct` tests must be updated to match the new tuned values.

## 4. API Contracts

N/A — the SPRT tool is a standalone binary with CLI interface only (documented in section 3.2). No library API surface.

## 5. Test Plan

### 5.1 `tools/sprt` build verification

The SPRT tool is a binary, not a library. Verification is via `cargo build --bin sprt` and integration testing through the SPRT runs themselves (each parameter tuning run exercises the tool end-to-end).

### 5.2 Search parameter tests in `crates/search/src/lib.rs`

**Update `futility_margins_match_spec` test**: Assert the new tuned futility margin and reverse futility margin values match the updated constants.

**Update `lmr_table_values_correct` test**: Recompute expected values for the new LMR divisor and assert they match.

**Existing behavioral tests remain unchanged**: `lmr_reduces_node_count` (>=30% reduction), `lmr_skips_tt_moves` (tactical best move preserved), `reverse_futility_prunes_node` (node reduction), `null_move_pruning_activates` — these test behaviors, not specific constant values, so they must continue to pass with the tuned parameters.

### 5.3 SPRT test results documentation

Each parameter change must have a documented SPRT result confirming non-regression. The results are printed by the SPRT tool to stdout and captured in the SPRT test results section. Format: parameter name, old value, new value, W/D/L, LLR, SPRT outcome (H0/H1).

## 6. Implementation Order

1. **Create `tools/sprt/Cargo.toml`** — tool manifest with workspace dependencies
2. **Create `tools/sprt/src/main.rs`** — SPRT binary implementing CLI, UCI engine communication, SPRT math, and game loop
3. **Update `Cargo.toml` (workspace root)** — add `"tools/sprt"` to members
4. **Verify SPRT tool builds**: `cargo build --bin sprt`
5. **Add aspiration windows to `crates/search/src/lib.rs`** — implement the aspiration window logic in the `search()` function with initial constants `ASPIRATION_DELTA = 25` and `ASPIRATION_WIDEN_FACTOR = 4`
6. **SPRT-validate aspiration windows**: Run SPRT with aspiration windows enabled (candidate) vs disabled (baseline). Accept if H1.
7. **SPRT-tune LMR divisor**: Test candidate divisor values, accept SPRT winner, update the constant in `lmr_table()`
8. **SPRT-tune null move reduction**: Test candidate R values and depth thresholds, accept SPRT winner, update inline constants
9. **SPRT-tune futility margins**: Test candidate margin arrays, accept SPRT winner, update `FUTILITY_MARGINS` and `REVERSE_FUTILITY_MARGINS`
10. **SPRT-tune aspiration window delta**: Test candidate delta values, accept SPRT winner, update `ASPIRATION_DELTA`
11. **Update unit tests**: Modify `futility_margins_match_spec` and `lmr_table_values_correct` to assert the new tuned values
12. **Final verification**: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## 7. Verification Commands

```bash
# Build the entire workspace including the new SPRT tool
cargo build --release

# Verify the SPRT tool binary exists
cargo build --bin sprt

# Run all unit tests (including updated search parameter tests)
cargo test

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy -- -D warnings

# Example SPRT run (baseline vs candidate with aspiration windows)
cargo run --release --bin sprt -- \
  --baseline ./target/release/engine \
  --candidate ./target/release/engine \
  --openings tools/selfplay/openings.txt \
  --elo0 0 --elo1 5 --alpha 0.05 --beta 0.05 \
  --movetime 100 --maxgames 200

# Run selfplay to confirm no crashes with tuned parameters
cargo run --release --bin selfplay -- \
  --engine ./target/release/engine \
  --openings tools/selfplay/openings.txt \
  --games 100 --movetime 100
```

## 8. Design Deviations

**Aspiration windows — adding new feature in search crate**

- **Parent spec requires**: "Tuned aspiration window sizes applied to the `search` crate"
- **Current state**: The `search()` function uses a full window `(-INFINITY, INFINITY)` at every depth — aspiration windows are not implemented at all
- **Deviation**: Before tuning aspiration window "sizes", the aspiration window mechanism must first be implemented. The task spec says to tune aspiration window sizes, which implies the feature exists. Since it does not, we implement the standard aspiration window pattern (initial delta + widening on fail) as part of this task, then tune the delta and widening factor via SPRT. This is the minimal implementation needed to have aspiration window parameters to tune.
- **Alternative considered**: Skip aspiration windows entirely — rejected because the task spec explicitly lists them as a deliverable.

**SPRT tool — new binary not explicitly listed as a deliverable but required by verification criteria**

- **Parent spec requires**: "SPRT test results for each individual parameter change confirming non-regression"
- **Current state**: No SPRT tool exists in the codebase
- **Deviation**: The task spec says "Optionally, a tuning script or tool that automates running SPRT tests for each parameter candidate" and verification criterion #1 requires SPRT test results. Since SPRT testing requires running two engine binaries head-to-head with statistical termination criteria, and no such tool exists, we create `tools/sprt` as a minimal SPRT runner. This is the infrastructure needed to produce the required SPRT test results.