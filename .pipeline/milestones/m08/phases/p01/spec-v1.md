Good, I now have complete context. Let me produce the phase specification.

# Phase P01 — Test Infrastructure

## Goal

Build comprehensive test harnesses that validate engine correctness across move generation (extended perft suite of 100+ positions), tactical problem-solving (WAC, STS, ECM suites with solve-rate measurement), and sustained self-play (2,000+ games with randomized openings detecting crashes, illegal moves, assertion failures, and time losses) — establishing the verification foundation that all subsequent tuning and strength measurement phases depend on.

## Design Decisions

1. **Perft positions sourced from established external suites**: Positions and reference node counts must come from published, externally-validated perft suites (e.g., Kiwipete, CPW perft results, Ethereal perft collection) rather than hand-crafted positions. This ensures reference counts are trustworthy and reproducible.

2. **Tactical suites driven via UCI protocol**: The tactical test runner sends `position` and `go depth N` / `go movetime T` commands to the engine binary over UCI, then parses `bestmove` responses. This tests the engine through the same interface real GUIs use, catching integration issues that unit tests miss.

3. **Self-play framework uses UCI for engine communication**: The self-play harness launches two instances of the engine binary and communicates via UCI stdin/stdout, matching how CuteChess-CLI and other tournament managers drive engines. This validates the full engine stack end-to-end.

4. **Test infrastructure placed in `tests/` and `tools/`**: Perft validation lives as integration tests under `tests/`. The tactical test runner and self-play framework are standalone binaries under `tools/`, consistent with the project structure defined in the master plan.

5. **Both fixed-depth and fixed-time modes for tactical suites**: Supporting both modes allows solve-rate comparison across engine versions — fixed-depth measures raw tactical ability, fixed-time measures practical strength under time constraints.

## Tasks

### T01 — Extended Perft Suite

Collect 100+ perft positions with verified node counts from established test suites. Implement an integration test that runs perft on each position and asserts the node count matches the reference value. The test must cover standard, tricky, and edge-case positions (castling rights, en passant, promotions, discovered checks, pins).

**Deliverables:**
- A data file (or embedded constants) containing 100+ FEN strings with reference perft node counts at specified depths
- An integration test under `tests/` that runs perft on all positions and reports pass/fail per position
- All 100+ positions pass, confirming movegen correctness

### T02 — Tactical Test Suite

Implement a tool that loads tactical puzzle suites (WAC, STS, ECM) in EPD format, sends each position to the engine via UCI at a fixed depth or fixed time, compares the engine's `bestmove` to the expected solution, and reports the overall solve rate.

**Deliverables:**
- EPD files for WAC, STS, and ECM suites under `tests/` or a data directory
- A binary under `tools/` that runs the tactical test suite against the engine, supporting both `go depth N` and `go movetime T` modes
- Solve-rate output per suite (number solved / total, percentage)

### T03 — Self-Play Framework

Implement a tool that orchestrates self-play games between two instances of the engine via UCI. The framework uses randomized openings, runs at least 2,000 games, and detects crashes (process exit), illegal moves (engine sends invalid UCI output), assertion failures (non-zero exit code), and time losses (engine exceeds allocated time).

**Deliverables:**
- A binary under `tools/` that runs self-play tournaments via UCI
- A set of randomized opening positions (FENs or short opening move sequences) for game diversity
- Reporting of game outcomes (win/draw/loss counts) and any detected errors (crashes, illegal moves, assertion failures, time losses)
- Successful completion of 2,000+ games without crashes, illegal moves, or assertion failures

## Exit Criteria

1. Extended perft suite passes for 100+ positions from established test suites, confirming movegen correctness
2. Tactical test suite solve rates are measured for WAC, STS, and ECM at both fixed-depth and fixed-time settings
3. Self-play framework runs at least 2,000 games with randomized openings without crashes, illegal moves, or assertion failures

## Dependencies

- **M01–M07 complete**: Core types, move generation, search, UCI protocol, transposition table, advanced search techniques, and advanced evaluation must all be implemented — the test infrastructure exercises the full engine stack
- No dependency on other M08 phases — this is the first phase in the milestone

## Artifacts

- `tests/perft_suite.rs` — integration test running 100+ perft positions with reference node counts
- `tools/tactical/` — binary for running WAC, STS, ECM tactical test suites via UCI
- `tools/selfplay/` — binary for orchestrating self-play tournaments via UCI
- EPD/data files for tactical suites (WAC, STS, ECM) and randomized opening positions