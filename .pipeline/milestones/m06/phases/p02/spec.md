# Phase P02 — Move Ordering Improvements

## Goal

Enhance quiet move ordering and capture evaluation to maximize alpha-beta cutoff rates. These improvements build on the existing move ordering pipeline (MVV-LVA, killer moves, TT move, PV move) by adding the history heuristic, counter-move heuristic, and static exchange evaluation (SEE), all integrated into the `search` crate.

## Deliverables

- **T01 — History Heuristic**: Track which quiet moves cause beta cutoffs by accumulating scores indexed by (piece, to-square) or (side, from, to). Use accumulated history scores to order quiet moves that are not TT moves, killers, or counter-moves. Must achieve ≥5% reduction in nodes searched at depth 10 on the standard benchmark position set compared to baseline with only MVV-LVA, killer, TT, and PV move ordering.
- **T02 — Counter-Move Heuristic**: For each (piece, to-square) of the previous move, store the move that refuted it (caused a beta cutoff). Use the counter-move as a move ordering signal, typically ordered after killers but before plain history-ordered quiet moves.
- **T03 — Static Exchange Evaluation (SEE)**: Evaluate a capture sequence on a single square to determine if a capture is winning, losing, or equal. Use SEE to prune bad captures (losing exchanges) from the search and to order good captures. Must be validated by unit tests covering equal, winning, and losing exchanges.

## Technical Decisions & Constraints

- History heuristic tables should be indexed by [side][from_square][to_square] or [piece][to_square] — the choice should balance memory usage and hit rate.
- History scores should use an aging/scaling mechanism to prevent overflow and to favor recent information over stale data.
- Counter-move table is indexed by [piece][to_square] of the previous move, storing a single move per entry.
- SEE operates on the board state using attack/defend information from bitboards. It simulates the sequence of captures on a square using a swap algorithm with the least valuable attacker first.
- SEE is used in two contexts: (1) capture ordering within the move list, and (2) pruning of bad captures (negative SEE) to skip losing exchanges entirely.
- The combined history + counter-move + SEE ordering targets the ≥5% node reduction criterion specified in the milestone exit criteria.

## Dependencies

- Depends on M06/P01 (Pruning Techniques) — pruning should be in place so that node count improvements from move ordering are measured on top of the pruned search tree.
- Depends on M05 (Transposition Table & Hash) — TT move ordering must be functional.
- Depends on M03 (Basic Search & Evaluation) — existing move ordering infrastructure (MVV-LVA, killer moves, PV move) must be in place.
- Depends on M01–M02 (Types, Board, Movegen) — bitboard attack generation needed for SEE implementation.