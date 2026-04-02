Now I have all the information needed. Let me produce the task plan.

## 1. Deliverables

- `tools/tournament/Cargo.toml` — Crate manifest for the tournament runner binary, added as a workspace member
- `tools/tournament/src/main.rs` — Tournament runner that invokes CuteChess-CLI for engine-vs-engine matches against at least 3 reference engines, parses output, and produces Elo estimates with 95% confidence intervals
- `tools/tournament/engines.toml` — Configuration file defining reference engine entries (Stockfish at reduced depth, Ethereal, Rustic) with paths, time controls, and UCI options
- Root `Cargo.toml` — Modified to add `tools/tournament` to workspace members

## 2. Dependencies

- No new Rust crate dependencies. The tournament tool is a standalone binary that invokes CuteChess-CLI as an external process and parses its text output. It does not depend on any chess-engine workspace crates.
- **External tools required at runtime** (not committed to the repo):
  - `cutechess-cli` binary — the tournament manager
  - Reference engine binaries: Stockfish, Ethereal, Rustic — downloaded by the user (or CI) before running

## 3. Implementation Details

### `tools/tournament/Cargo.toml`

- Package name: `tournament`
- Edition: workspace
- No workspace crate dependencies — this tool only shells out to CuteChess-CLI and parses text output
- Standard library only (no external crates)

### `tools/tournament/engines.toml`

- TOML configuration file defining reference engines
- Each engine entry has: `name`, `cmd` (path to binary), `args` (optional CLI arguments), `options` (UCI options to set, e.g., `Threads=1`, `Hash=64`), and `tc` (time control string for CuteChess-CLI, e.g., `tc=10+0.1`)
- Stockfish entry includes `option.Skill Level=5` and `option.Threads=1` to reduce strength
- Ethereal and Rustic entries with default settings
- A `[defaults]` section for shared values: `games = 500`, `concurrency = 1`, `openings_file` (path to `tools/selfplay/openings.txt`), `openings_format = "epd"`, `rounds` and `repeat = true` for color-fair pairing

### `tools/tournament/src/main.rs`

**Args struct and parsing** (follows existing pattern from `tools/sprt/src/main.rs`):
- `--engine <path>` — path to our engine binary (required)
- `--cutechess <path>` — path to CuteChess-CLI binary (required)
- `--config <path>` — path to engines.toml config (required)
- `--games <N>` — games per engine pair (default 500)
- `--concurrency <N>` — concurrent games (default 1)
- `--tc <string>` — time control override (e.g., `10+0.1`)
- `--openings <path>` — opening book/FEN file override

**Config parsing**:
- Parse `engines.toml` using a minimal TOML parser (hand-rolled, since the format is simple key-value pairs within `[[engine]]` sections — consistent with the project's convention of minimizing external crates)
- Each engine entry produces a `ReferenceEngine { name, cmd, options, tc }` struct

**Tournament execution** (per reference engine):
- Build a CuteChess-CLI command line:
  ```
  cutechess-cli \
    -engine name=OurEngine cmd=<engine_path> \
    -engine name=<ref_name> cmd=<ref_cmd> option.<key>=<val> ... \
    -each proto=uci tc=<tc> \
    -games <games> -rounds <games/2> -repeat \
    -openingfile <openings_path> \
    -concurrency <concurrency> \
    -ratinginterval 0 \
    -pgnout <temp_pgn_path>
  ```
- Spawn CuteChess-CLI via `std::process::Command` with stdout piped
- Read stdout line by line looking for the `Elo difference:` line and the score summary line that CuteChess-CLI prints at the end of a tournament
- Parse wins/draws/losses from the score line (format: `Score of Engine1 vs Engine2: W - L - D  [pct]`)
- Parse Elo difference and error margin from the Elo line (format: `Elo difference: X.XX +/- Y.YY`)

**Elo estimation**:
- Primary: parse CuteChess-CLI's built-in Elo output (it reports Elo difference with error margin)
- Fallback if parsing fails: compute Elo from W/D/L using the standard formula:
  - `score = (W + D * 0.5) / N`
  - `elo = -400 * log10(1/score - 1)` 
  - Standard error: `se = sqrt(score * (1 - score) / N)`, convert to Elo error margin
  - 95% CI: `elo ± 1.96 * elo_error`

**Output**:
- Per engine pair: opponent name, W/D/L, score %, Elo difference, 95% CI
- Final summary table with all 3 opponents
- Exit code 0 on success, 1 if CuteChess-CLI fails to run

**Error handling**: consistent with other tools — `unwrap()` allowed in binaries, `eprintln!` + `std::process::exit(1)` for user errors.

### Root `Cargo.toml`

- Add `"tools/tournament"` to the `members` array

## 4. API Contracts

Command-line interface:

```bash
tournament \
  --engine ./target/release/engine \
  --cutechess cutechess-cli \
  --config tools/tournament/engines.toml \
  --games 500 \
  --concurrency 1 \
  --tc "10+0.1"
```

Output format (stdout):

```
=== Tournament: OurEngine vs Stockfish ===
Games: 500
Score: +120 =260 -120 (50.0%)
Elo difference: +0.0 +/- 28.5 (95% CI)

=== Tournament: OurEngine vs Ethereal ===
Games: 500
Score: +180 =200 -120 (56.0%)
Elo difference: +41.8 +/- 29.1 (95% CI)

=== Tournament: OurEngine vs Rustic ===
Games: 500
Score: +200 =190 -110 (59.0%)
Elo difference: +63.1 +/- 29.4 (95% CI)

=== Summary ===
Opponent       Games   Score    Elo     95% CI
Stockfish      500     50.0%    +0.0    +/- 28.5
Ethereal       500     56.0%    +41.8   +/- 29.1
Rustic         500     59.0%    +63.1   +/- 29.4
```

N/A for REST/HTTP.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| `cargo build -p tournament` compiles | The crate builds without errors within the workspace |
| `cargo clippy -p tournament -- -D warnings` passes | Code meets project lint standards |
| `cargo fmt --check -p tournament` passes | Code formatting matches project standards |
| Manual invocation with `--help`-style missing args prints usage and exits 1 | Argument parsing handles missing required args gracefully |
| Parse a sample CuteChess-CLI score line | The regex/string parsing correctly extracts W/D/L from `Score of X vs Y: W - L - D  [pct]` |
| Parse a sample CuteChess-CLI Elo line | The string parsing correctly extracts Elo difference and error margin from `Elo difference: X.XX +/- Y.YY` |
| Elo computation from W/D/L | Given known W/D/L values, the fallback Elo calculation produces correct Elo and 95% CI |
| Full integration: run tournament against 3 reference engines with 500+ games each, verify CI ±30 or narrower | End-to-end verification of the exit criteria (requires CuteChess-CLI and reference engines installed) |

The unit-testable functions (Elo computation from W/D/L, output line parsing) are tested via `#[cfg(test)] mod tests` in `main.rs`, consistent with project conventions.

## 6. Implementation Order

1. Root `Cargo.toml` — add `"tools/tournament"` to workspace members
2. `tools/tournament/Cargo.toml` — create crate manifest
3. `tools/tournament/engines.toml` — create reference engine configuration
4. `tools/tournament/src/main.rs` — implement in order:
   a. `Args` struct and `parse_args()` (following `tools/sprt` pattern)
   b. Config file parsing (read `engines.toml`, produce `Vec<ReferenceEngine>`)
   c. CuteChess-CLI command builder (construct the full command line per engine pair)
   d. CuteChess-CLI output parser (extract score line and Elo line)
   e. Fallback Elo computation from W/D/L with 95% CI
   f. Per-engine tournament loop and summary output
   g. `#[cfg(test)] mod tests` — unit tests for Elo computation and output parsing

## 7. Verification Commands

```bash
# Build the tournament tool
cargo build -p tournament

# Lint check
cargo clippy -p tournament -- -D warnings

# Format check
cargo fmt -p tournament --check

# Run unit tests (Elo computation, output parsing)
cargo test -p tournament

# Full integration test (requires cutechess-cli and reference engines installed):
cargo build --release -p engine && \
./target/debug/tournament \
  --engine ./target/release/engine \
  --cutechess cutechess-cli \
  --config tools/tournament/engines.toml \
  --games 500
```

## 8. Design Deviations

**Deviation: No workspace crate dependencies**

- The phase spec artifact listing says `tools/tournament/` without specifying dependencies.
- The existing `tools/sprt` and `tools/selfplay` depend on `chess-types`, `chess-board`, `chess-movegen` because they play games internally via UCI protocol.
- The tournament tool delegates all game-playing to CuteChess-CLI and only parses its text output. Adding chess-engine crate dependencies would be unnecessary coupling for a tool that never touches positions, moves, or boards directly.
- The tournament binary depends only on `std`.

**Deviation: Hand-rolled TOML parsing instead of adding a `toml` crate**

- The task spec says "configuration for at least 3 reference engines" but does not prescribe a format or parser.
- The project convention (master plan) is "minimize external crates" and "core engine logic should depend only on the standard library." While this is a tool (not core engine), all other tools in the workspace use zero external crates except `crates/engine` (which uses `anyhow`).
- The `engines.toml` format is simple enough (flat key-value sections) that a minimal hand-rolled parser is sufficient and avoids adding a dependency.