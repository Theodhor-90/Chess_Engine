## Phase P02 — Negamax with Alpha-Beta

### Goal

Implement the core search algorithm for the chess engine: negamax with alpha-beta pruning, quiescence search to resolve tactical positions at leaf nodes, and iterative deepening with basic time management. This phase wires the evaluation function from Phase P01 into a tree search that can select the best move in any position.

### Deliverables

- **T01 — Negamax with alpha-beta pruning**: Implement the negamax search framework with alpha-beta pruning. The search takes a position, depth, alpha, and beta, and returns `(score, best_move)`. At depth 0, call the evaluation function from P01. Generate legal moves via `movegen`, recursively search each, and prune via alpha-beta bounds.
- **T02 — Quiescence search**: Extend the search beyond the main depth limit by searching captures and promotions to avoid the horizon effect. Implement stand-pat evaluation (use static eval as a lower bound). Only search tactical moves (captures, promotions) in quiescence nodes.
- **T03 — Iterative deepening with basic time management**: Search at depth 1, then depth 2, then depth 3, etc., collecting the best move found at each depth. Implement basic time management with a fixed time per move. Stop deepening when time is exhausted and return the best move from the last completed depth.

### Technical Constraints

- The `search` crate depends on `types`, `board`, `movegen`, and `eval` crates.
- Negamax returns scores from the perspective of the side to move (standard negamax convention).
- Alpha-beta window: initial call uses `(-INFINITY, +INFINITY)` or equivalent sentinel values.
- Quiescence search must not have unbounded depth — use a maximum quiescence depth or rely on the finite number of captures.
- Iterative deepening must be interruptible: check elapsed time periodically (e.g., every N nodes) and abort cleanly.
- Detect checkmate (no legal moves and in check → return `-MATE_SCORE + ply`) and stalemate (no legal moves and not in check → return `0`).
- No `unwrap()` in library code.

### Dependencies

- **M01**: `types` and `board` crates for position representation, make/unmake move.
- **M02**: `movegen` crate for `generate_legal_moves`.
- **P01 (Material Evaluation & Piece-Square Tables)**: The evaluation function used at leaf nodes and for stand-pat in quiescence search.