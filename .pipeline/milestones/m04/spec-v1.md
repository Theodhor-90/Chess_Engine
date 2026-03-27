Good — `crates/uci/` and `crates/engine/` don't exist yet, which aligns with M04 being the milestone that introduces them. Now I have everything I need.

**Pre-Output Self-Check:**
- [x] No "Risks" or "Risk Mitigation" section
- [x] No "Out of Scope" or "Non-Goals" section
- [x] Every item traces to a specific requirement in the master plan
- [x] No global conventions restated as exit criteria
- [x] No "for future flexibility", "to avoid gaps", or "proactive" language

## Milestone M04 — UCI Protocol

### Goal

Make the engine usable with standard chess GUIs and testing tools by implementing the Universal Chess Interface (UCI) protocol, including command parsing, info output, a stdin/stdout engine main loop with search thread management, and time management that integrates with the existing search to allocate and enforce per-move time budgets. Target: ~1200 Elo (playable via UCI).

### Scope

This milestone delivers the `crates/uci/` crate (UCI command parsing and output formatting) and the `crates/engine/` top-level binary (wiring UCI, search, eval, movegen, and board together into a runnable chess engine). It includes:

- Parsing all standard UCI commands (`uci`, `isready`, `ucinewgame`, `position`, `go`, `stop`, `quit`)
- Producing all required UCI output (`id`, `uciok`, `readyok`, `bestmove`, `info` lines with depth, score, nodes, nps, pv, time)
- An engine main loop with stdin/stdout communication, search thread management, and `stop` signal handling via atomic flag
- Time allocation from UCI time parameters (`wtime`, `btime`, `winc`, `binc`, `movestogo`)
- Search integration that checks elapsed time periodically and aborts when the time budget is exceeded
- Support for all `go` variants: `depth`, `nodes`, `movetime`, `infinite`, `ponder`

### Phases

#### Phase P01 — UCI Core

Implement the UCI protocol layer: command parsing, output formatting, and the engine main loop.

- **T01**: UCI command parser — parse `uci`, `isready`, `ucinewgame`, `position`, `go`, `stop`, `quit` commands from stdin text into structured representations.
- **T02**: UCI output — format and emit `id`, `uciok`, `readyok`, `bestmove`, `info` (depth, score, nodes, nps, pv, time) responses to stdout.
- **T03**: Engine main loop — stdin/stdout communication loop, search thread management, `stop` signal handling via atomic flag. This is the `crates/engine/` binary that wires UCI, search, eval, movegen, and board together.

#### Phase P02 — Time Management

Add time-aware search control so the engine plays under clock constraints without flagging.

- **T01**: Time allocation — given `wtime`, `btime`, `winc`, `binc`, `movestogo`, calculate the time budget for the current move. Simple formula: `time_left / moves_to_go + increment`.
- **T02**: Search integration — check elapsed time periodically during search (every N nodes), abort when time budget is exceeded, and return the best move found so far.
- **T03**: UCI `go` variants — support `depth`, `nodes`, `movetime`, `infinite`, `ponder` modes. Ensure `stop` always responds with `bestmove`.

### Exit Criteria

1. Engine correctly responds to all standard UCI commands (`uci`, `isready`, `ucinewgame`, `position`, `go`, `stop`, `quit`)
2. Engine can be loaded in standard GUIs (Arena, CuteChess) and play complete games
3. Time management correctly allocates and respects time budgets without flagging
4. All `go` variants (`depth`, `nodes`, `movetime`, `infinite`) work correctly
5. `stop` command immediately halts search and returns `bestmove`

### Dependencies

- **M01** (Core Types & Board Representation) — `Position`, `Move`, `Bitboard`, FEN parsing, Zobrist hashing, make/unmake move
- **M02** (Move Generation) — legal move generation for all piece types
- **M03** (Basic Search & Evaluation) — negamax with alpha-beta, quiescence search, iterative deepening, evaluation function, move ordering