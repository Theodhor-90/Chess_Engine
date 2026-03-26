## Milestone M04 — UCI Protocol

### Goal

Make the engine usable with standard chess GUIs and testing tools. Target: ~1200 Elo (playable via UCI). The engine communicates via stdin/stdout following the Universal Chess Interface specification.

### Phases

#### Phase P01 — UCI Core
- **T01**: UCI command parser — parse `uci`, `isready`, `ucinewgame`, `position`, `go`, `stop`, `quit` commands.
- **T02**: UCI output — `id`, `uciok`, `readyok`, `bestmove`, `info` (depth, score, nodes, nps, pv, time).
- **T03**: Engine main loop — stdin/stdout communication, search thread management, `stop` signal handling via atomic flag.

#### Phase P02 — Time Management
- **T01**: Time allocation — given `wtime`, `btime`, `winc`, `binc`, `movestogo`, calculate time budget for this move. Simple formula: `time_left / moves_to_go + increment`.
- **T02**: Search integration — check elapsed time periodically during search (every N nodes), abort when time budget exceeded. Return best move found so far.
- **T03**: UCI `go` variants — support `depth`, `nodes`, `movetime`, `infinite`, `ponder` modes. Ensure `stop` always responds with `bestmove`.

### Exit Criteria
- Engine correctly responds to all standard UCI commands (`uci`, `isready`, `ucinewgame`, `position`, `go`, `stop`, `quit`)
- Engine can be loaded in standard GUIs (Arena, CuteChess) and play complete games
- Time management correctly allocates and respects time budgets without flagging
- All `go` variants (`depth`, `nodes`, `movetime`, `infinite`) work correctly
- `stop` command immediately halts search and returns `bestmove`
- All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`