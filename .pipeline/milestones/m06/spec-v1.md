Now I have a thorough understanding of both the master plan requirements for M06 and the current codebase state. Let me produce the specification.

# Milestone M06 — Advanced Search Techniques

## Goal

Push search depth and efficiency significantly by implementing pruning techniques (null move pruning, late move reductions, futility pruning), move ordering improvements (history heuristic, counter-move heuristic, static exchange evaluation), and search extensions (check extension, singular extension, Principal Variation Search), targeting approximately 1800 Elo.

## Scope

This milestone adds three categories of search enhancements to the existing alpha-beta framework established in M03 and augmented with the transposition table in M05:

- **Pruning techniques** that skip large portions of the search tree when the position strongly favors one side (null move pruning, LMR, futility pruning)
- **Move ordering improvements** that ensure the best moves are searched first, maximizing cutoff rates (history heuristic, counter-move heuristic, SEE)
- **Search extensions** that deepen the search in critical positions to avoid missing forced tactics (check extension, singular extension, PVS)

All techniques integrate into the existing `search` crate's negamax/alpha-beta loop and interact with the transposition table, killer table, and move ordering pipeline already in place.

## Phases

### Phase P01 — Pruning Techniques

Implement three pruning methods that reduce the effective branching factor by skipping moves or subtrees that are unlikely to affect the result.

- **T01**: Null move pruning — skip a move (give opponent two moves in a row); if the resulting score still exceeds beta, prune. Disable in endgame/zugzwang-prone positions.
- **T02**: Late move reductions (LMR) — reduce search depth for moves ordered late in the move list (likely bad moves). Re-search at full depth if reduced search returns above alpha.
- **T03**: Futility pruning — near leaf nodes, if static eval + margin is below alpha, skip quiet moves. Reverse futility pruning at pre-frontier nodes.

### Phase P02 — Move Ordering Improvements

Enhance quiet move ordering and capture evaluation to maximize alpha-beta cutoff rates.

- **T01**: History heuristic — track which quiet moves cause beta cutoffs; use accumulated scores to order quiet moves.
- **T02**: Counter-move heuristic — for each (piece, to-square) of the previous move, store the move that refuted it. Use as a move ordering signal.
- **T03**: Static exchange evaluation (SEE) — evaluate a capture sequence to determine if a capture is winning, losing, or equal. Use to prune bad captures and order good ones.

### Phase P03 — Search Extensions

Add selective search extensions that deepen the search in tactically critical positions, plus replace plain alpha-beta with PVS.

- **T01**: Check extension — extend search by one ply when the side to move is in check (avoid missing forced tactics).
- **T02**: Singular extension — if the TT move is significantly better than all alternatives (verified by a reduced-depth search excluding it), extend it by one ply.
- **T03**: Principal Variation Search (PVS) — search first move with full window, remaining moves with zero-window. Re-search with full window on fail-high. Replaces plain alpha-beta.

## Exit Criteria

1. Null move pruning correctly prunes futile branches and is disabled in zugzwang-prone positions
2. LMR reduces node counts significantly with minimal Elo loss (net positive)
3. Futility pruning safely skips hopeless quiet moves near leaf nodes
4. History and counter-move heuristics measurably improve move ordering for quiet moves
5. SEE correctly evaluates capture sequences and is used for both pruning and ordering
6. Check and singular extensions improve tactical accuracy
7. PVS replaces plain alpha-beta and produces equivalent or better results
8. Engine plays at approximately 1800 Elo level
9. All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`

## Dependencies

- **M05 (Transposition Table & Hash)** — TT probing and TT move ordering must be in place; singular extension depends on TT entries to identify candidate moves
- **M03 (Basic Search & Evaluation)** — negamax with alpha-beta, quiescence search, iterative deepening, MVV-LVA ordering, killer moves, and PV move ordering must be functional
- **M01–M02 (Types, Board, Movegen)** — legal move generation, make/unmake, and is_square_attacked must be correct and performant