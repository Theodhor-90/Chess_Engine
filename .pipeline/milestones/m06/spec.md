## Milestone M06 — Advanced Search Techniques

### Goal

Push search depth and efficiency significantly. Each technique prunes large portions of the search tree. Target: ~1800 Elo.

### Phases

#### Phase P01 — Pruning Techniques
- **T01**: Null move pruning — skip a move (give opponent two moves in a row); if the resulting score still exceeds beta, prune. Disable in endgame/zugzwang-prone positions.
- **T02**: Late move reductions (LMR) — reduce search depth for moves ordered late in the move list (likely bad moves). Re-search at full depth if reduced search returns above alpha.
- **T03**: Futility pruning — near leaf nodes, if static eval + margin is below alpha, skip quiet moves. Reverse futility pruning at pre-frontier nodes.

#### Phase P02 — Move Ordering Improvements
- **T01**: History heuristic — track which quiet moves cause beta cutoffs; use accumulated scores to order quiet moves.
- **T02**: Counter-move heuristic — for each (piece, to-square) of the previous move, store the move that refuted it. Use as a move ordering signal.
- **T03**: Static exchange evaluation (SEE) — evaluate a capture sequence to determine if a capture is winning, losing, or equal. Use to prune bad captures and order good ones.

#### Phase P03 — Search Extensions
- **T01**: Check extension — extend search by one ply when the side to move is in check (avoid missing forced tactics).
- **T02**: Singular extension — if the TT move is significantly better than all alternatives (verified by a reduced-depth search excluding it), extend it by one ply.
- **T03**: Principal Variation Search (PVS) — search first move with full window, remaining moves with zero-window. Re-search with full window on fail-high. Replaces plain alpha-beta.

### Exit Criteria
- Null move pruning correctly prunes futile branches and is disabled in zugzwang-prone positions
- LMR reduces node counts significantly with minimal Elo loss (net positive)
- Futility pruning safely skips hopeless quiet moves near leaf nodes
- History and counter-move heuristics measurably improve move ordering for quiet moves
- SEE correctly evaluates capture sequences and is used for both pruning and ordering
- Check and singular extensions improve tactical accuracy
- PVS replaces plain alpha-beta and produces equivalent or better results
- Engine plays at ~1800 Elo level
- All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`