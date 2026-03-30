# T01 — Extended Perft Suite

## Objective

Collect 100+ perft positions with verified node counts from established, externally-validated test suites and implement an integration test that runs perft on each position, asserting correctness. This regression-proofs the move generator.

## Deliverables

- A data file (or embedded constants) containing 100+ FEN strings with reference perft node counts at specified depths, sourced from published suites (Kiwipete, CPW perft results, Ethereal perft collection, etc.)
- An integration test under `tests/perft_suite.rs` that runs perft on all positions and reports pass/fail per position
- All 100+ positions must pass, confirming movegen correctness

## Files to Create or Modify

- **Create**: `tests/perft_suite.rs` — integration test running 100+ perft positions with reference node counts
- **Create or modify**: A data file or embedded constant block containing the 100+ FEN strings with reference counts and depths
- **May reference**: existing `perft` function in the codebase (likely in `crates/movegen/` or `tools/`)

## Key Implementation Details

- Positions and reference node counts must come from published, externally-validated perft suites — not hand-crafted positions. This ensures reference counts are trustworthy and reproducible.
- The suite must cover standard positions, tricky positions, and edge cases: castling rights, en passant, promotions, discovered checks, pins.
- Must include at least 100 positions.
- Perft validation lives as integration tests under `tests/`, consistent with the project structure defined in the master plan.

## Dependencies

- None within this phase. This is the first task and has no dependency on T02 or T03.
- Depends on M01–M07 being complete (core types, move generation, search, UCI, TT, advanced search, advanced evaluation).

## Verification Criteria

1. `cargo test --test perft_suite` (or equivalent) runs successfully
2. At least 100 distinct positions are tested
3. All positions pass — computed perft node counts match published reference values
4. Positions cover edge cases: castling rights, en passant, promotions, discovered checks, pins