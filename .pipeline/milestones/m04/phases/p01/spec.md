## Phase P01 — UCI Core

### Goal

Implement the UCI (Universal Chess Interface) protocol layer for the chess engine, comprising command parsing from stdin text into structured representations, output formatting for all required UCI responses, and the engine main loop binary (`crates/engine/`) that wires UCI communication with the existing search, eval, movegen, and board crates into a runnable chess engine with stdin/stdout communication and search thread management.

### Deliverables

- **T01 — UCI command parser**: Parse `uci`, `isready`, `ucinewgame`, `position`, `go`, `stop`, `quit` commands from stdin text into structured Rust representations. The parser must handle all standard UCI command syntax including `position startpos moves ...` and `position fen <fen> moves ...`, as well as `go` with its various parameters (`wtime`, `btime`, `winc`, `binc`, `movestogo`, `depth`, `nodes`, `movetime`, `infinite`, `ponder`).
- **T02 — UCI output**: Format and emit `id` (name, author), `uciok`, `readyok`, `bestmove`, and `info` lines (with depth, score, nodes, nps, pv, time fields) to stdout. Output must conform to the UCI specification so standard GUIs can parse it.
- **T03 — Engine main loop**: Build the `crates/engine/` top-level binary with a stdin/stdout communication loop that reads UCI commands, dispatches them, manages a search thread, and handles the `stop` signal via an atomic flag. This binary wires together the `uci`, `search`, `eval`, `movegen`, and `board` crates.

### Technical Decisions & Constraints

- This phase creates two crates: `crates/uci/` (library for parsing and formatting) and `crates/engine/` (binary).
- The engine binary communicates exclusively via stdin/stdout per the UCI spec.
- Search must run on a separate thread so the main loop remains responsive to `stop` and `quit` commands.
- `stop` signal handling uses a shared atomic flag that the search checks periodically.
- No `unwrap()` in the `uci` library crate; use `Result`/`Option` propagation. `unwrap()` is allowed in the `engine` binary.
- Error handling: `thiserror` for library errors in `crates/uci/`, `anyhow` for the `crates/engine/` binary entry point.
- Minimize external dependencies; core protocol logic should rely only on the standard library.

### Dependencies

- **M01** (Core Types & Board Representation) — `Position`, `Move`, `Bitboard`, FEN parsing, Zobrist hashing, make/unmake move
- **M02** (Move Generation) — legal move generation for all piece types
- **M03** (Basic Search & Evaluation) — negamax with alpha-beta, quiescence search, iterative deepening, evaluation function, move ordering
- No dependency on prior phases within this milestone (this is the first phase).