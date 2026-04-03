## Task T01 — UCI Command Parser

### Objective

Create the `crates/uci/` library crate with a UCI command parser that converts stdin text lines into structured `UciCommand` enum variants. This is the input half of the UCI protocol layer.

### Deliverables

- **New crate**: `crates/uci/` with `Cargo.toml` and `src/lib.rs`
- **`UciCommand` enum** with variants:
  - `Uci` — the `uci` command
  - `IsReady` — the `isready` command
  - `UciNewGame` — the `ucinewgame` command
  - `Position` — carries a FEN string (or startpos indicator) and an optional move list
  - `Go` — carries a `GoParams` struct
  - `Stop` — the `stop` command
  - `Quit` — the `quit` command
- **`GoParams` struct** with optional fields: `wtime`, `btime`, `winc`, `binc`, `movestogo`, `depth`, `nodes`, `movetime`, `infinite` (bool), `ponder` (bool)
- **`parse` function** — `fn parse(input: &str) -> Result<UciCommand, UciError>` (or similar signature)
- Unit tests for all command variants

### Implementation Details

- Parse `position startpos [moves ...]` and `position fen <fen> [moves ...]` — the FEN is 6 space-separated fields; moves follow the `moves` keyword
- Parse `go` with any combination of optional parameters: `wtime`, `btime`, `winc`, `binc`, `movestogo`, `depth`, `nodes`, `movetime`, `infinite`, `ponder`
- All other commands (`uci`, `isready`, `ucinewgame`, `stop`, `quit`) are simple keyword matches
- Handle edge cases: empty input, unknown commands, malformed `position` and `go` arguments
- Use `thiserror` for error types in the library crate
- No `unwrap()` in the library crate — use `Result`/`Option` propagation
- The crate should depend only on the standard library plus `thiserror`
- No I/O in this crate — it operates on string inputs and produces structured data

### Dependencies

- No dependencies on other tasks within this phase (this is the first task)
- Depends on M01 types crate for `Move` and `Square` types (for the move list in `Position` variant), though the parser may store moves as raw strings and defer parsing to the engine layer

### Verification Criteria

1. `cargo test -p uci` passes with unit tests covering:
   - All 7 command variants parsed correctly
   - `position startpos` with and without moves
   - `position fen <valid fen>` with and without moves
   - `go` with various parameter combinations (all params, some params, no params)
   - `go infinite`, `go depth 5`, `go movetime 1000`, etc.
   - Empty input returns an error
   - Unknown commands return an error (or are handled gracefully)
   - Malformed input returns appropriate errors
2. `cargo clippy -p uci -- -D warnings` passes
3. `cargo fmt -p uci --check` passes