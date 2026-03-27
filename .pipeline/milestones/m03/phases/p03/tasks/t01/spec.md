# T01 — MVV-LVA Capture Ordering

## Objective

Implement a move scoring function that assigns MVV-LVA (Most Valuable Victim, Least Valuable Attacker) scores to capture moves, and integrate it into the search so that captures are sorted by this score before being searched. Quiescence search also benefits from this ordering.

## Deliverables

- A move scoring function that computes MVV-LVA scores for capture moves
- Integration into negamax search and quiescence search so captures are sorted before iteration

## Key Implementation Details

- **Scoring formula**: `victim_value * 100 - attacker_value`. This ensures capturing a high-value piece with a low-value attacker always scores higher than capturing a low-value piece with any attacker, while breaking ties in favor of the least valuable attacker.
- **Piece values**: Use `chess_eval::material::piece_value` (P=100, N=320, B=330, R=500, Q=900) from the `eval` crate.
- **Move scoring approach**: Assign each move a numeric score, then sort the move list before searching. Ordering happens between generation and search iteration — the move generation interface does not change.
- **File to create**: `crates/search/src/ordering.rs` — move scoring and sorting logic.
- **File to modify**: `crates/search/src/lib.rs` — integrate move ordering into negamax and quiescence search functions.

## Dependencies

- **M01 types crate**: `Move`, `MoveFlag`, `PieceKind`, `Square`, `Color`
- **M01 board crate**: `Position` with `make_move`/`unmake_move`
- **M02 movegen crate**: `generate_legal_moves`
- **M03/P01 eval crate**: `piece_value(kind)` for MVV-LVA scoring
- **M03/P02 search crate**: Negamax, quiescence, and iterative deepening framework including `SearchContext`
- No dependencies on other tasks within this phase (T01 is the first task).

## Verification Criteria

1. Captures are ordered by MVV-LVA scoring — capturing a higher-value piece with a lower-value attacker is searched before capturing a lower-value piece with a higher-value attacker.
2. Both negamax and quiescence search use MVV-LVA ordering for captures.
3. All existing tests continue to pass (`cargo test`).
4. Code passes `cargo fmt --check` and `cargo clippy -- -D warnings`.