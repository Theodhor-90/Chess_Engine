The background task already completed and I read the results earlier. All tests passed successfully.

To summarize the final state — all 4 review issues are already resolved in the current code:

- **`crates/search/src/lib.rs`**: `order_moves` calls in `tt_move_ordered_first_after_search` (line 1012) and `tt_move_ordering_multiple_positions` (line 1108) include `&history` and `Color::White` params
- **`crates/search/src/ordering.rs`**: `#[allow(clippy::too_many_arguments)]` on `score_move` (line 34) and `order_moves` (line 58)
- **`crates/search/src/lib.rs`**: `needless_range_loop` fixed at line 415 — uses `for prev_mv in searched_quiets.iter().take(...)` iterator
- **`crates/search/src/lib.rs`**: formatting correct — `cargo fmt --check` passes

Verification:
- `cargo fmt --check` — pass
- `cargo clippy -- -D warnings` — pass
- `cargo test --workspace` — 363 tests, all pass