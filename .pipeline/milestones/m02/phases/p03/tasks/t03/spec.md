# T03 — Perft Function and Test Suite

## Objective

Implement a `perft` function and validate the entire move generator end-to-end against known reference counts for six standard positions to depth 5+. This is the definitive correctness gate for all of move generation. Also create a standalone perft runner binary.

## Deliverables

- Updated: `crates/movegen/src/lib.rs` — add public `perft` function
- New file: `tests/perft.rs` — integration tests validating perft against six standard reference positions
- New file: `tools/perft.rs` (or `tools/perft/main.rs`) — standalone perft runner binary with CLI interface

## Implementation Details

### Perft Function

`perft(pos: &mut Position, depth: u32) -> u64` recursively counts leaf nodes:
- Generate legal moves via `generate_legal_moves`
- For each move: call `make_move`, recurse at `depth - 1`, then call `unmake_move`
- At depth 0, return 1

### Integration Test Suite

Create `tests/perft.rs` validating against these six standard positions, all to depth 5+:

| Position | FEN | Depth | Expected Nodes |
|----------|-----|-------|----------------|
| Starting position | `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1` | 5 | 4,865,609 |
| Kiwipete | `r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -` | 5 | 193,690,690 |
| CPW Position 3 | `8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - -` | 5 | 674,624 |
| CPW Position 4 | `r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq -` | 5 | 15,833,292 |
| CPW Position 5 | `rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ -` | 5 | 89,941,194 |
| CPW Position 6 | `r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - -` | 5 | 164,075,551 |

Exact match is required — any discrepancy indicates a bug in move generation.

### Standalone Perft Runner Binary

Create a binary in `tools/` that:
- Accepts a FEN string and depth as command-line arguments
- Prints the perft result
- Supports optional `divide` mode that prints per-move subtree counts for debugging

## Dependencies

- **T02 (Legal Move Filter)** — `generate_legal_moves` must be available and correct.
- **M01 complete** — `Position::from_fen`, `make_move`, `unmake_move`.

## Verification Criteria

- All six perft test positions match expected node counts exactly at depth 5+
- The standalone perft runner binary compiles and runs correctly with FEN + depth arguments
- Divide mode works and per-move subtree counts sum to the total perft count
- All tests pass via `cargo test` (including the integration tests in `tests/perft.rs`)
- `cargo clippy -- -D warnings` passes
- `cargo fmt --check` passes