# T03 — UCI `go` Variants

## Objective

Implement full support for all `go` sub-commands so the engine handles every standard UCI search mode: `depth`, `nodes`, `movetime`, `infinite`, and `ponder`. The `GoParams` struct fields already parsed in Phase P01 are now acted upon.

## Deliverables

Full support for all `go` sub-commands:

- **`go depth N`**: Search to exactly depth N and stop. Return `bestmove`.
- **`go nodes N`**: Search until at least N nodes have been visited, then stop. May overshoot by up to 1024 nodes due to the check interval. Return `bestmove`.
- **`go movetime N`**: Search for at least N milliseconds, then stop. May overshoot by the duration of one check interval. Return `bestmove`.
- **`go infinite`**: Search until `stop` is received, then return `bestmove`.
- **`go ponder`**: Search on the opponent's time without a time constraint. On `ponderhit`, switch to normal time management for the remaining budget. On `stop`, immediately return `bestmove`.
- **`stop`** command always causes the engine to emit `bestmove`.

## Files to Modify

- **`crates/search/src/lib.rs`** — add support for `depth`, `nodes`, `movetime`, `infinite` limits in the search loop; add `ponder` support
- **`crates/engine/src/main.rs`** — update the `Go` handler to detect which variant is active and configure the search accordingly; add `ponderhit` command handling

## Key Constraints

- Node counting uses the existing 1024-node check interval — `nodes` limit may overshoot by up to 1024 nodes
- `movetime` limit may overshoot by the duration of one check interval
- During `ponder` mode, the engine searches without a time constraint. On `ponderhit`, it switches to normal time management. On `stop`, it returns `bestmove`.
- `stop` must **always** result in a `bestmove` response, regardless of which `go` variant is active
- The `GoParams` struct fields were already parsed in Phase P01 — this task makes them functional

## Dependencies

- **T01** (Time Allocation) — for `ponderhit` switching to normal time management
- **T02** (Search Integration) — for the time-budget-aware search mechanism
- Phase P01 (UCI Core) — `GoParams` parsing, engine main loop, stop flag, `bestmove` output formatting

## Verification Criteria

1. `go depth N` searches to exactly depth N and returns `bestmove`
2. `go nodes N` searches until at least N nodes visited and returns `bestmove` (may overshoot by up to 1024)
3. `go movetime N` searches for at least N milliseconds and returns `bestmove` (may overshoot by one check interval)
4. `go infinite` searches until `stop` is received, then returns `bestmove`
5. `go ponder` searches until `ponderhit` (switch to time management) or `stop` (return `bestmove`)
6. `stop` command always halts search and responds with `bestmove`
7. `cargo test` passes
8. `cargo clippy -- -D warnings` passes
9. `cargo fmt --check` passes