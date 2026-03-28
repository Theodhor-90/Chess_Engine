All verification commands pass:
- `cargo fmt --check` — clean
- `cargo clippy -- -D warnings` — clean
- `cargo test -p chess-search` — 102/102 passed
- `cargo build` — success

## Files modified

- **`crates/search/src/lib.rs`** — Added `countermove_table: CounterMoveTable::new(),` to two `SearchContext` initializations in `pv_ordering_reduces_nodes` test (lines ~801 and ~837) that were missed by v2 implementation
- **`crates/search/src/ordering.rs`** — Changed test FEN in `counter_move_ordered_between_killers_and_quiet` from `"4k3/8/8/8/8/8/3q4/R3K3 w - - 0 1"` (king in check, only 1 legal quiet move) to `"4k3/8/8/8/8/8/8/R3K3 w - - 0 1"` (king not in check, many quiet moves available)