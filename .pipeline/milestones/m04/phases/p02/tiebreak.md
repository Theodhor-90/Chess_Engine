# Phase P02 — Time Management

## Goal

Add time-aware search control so the engine can play under clock constraints without flagging, by implementing a dedicated time allocation module that computes per-move time budgets from UCI clock parameters (`wtime`, `btime`, `winc`, `binc`, `movestogo`), integrating periodic time checking into the search loop to abort when the budget is exceeded, and supporting all UCI `go` variants (`depth`, `nodes`, `movetime`, `infinite`, `ponder`) with correct `stop`/`bestmove` behavior.

## Design Decisions

1. **Time allocation as a pure function**: The time budget calculation lives in a standalone, testable module (or function) that takes clock parameters and the side to move, and returns a `Duration`. This keeps time logic decoupled from search internals and independently unit-testable.

2. **Reuse of the existing atomic stop flag**: The `Arc<AtomicBool>` stop flag established in Phase P01 serves dual duty — it is set both by the `stop` UCI command and by the time management system when the budget expires. No additional signaling mechanism is needed.

3. **Node-interval time checking**: Time is checked every 1024 nodes (already present in `SearchContext::check_time`) rather than every node, to minimize the performance overhead of `Instant::elapsed()` system calls. This interval is sufficient for millisecond-level time control accuracy.

4. **Return best move from last completed iteration**: When time expires mid-iteration during iterative deepening, the engine returns the best move from the deepest *fully completed* iteration, not a partially searched one. This ensures the returned move has a complete evaluation behind it.

5. **Default `movestogo` value of 25**: When `movestogo` is not provided by the GUI (sudden-death time control), the engine uses a fixed default of 25 moves remaining to avoid spending too much time early or flagging late.

6. **`ponder` mode**: During ponder, the engine searches without a time constraint. On `ponderhit`, it switches to normal time management for the remaining budget. On `stop`, it immediately returns `bestmove`.

## Tasks

### T01 — Time Allocation

**Deliverables**: A time allocation function (or small module) that, given `wtime`, `btime`, `winc`, `binc`, `movestogo`, and the side to move, returns a `Duration` representing the time budget for the current move. The formula is `time_left / moves_to_go + increment`, where `moves_to_go` defaults to 25 when `movestogo` is absent. This replaces the hardcoded `Duration::from_secs(5)` fallback currently in `crates/engine/src/main.rs` (lines 121–128) where the `Go` handler assigns a fixed 5-second budget when `wtime`/`btime` are present. The new function lives in `crates/uci/` (or a dedicated `time` submodule).

Unit tests verify correct budgets for the following clock scenarios:

- Sudden death with no increment (e.g., 5 minutes remaining, no `movestogo`, no increment)
- Sudden death with increment (e.g., 5 minutes remaining + 3 second increment, no `movestogo`)
- `movestogo` provided with increment (e.g., 2 minutes remaining, 20 moves to go, 5 second increment)
- `movestogo` provided without increment (e.g., 2 minutes remaining, 20 moves to go, zero increment)
- Very low time remaining (under 1 second, with and without increment)
- Zero increment with no `movestogo` (pure sudden death)

### T02 — Search Integration

**Deliverables**: Modify the search to use the time allocation from T01 as its time budget. The existing `SearchContext::check_time` mechanism (every 1024 nodes) enforces the budget by setting `aborted = true` when elapsed time exceeds the allocation. When aborted mid-iteration in iterative deepening, the engine returns the best move from the last fully completed iteration. The engine main loop in `crates/engine/src/main.rs` is updated to call the T01 time allocation function and pass the computed budget to `search()`. Tests verify that search aborts correctly when the time budget expires and returns a valid best move.

### T03 — UCI `go` Variants

**Deliverables**: Full support for all `go` sub-commands: `depth` (search to a fixed depth and stop), `nodes` (search until at least N nodes have been visited, then stop — may overshoot by up to 1024 nodes due to the check interval), `movetime` (search for at least N milliseconds, then stop — may overshoot by the duration of one check interval), `infinite` (search until `stop` is received), and `ponder` (search on the opponent's time; respond to `ponderhit` by switching to normal time management, respond to `stop` by returning `bestmove`). The `GoParams` struct fields already parsed in P01 are now acted upon. The `stop` command always causes the engine to emit `bestmove`. Tests verify each variant produces correct behavior.

## Exit Criteria

1. Time allocation correctly computes a per-move budget from `wtime`, `btime`, `winc`, `binc`, and `movestogo` (defaulting to 25 when `movestogo` is absent)
2. Search aborts when the time budget is exceeded and returns the best move from the last fully completed iteration
3. `go depth N` searches to exactly depth N and returns `bestmove`
4. `go nodes N` searches until at least N nodes have been visited and returns `bestmove` (may overshoot by up to 1024 nodes due to the check interval)
5. `go movetime N` searches for at least N milliseconds and returns `bestmove` (may overshoot by the duration of one check interval)
6. `go infinite` searches until `stop` is received, then returns `bestmove`
7. `go ponder` searches until `ponderhit` (switch to time management) or `stop` (return `bestmove`)
8. `stop` command always halts search and responds with `bestmove`

## Dependencies

- **Phase P01 — UCI Core**: The engine main loop, UCI command parser (including `GoParams` with all time fields), UCI output formatting (`bestmove`, `info`), and the `Arc<AtomicBool>` stop flag
- **M03 — Basic Search & Evaluation**: The iterative deepening search framework (`chess_search::search`), `SearchContext` with its `check_time` mechanism, negamax, quiescence search, and evaluation

## Artifacts

- New or extended time allocation module/function (in `crates/uci/` or a dedicated `time` submodule)
- Modified `crates/search/src/lib.rs` — search integration for `depth`, `nodes`, `movetime`, `infinite` limits and `ponder` support
- Modified `crates/engine/src/main.rs` — updated `Go` handler to compute time budget via the allocation function and to handle `ponderhit`
- Unit tests for time allocation across the six enumerated clock scenarios
- Integration tests verifying each `go` variant and `stop` behavior