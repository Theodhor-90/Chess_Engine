All verification commands pass. Here's the summary:

- **`crates/search/src/lib.rs`** — modified:
  - `let score` → `let mut score` (line 305) — fixes compilation error for LMR re-search reassignment
  - Added `lmr_enabled: true` to `tt_move_ordering_across_positions` test's `SearchContext` initializer — fixes missing field compilation error
  - Added `#[allow(clippy::needless_range_loop)]` on `lmr_table()` — suppresses lint where range loop is clearer than iterator for 2D table init
  - Replaced manual `moves_searched` counter with `(0_u32..).zip(moves.into_iter())` — fixes clippy `explicit_counter_loop` lint
  - Added test `lmr_table_values_correct` — verifies row/column 0 are zero, `table[6][4]==1`, `table[10][10]==3`
  - Added test `lmr_reduces_node_count` — benchmarks ≥30% node reduction at depth 10 on 3 positions with LMR enabled vs disabled
  - Added test `lmr_skips_tt_moves` — confirms best move is identical with/without LMR on tactical (forced mate) positions
  - Added test `lmr_re_search_on_fail_high` — confirms best move and score match with/without LMR on forced mate positions