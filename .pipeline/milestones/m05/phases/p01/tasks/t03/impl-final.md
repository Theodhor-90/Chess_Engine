All verification commands passed. Here's the summary:

**Files modified:**

- **`crates/search/src/tt.rs`** — Added `MAX_PLY` constant, `score_to_tt`, `score_from_tt` helpers; added 7 unit tests (positive/negative mate, non-mate, round-trip)
- **`crates/search/src/ordering.rs`** — Added `TT_SCORE` (200,000) constant; added `tt_move: Option<Move>` param to `score_move`/`order_moves`; TT move ranked above PV; updated existing test call sites; added `tt_move_ordered_before_pv` test
- **`crates/search/src/lib.rs`** — Added `tt: TranspositionTable` to `SearchContext`; TT probe at negamax entry (cutoffs skipped at root); TT store at negamax exit with `original_alpha`-based bound determination; `tt_move` passed to `order_moves`; `new_generation()` called before ID loop; updated `test_ctx()` helper; added `tt_reduces_node_count`, `tt_move_is_ordered_first`, `mate_score_correct_with_tt` tests; fixed mate test to use valid back-rank mate position (`6k1/5ppp/8/8/8/8/8/3Q1RK1`)