# T01 — TT-Based Move Ordering

## Objective

Verify and harden the TT move ordering path established in M05/P01. Add a legality check so that an invalid TT move from a hash collision is discarded rather than ordered first, and add tests confirming correctness and performance of TT move ordering.

## Deliverables

- **Legality validation for TT moves** before passing to `order_moves` in `crates/search/src/lib.rs`: confirm the TT move appears in the legal move list for the current position before using it for ordering.
- **Tests** verifying:
  - (a) The TT move is always ordered first across varied positions (captures, quiet moves, promotions)
  - (b) An invalid TT move from a hash collision is discarded rather than ordered first
  - (c) TT move ordering produces fewer nodes searched than ordering without the TT move on a set of test positions

## Files to Modify

- `crates/search/src/lib.rs` — Add legality validation for TT moves before passing to `order_moves`

## Key Implementation Details

- Phase P01 already passes the TT move through `order_moves` with the highest-priority score (`TT_SCORE = 200_000`). This task refines that path.
- Before passing the TT move to `order_moves`, check that it appears in the legal move list for the current position (a TT collision could produce an invalid move from a different position).
- `crates/search/src/ordering.rs` should remain unchanged — the TT move ordering mechanism from P01 is already functional.

## Dependencies

- **M05/P01** (Transposition Table): TT data structure, probe/store integration, TT move ordering infrastructure with `TT_SCORE = 200_000`
- **M02** (Move Generation): Legal move generation for validating TT moves
- **M03** (Basic Search): Negamax with alpha-beta and move ordering framework in `crates/search/src/lib.rs`

## Verification Criteria

1. TT move is placed first in move ordering when a TT hit provides a best move
2. Invalid TT moves from hash collisions are discarded before ordering
3. All existing tests in `crates/search/` continue to pass
4. `cargo test` passes across the full workspace