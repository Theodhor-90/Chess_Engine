## Files created/modified

- **`Cargo.toml`** (root) — Added `"tools/tournament"` to workspace members
- **`tools/tournament/Cargo.toml`** — New crate manifest; `name = "tournament"`, `edition.workspace = true`, no dependencies (std-only)
- **`tools/tournament/engines.toml`** — Reference engine config with `[defaults]` section (games=500, concurrency=1, openings from selfplay) and 3 `[[engine]]` entries: Stockfish (Skill Level 5, Threads=1, Hash=64), Ethereal, Rustic
- **`tools/tournament/src/main.rs`** — Tournament runner binary:
  - `Args` struct + `parse_args()` — CLI arg parsing (`--engine`, `--cutechess`, `--config`, `--games`, `--concurrency`, `--tc`, `--openings`)
  - `parse_config()` — hand-rolled TOML parser producing `Config { defaults, engines }`
  - `run_tournament()` — builds CuteChess-CLI command, spawns it, pipes stdout, parses `Score of` and `Elo difference` lines
  - `parse_score_line()` — extracts W/L/D from CuteChess output
  - `parse_elo_line()` — extracts Elo diff and error margin
  - `compute_elo_from_wdl()` — fallback Elo + 95% CI from W/D/L
  - `print_result()` / `print_summary()` — formatted output
  - 12 unit tests: score line parsing (valid, varied, invalid), Elo line parsing (positive, negative, invalid), Elo computation (equal, positive, negative, zero games, CI range), config file parsing