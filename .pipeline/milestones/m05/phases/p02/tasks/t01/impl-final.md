All verification commands passed. Implementation is complete.

- `crates/search/src/lib.rs` — Modified:
  - Added TT move legality validation in `negamax` (lines 175-180): after generating legal moves, checks if TT move is in the list; discards it if not (guards against hash collisions)
  - Added test `tt_move_ordering_across_positions`: verifies TT move is ordered first across 3 positions (capture, quiet, promotion)
  - Added test `invalid_tt_move_discarded`: fabricates invalid TT move (a1→h8 quiet), stores in TT, confirms negamax discards it and returns a valid move
  - Added test `tt_move_ordering_reduces_nodes`: compares node counts with 1MB TT vs 0MB TT at depth 5, asserts TT-enabled search uses fewer nodes