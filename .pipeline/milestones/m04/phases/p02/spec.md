## Phase P02 — Time Management

### Goal

Add time-aware search control so the engine can play under clock constraints without flagging, by implementing a time allocation algorithm that computes per-move time budgets from UCI clock parameters, integrating time checking into the search loop to abort when the budget is exceeded, and supporting all UCI `go` variants (`depth`, `nodes`, `movetime`, `infinite`, `ponder`) with correct `stop`/`bestmove` behavior.

### Deliverables

- **T01 — Time allocation**: Given UCI parameters `wtime`, `btime`, `winc`, `binc`, `movestogo`, calculate the time budget for the current move. The initial formula is `time_left / moves_to_go + increment`. When `movestogo` is not provided, use a reasonable default estimate.
- **T02 — Search integration**: Modify the search to check elapsed time periodically (every N nodes) and abort when the time budget is exceeded, returning the best move found so far from the deepest completed iteration.
- **T03 — UCI `go` variants**: Implement support for `depth` (search to fixed depth), `nodes` (search fixed number of nodes), `movetime` (search for fixed milliseconds), `infinite` (search until `stop`), and `ponder` (search on opponent's time) modes. Ensure `stop` always causes the engine to respond with `bestmove`.

### Technical Decisions & Constraints

- Time checking happens every N nodes (not every node) to minimize overhead — a typical interval is every 1024 or 2048 nodes.
- The search must be interruptible: the atomic `stop` flag from P01 is reused for time-based abort as well.
- When time expires mid-search during iterative deepening, the engine returns the best move from the last fully completed iteration (not a partially searched iteration).
- `ponder` mode means the engine searches during the opponent's turn; when a `ponderhit` is received, it switches to normal time management. When `stop` is received during ponder, it returns `bestmove`.
- The time management module should be testable independently of the search (i.e., pure function from clock parameters to time budget).

### Dependencies

- **Phase P01 — UCI Core**: The engine main loop, command parser (to receive `go` parameters), output formatting (to emit `bestmove`), and the atomic stop flag are all prerequisites.
- **M03** (Basic Search & Evaluation) — The iterative deepening search framework that this phase modifies to be time-aware.