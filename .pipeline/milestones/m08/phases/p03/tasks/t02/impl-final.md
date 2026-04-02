All verification commands pass. Summary of changes:

- **`tools/sprt/Cargo.toml`** — removed `chess-types`, `chess-board`, `chess-movegen` dependencies; `[dependencies]` section is now empty
- **`tools/sprt/src/main.rs`** — complete rewrite:
  - Replaced direct engine-driving implementation with CuteChess-CLI wrapper
  - `Args` struct with `--baseline`, `--candidate`, `--cutechess` (required), plus optional SPRT params with defaults (`elo0=0`, `elo1=5`, `alpha=0.05`, `beta=0.05`, `batch-size=100`, `max-games=10000`, `tc=10+0.1`, `concurrency=1`, `openings`)
  - `parse_args()` — manual arg parsing with defaults for optional params
  - `print_usage()` — help text to stderr
  - `run_batch()` — invokes CuteChess-CLI with Candidate as first `-engine` arg, parses score line, returns `(candidate_wins, candidate_losses, draws)`
  - `parse_score_line()` — parses `"Score of X vs Y: W - L - D [pct]"` format
  - `elo_to_score()` — Elo to expected score conversion
  - `log_likelihood_ratio()` — trinomial LLR computation
  - `sprt_bounds()` — decision threshold computation
  - `main()` — batch loop with LLR checking, status output, exit codes (0=ACCEPT, 1=REJECT, 2=INCONCLUSIVE)
  - 9 unit tests: `test_sprt_bounds`, `test_elo_to_score_zero`, `test_elo_to_score_positive`, `test_llr_equal_score`, `test_llr_positive`, `test_llr_negative`, `test_llr_zero_games`, `test_parse_score_line_valid`, `test_parse_score_line_invalid`