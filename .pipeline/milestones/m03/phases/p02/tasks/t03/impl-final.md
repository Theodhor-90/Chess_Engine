All verification commands pass. Here's the summary:

- **`crates/search/src/lib.rs`** — Modified:
  - Added `use std::time::{Duration, Instant}`
  - Added `pub struct SearchContext` with fields `start`, `time_budget`, `nodes`, `aborted` and `check_time()` method (checks every 1024 nodes)
  - Modified `quiescence` signature: added `ctx: &mut SearchContext` param, node counting, periodic time check, early abort on `ctx.aborted`
  - Modified `negamax` signature: added `ctx: &mut SearchContext` param, node counting, periodic time check, early abort on `ctx.aborted`
  - Added `pub fn search(pos, time_budget) -> Option<Move>`: iterative deepening loop (depth 1, 2, 3, …), returns best move from last fully completed depth, stops on mate or time expiry
  - Updated all 11 existing tests to pass `&mut SearchContext` via `test_ctx()` helper
  - Added 6 new tests: `iterative_deepening_returns_legal_move`, `iterative_deepening_finds_better_moves_at_depth`, `search_respects_time_budget`, `search_returns_none_for_checkmate`, `search_returns_none_for_stalemate`, `node_counter_increments`