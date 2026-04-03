## Phase P03 — Move Ordering (Basic)

### Goal

Improve search efficiency by ordering moves so that the best moves are searched first, causing more alpha-beta cutoffs and reducing the number of nodes explored. This phase adds three complementary move ordering techniques: MVV-LVA for captures, killer moves for quiet moves that caused cutoffs, and PV move ordering from iterative deepening.

### Deliverables

- **T01 — MVV-LVA capture ordering**: Implement Most Valuable Victim, Least Valuable Attacker ordering for capture moves. Score each capture by `victim_value - attacker_value / large_constant` (or equivalent) so that capturing a queen with a pawn is ordered before capturing a pawn with a queen. Sort captures by this score before searching.
- **T02 — Killer move heuristic**: Store 2 killer moves per ply — quiet (non-capture) moves that caused a beta cutoff. When ordering moves, give killer moves a high priority (below captures but above other quiet moves). Update killers when a quiet move causes a beta cutoff.
- **T03 — PV move ordering**: Store the principal variation (sequence of best moves) from the previous iterative deepening iteration. At each node, if the PV move for that ply is available, search it first (highest priority, before captures and killers).

### Technical Constraints

- Move ordering integrates into the existing search from P02 — the search calls a move ordering/scoring function before iterating over moves.
- PV move should have highest priority, then captures sorted by MVV-LVA, then killer moves, then remaining quiet moves.
- Killer moves must be validated as pseudo-legal before being tried (they come from sibling nodes and may not be legal in the current position).
- The killer table and PV table should be cleared or aged appropriately between searches (e.g., on `ucinewgame` or at the start of a new search).
- Measurable improvement: move ordering should produce a measurable node-count reduction compared to unordered search at the same depth.
- No `unwrap()` in library code.

### Dependencies

- **M01**: `types` and `board` crates.
- **M02**: `movegen` crate.
- **P01 (Material Evaluation & Piece-Square Tables)**: Piece values needed for MVV-LVA scoring.
- **P02 (Negamax with Alpha-Beta)**: The search framework that move ordering plugs into. Iterative deepening provides the PV from the previous iteration.