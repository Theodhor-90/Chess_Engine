## Task T03 — Engine Main Loop

### Objective

Build the `crates/engine/` top-level binary crate with a stdin/stdout communication loop that reads UCI commands, dispatches them to handlers, manages a search thread, and handles the `stop` signal via a shared `Arc<AtomicBool>`. This binary wires together the `uci`, `search`, `eval`, `movegen`, `board`, and `types` crates into a runnable chess engine. Additionally, extend the search crate's `SearchContext` to accept and check an external stop flag.

### Deliverables

- **New crate**: `crates/engine/` with `Cargo.toml` and `src/main.rs`
- **Workspace update**: Root `Cargo.toml` updated to include both `crates/uci` and `crates/engine` as workspace members
- **Engine main loop** in `main.rs` that:
  - Reads lines from stdin, parses them via the `uci` crate parser
  - Dispatches each `UciCommand` to appropriate handlers
  - Handles `uci` → responds with `id name`, `id author`, `uciok`
  - Handles `isready` → responds with `readyok`
  - Handles `ucinewgame` → resets engine state (position, search tables)
  - Handles `position` → sets up the internal `Position` using FEN parsing from the `board` crate and applies moves via `make_move`
  - Handles `go` → spawns a search thread via `std::thread::spawn`, shares an `Arc<AtomicBool>` stop flag, runs iterative deepening search, emits `info` lines during search, emits `bestmove` when search completes or is stopped
  - Handles `stop` → sets the atomic stop flag to `true`, waits for search thread to finish, ensures `bestmove` is emitted
  - Handles `quit` → sets stop flag, waits for search thread, exits cleanly
- **Search crate modification**: Extend `SearchContext` to accept an `Arc<AtomicBool>` stop flag. The existing `check_time` method (called every 1024 nodes via `ctx.nodes & 1023 == 0`) is extended to also check the stop flag and set `self.aborted = true` if the flag is set. No change to check frequency or node-count interval.
- Only one search can be active at a time

### Implementation Details

- The binary depends on: `uci`, `search`, `eval`, `movegen`, `board`, `types`, `anyhow`
- Use `anyhow` for error handling in the binary; `unwrap()` is acceptable in binary code but prefer proper error handling
- Search thread model: `go` spawns a thread, main loop continues reading stdin for `stop`/`quit`. When search finishes (naturally or via stop), emit `bestmove`
- The stop flag is `Arc<AtomicBool>` — main thread sets it to `true`, search thread checks it in `check_time`
- Reset the stop flag to `false` before each new `go` command
- For this phase (P01), time management is not yet implemented — the `go` command will use whatever time/depth limits are directly available (e.g., `go depth 5` uses fixed depth, `go infinite` searches until stopped). Full time management comes in P02.
- The engine must be loadable by UCI-compatible GUIs (Arena, CuteChess) and complete the UCI handshake

### Files to Create or Modify

- `crates/engine/Cargo.toml` — new binary crate manifest
- `crates/engine/src/main.rs` — engine main loop
- `Cargo.toml` — workspace root, add `crates/uci` and `crates/engine` to members
- `crates/search/src/*.rs` — modify `SearchContext` to accept and check `Arc<AtomicBool>` stop flag in `check_time`

### Dependencies

- Depends on **T01** (UCI command parser) and **T02** (UCI output formatting)
- Depends on all prior milestone crates: `types`, `board`, `movegen`, `eval`, `search`

### Verification Criteria

1. `cargo build -p engine` compiles successfully
2. The engine binary can be run and responds correctly to the UCI handshake:
   - Send `uci` → receive `id name ...`, `id author ...`, `uciok`
   - Send `isready` → receive `readyok`
3. The engine accepts `position startpos` and `position fen <fen> [moves ...]`
4. The engine accepts `go depth 5` (or similar) and responds with `info` lines and `bestmove`
5. The engine accepts `stop` during search and immediately responds with `bestmove`
6. The engine accepts `quit` and exits cleanly
7. `cargo test --workspace` passes (including new and existing tests)
8. `cargo clippy --workspace -- -D warnings` passes
9. `cargo fmt --all --check` passes
10. The engine can be loaded in CuteChess or Arena and play a complete game (manual verification)