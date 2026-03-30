# T02 — Tactical Test Suite

## Objective

Implement a tool that loads tactical puzzle suites (WAC, STS, ECM) in EPD format, sends each position to the engine via UCI at a fixed depth or fixed time, compares the engine's `bestmove` to the expected solution, and reports the overall solve rate. This measures the engine's tactical strength and enables tracking across versions.

## Deliverables

- EPD files for WAC (Win At Chess), STS (Strategic Test Suite), and ECM (Encyclopedia of Chess Middlegames) suites under `tests/` or a data directory
- A standalone binary under `tools/tactical/` that runs the tactical test suite against the engine
- Support for both `go depth N` and `go movetime T` modes (fixed-depth measures raw tactical ability, fixed-time measures practical strength)
- Solve-rate output per suite: number solved / total, percentage

## Files to Create or Modify

- **Create**: `tools/tactical/` — directory for the tactical test runner binary
- **Create**: `tools/tactical/src/main.rs` (or appropriate Cargo binary structure) — the tactical test runner
- **Create**: EPD/data files for WAC, STS, and ECM suites
- **Modify**: `Cargo.toml` (workspace root) — add the new binary to the workspace

## Key Implementation Details

- The tactical test runner communicates with the engine via UCI protocol (stdin/stdout), matching how real GUIs interact with the engine. This catches integration issues that unit tests miss.
- The runner sends `position fen <fen>` and `go depth N` or `go movetime T` commands, then parses the `bestmove` response.
- EPD format: each line contains a FEN (first 4 fields), followed by operations like `bm` (best move) indicating the expected solution.
- Both fixed-depth and fixed-time modes must be supported, selectable via command-line arguments.
- Output should clearly report per-suite solve rates.

## Dependencies

- **T01** (Extended Perft Suite): No strict code dependency, but T01 validates movegen correctness which this task implicitly relies on. Can be developed in parallel.
- Depends on M01–M07 being complete, particularly the UCI protocol (M04) since this tool communicates via UCI.

## Verification Criteria

1. The binary builds and runs successfully: `cargo run --bin tactical` (or equivalent)
2. WAC, STS, and ECM EPD suites are present and parseable
3. The tool successfully communicates with the engine via UCI, sending positions and receiving bestmove responses
4. Solve rates are reported per suite in both fixed-depth and fixed-time modes
5. Output format includes: suite name, number solved, total positions, percentage