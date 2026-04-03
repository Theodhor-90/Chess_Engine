## Phase P01 — UCI Core

### Goal

Implement the UCI (Universal Chess Interface) protocol layer comprising command parsing from stdin text into structured representations, output formatting for all required UCI responses, and the engine main loop binary (`crates/engine/`) that wires UCI communication with the existing search, eval, movegen, and board crates into a runnable chess engine with stdin/stdout communication, search thread management, and `stop` signal handling via atomic flag.

### Design Decisions

1. **Two-crate split**: This phase creates `crates/uci/` (library crate for parsing and formatting) and `crates/engine/` (binary crate). The library crate handles UCI protocol logic with no I/O — it operates on string inputs and produces string outputs. The binary crate owns stdin/stdout and threading.

2. **Structured command representation**: UCI commands are parsed into a `UciCommand` enum with variants for each command (`Uci`, `IsReady`, `UciNewGame`, `Position`, `Go`, `Stop`, `Quit`). The `Position` variant carries a FEN string (or startpos indicator) and a move list. The `Go` variant carries a `GoParams` struct with optional fields for `wtime`, `btime`, `winc`, `binc`, `movestogo`, `depth`, `nodes`, `movetime`, `infinite`, `ponder`.

3. **Atomic stop flag**: The engine main loop creates an `Arc<AtomicBool>` stop flag shared between the main thread and the search thread. The main thread sets the flag to `true` upon receiving `stop` or `quit`. The search crate's `SearchContext` already checks for abort conditions every 1024 nodes (`ctx.nodes & 1023 == 0` in the `negamax` and `quiescence` functions, calling `check_time`). This phase extends `SearchContext` to accept an `Arc<AtomicBool>` and check it within the existing `check_time` method — setting `self.aborted = true` if the flag is set. No change to the check frequency or node-count interval; this phase only adds the stop flag as an additional abort source within the existing 1024-node check cadence.

4. **Search thread model**: The `go` command spawns a search thread via `std::thread::spawn`. Only one search can be active at a time. The main loop remains responsive to `stop` and `quit` while search runs. When search completes or is stopped, the engine emits `bestmove`.

5. **Error handling**: `thiserror` for library errors in `crates/uci/`; `anyhow` for the `crates/engine/` binary. No `unwrap()` in the `uci` library crate.

6. **Minimal dependencies**: Core protocol logic in `crates/uci/` depends only on the standard library (plus `thiserror` for error types). The `crates/engine/` binary depends on `uci`, `search`, `eval`, `movegen`, `board`, `types`, and `anyhow`.

### Tasks

**T01 — UCI command parser**

Parse `uci`, `isready`, `ucinewgame`, `position`, `go`, `stop`, `quit` commands from stdin text into a `UciCommand` enum. The parser handles:
- `position startpos [moves ...]` and `position fen <fen> [moves ...]`
- `go` with optional parameters: `wtime`, `btime`, `winc`, `binc`, `movestogo`, `depth`, `nodes`, `movetime`, `infinite`, `ponder`
- All other commands as simple keyword matches

Deliverables: `crates/uci/` crate with `UciCommand` enum, `GoParams` struct, and `parse` function. Unit tests for all command variants including edge cases (empty input, unknown commands, malformed `position` and `go` arguments).

**T02 — UCI output**

Format and emit `id` (name, author), `uciok`, `readyok`, `bestmove`, and `info` lines (with depth, score, nodes, nps, pv, time fields) to stdout. Output conforms to the UCI specification so standard GUIs can parse it. Moves in `bestmove` and `pv` use UCI coordinate notation (e.g., `e2e4`, `a7a8q`) via the existing `Move::Display` implementation.

Deliverables: UCI output formatting functions/types in `crates/uci/`. Unit tests verifying output strings match UCI specification format.

**T03 — Engine main loop**

Build the `crates/engine/` top-level binary with a stdin/stdout communication loop that reads UCI commands, dispatches them to handlers, manages a search thread, and handles the `stop` signal via a shared `Arc<AtomicBool>`. This binary wires together the `uci`, `search`, `eval`, `movegen`, `board`, and `types` crates into a runnable chess engine.

This task extends the search crate's `SearchContext` to accept an `Arc<AtomicBool>` stop flag. The existing `check_time` method (called every 1024 nodes) is extended to also check the stop flag and set `self.aborted = true` if the flag is set. No change to the check frequency or node-count interval.

Deliverables: `crates/engine/` binary crate with `main` function. The engine can be loaded in a UCI-compatible GUI (Arena, CuteChess), respond to the UCI handshake, accept positions, run searches, and return `bestmove`. Workspace `Cargo.toml` updated to include both new crates.

### Exit Criteria

1. The engine correctly parses all standard UCI commands: `uci`, `isready`, `ucinewgame`, `position` (startpos and fen variants with move lists), `go` (with all parameter variants), `stop`, `quit`
2. The engine emits correct UCI responses: `id name`, `id author`, `uciok`, `readyok`, `bestmove`, and `info` lines with depth, score, nodes, nps, pv, and time
3. The engine main loop runs a search on a separate thread and remains responsive to `stop` and `quit` during search
4. `stop` immediately halts the search and the engine responds with `bestmove`
5. All unit tests in `crates/uci/` pass and the `crates/engine/` binary compiles and runs successfully

### Dependencies

- **M01** (Core Types & Board Representation) — `Position`, `Move`, `Square`, `Bitboard`, FEN parsing, Zobrist hashing, make/unmake move
- **M02** (Move Generation) — legal move generation for all piece types
- **M03** (Basic Search & Evaluation) — `search()` function with iterative deepening, negamax with alpha-beta, quiescence search, evaluation, move ordering
- No dependency on prior phases within M04 (this is the first phase)

### Artifacts

- `crates/uci/Cargo.toml` — new crate manifest
- `crates/uci/src/lib.rs` — UCI command parser, output formatter, error types
- `crates/engine/Cargo.toml` — new binary crate manifest
- `crates/engine/src/main.rs` — engine main loop binary
- `Cargo.toml` — workspace root updated to include `crates/uci` and `crates/engine`