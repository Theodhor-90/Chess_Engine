## Phase P01 — Test Infrastructure

### Goal

Build comprehensive test harnesses that validate engine correctness across move generation, tactical problem-solving, and sustained self-play. This phase establishes the verification foundation that all subsequent tuning and strength measurement phases depend on.

### Deliverables

- **Extended perft suite** (T01): Validate movegen against 100+ positions from established test suites. Regression-proof the move generator so future changes cannot silently break correctness.
- **Tactical test suite** (T02): Integrate WAC (Win At Chess), STS (Strategic Test Suite), and ECM (Encyclopedia of Chess Middlegames) problem sets. Measure solve rate at fixed depth/time to quantify tactical strength.
- **Self-play framework** (T03): Engine plays itself with randomized openings. Detect crashes, illegal moves, assertion failures, and time losses over at least 2,000 games.

### Technical Constraints

- Perft positions must come from established, externally-validated test suites (not hand-crafted) to ensure reference counts are trustworthy.
- Tactical suites must support both fixed-depth and fixed-time modes so solve rate can be compared across engine versions.
- Self-play must use randomized openings to ensure diverse position coverage and must run at least 2,000 games to surface rare crash/illegal-move bugs.
- The engine binary communicates via UCI — self-play framework must drive the engine through the UCI protocol.
- All test infrastructure lives under `tests/` or `tools/` per the project structure conventions.
- Rust conventions apply: `cargo test` for unit/integration tests, `criterion` for benchmarks, `thiserror`/`anyhow` for error handling, no `unwrap()` in library crates.

### Dependencies

- **M01–M07 must be complete**: core types, move generation, search, UCI protocol, transposition table, advanced search, and advanced evaluation are all prerequisites.
- No dependency on other M08 phases — this is the first phase in the milestone.