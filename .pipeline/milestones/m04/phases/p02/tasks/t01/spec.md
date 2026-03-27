# T01 — Time Allocation

## Objective

Implement a time allocation function (or small module) that computes a per-move time budget from UCI clock parameters. This replaces the hardcoded `Duration::from_secs(5)` fallback currently in `crates/engine/src/main.rs` (lines 121–128).

## Deliverables

A pure function that, given `wtime`, `btime`, `winc`, `binc`, `movestogo`, and the side to move, returns a `Duration` representing the time budget for the current move.

**Formula**: `time_left / moves_to_go + increment`
- `moves_to_go` defaults to **25** when `movestogo` is absent (sudden-death time control)
- `time_left` is `wtime` or `btime` depending on the side to move
- `increment` is `winc` or `binc` depending on the side to move

## Files to Create or Modify

- **New**: Time allocation module/function in `crates/uci/` (or a dedicated `time` submodule within it)
- The function must be public and usable from `crates/engine/`

## Key Constraints

- Time allocation must be a **pure function** — takes clock parameters and side to move, returns a `Duration`. Decoupled from search internals, independently unit-testable.
- Must handle edge cases: very low time remaining (under 1 second), zero increment, missing `movestogo`, etc.
- Must not introduce new dependencies beyond the standard library

## Dependencies

- None within this phase (this is the first task)
- Depends on Phase P01 (UCI Core) for the `GoParams` struct that already parses `wtime`, `btime`, `winc`, `binc`, `movestogo` fields

## Verification Criteria

1. Unit tests pass for all six clock scenarios:
   - Sudden death with no increment (e.g., 5 minutes remaining, no `movestogo`, no increment)
   - Sudden death with increment (e.g., 5 minutes remaining + 3 second increment, no `movestogo`)
   - `movestogo` provided with increment (e.g., 2 minutes remaining, 20 moves to go, 5 second increment)
   - `movestogo` provided without increment (e.g., 2 minutes remaining, 20 moves to go, zero increment)
   - Very low time remaining (under 1 second, with and without increment)
   - Zero increment with no `movestogo` (pure sudden death)
2. Function returns correct `Duration` values matching the formula `time_left / moves_to_go + increment`
3. `cargo test` passes
4. `cargo clippy -- -D warnings` passes
5. `cargo fmt --check` passes