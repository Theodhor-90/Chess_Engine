# Phase P01 — Pruning Techniques

## Goal

Implement three pruning methods that reduce the effective branching factor of the search tree by skipping moves or subtrees unlikely to affect the result. These techniques integrate into the existing negamax/alpha-beta loop in the `search` crate and interact with the transposition table, killer table, and move ordering pipeline established in M03 and M05.

## Deliverables

- **T01 — Null Move Pruning**: Skip a move (give the opponent two moves in a row); if the resulting score still exceeds beta, prune the subtree. Null move pruning must be disabled when the side to move has zero non-pawn material (king and pawns only), as these positions are prone to zugzwang.
- **T02 — Late Move Reductions (LMR)**: Reduce search depth for moves ordered late in the move list (likely bad moves). If the reduced-depth search returns a score above alpha, re-search at full depth. Must achieve ≥30% node count reduction at depth 10 on the standard benchmark position set (startpos, Kiwipete, perft suite positions) compared to baseline without LMR, with no net Elo loss in self-play.
- **T03 — Futility Pruning**: Near leaf nodes, if static eval + margin is below alpha, skip quiet moves. Also implement reverse futility pruning at pre-frontier nodes. Must not reduce the solve rate on the WAC (Win At Chess) tactical test suite at fixed depth compared to baseline without futility pruning.

## Technical Decisions & Constraints

- All pruning techniques plug into the existing alpha-beta search loop in `crates/search/`.
- Null move pruning requires a mechanism to pass the turn (null move) and must check for zugzwang conditions by inspecting non-pawn material for the side to move.
- LMR requires knowledge of move ordering position (move index in the move list) to determine which moves to reduce.
- Futility pruning requires access to a static evaluation score at each node, which is already available from the `eval` crate.
- Node count benchmarking is used as a proxy for strength validation within this milestone; formal Elo testing is deferred to M08.

## Dependencies

- Depends on M05 (Transposition Table & Hash) — TT probing and TT move ordering must be in place.
- Depends on M03 (Basic Search & Evaluation) — negamax with alpha-beta, quiescence search, iterative deepening, MVV-LVA ordering, killer moves, and PV move ordering must be functional.
- Depends on M01–M02 (Types, Board, Movegen) — legal move generation, make/unmake, and is_square_attacked must be correct and performant.
- No dependency on other M06 phases (P02, P03).