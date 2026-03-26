## Milestone M03 — Basic Search & Evaluation

### Goal

A playable engine: it can search positions, evaluate them, and pick reasonable moves. Not strong yet, but functional end-to-end. Target: ~1200 Elo.

### Phases

#### Phase P01 — Material Evaluation & Piece-Square Tables
- **T01**: Material counting evaluation (standard piece values: P=100, N=320, B=330, R=500, Q=900).
- **T02**: Piece-square tables for all piece types — separate middlegame and endgame tables. Tapered evaluation interpolating between game phases based on remaining material.
- **T03**: Game phase calculation — compute a phase score from remaining material to weight middlegame vs endgame PST values.

#### Phase P02 — Negamax with Alpha-Beta
- **T01**: Negamax search framework with alpha-beta pruning. Returns `(score, best_move)`.
- **T02**: Quiescence search — extend search with captures and promotions beyond the main search depth to avoid horizon effect. Stand-pat evaluation.
- **T03**: Iterative deepening — search depth 1, 2, 3, ... collecting best move at each depth. Basic time management (fixed time per move initially).

#### Phase P03 — Move Ordering (Basic)
- **T01**: MVV-LVA (Most Valuable Victim, Least Valuable Attacker) capture ordering.
- **T02**: Killer move heuristic — store 2 killer moves per ply that caused beta cutoffs.
- **T03**: PV move ordering — store principal variation from previous iteration, search PV move first.

### Exit Criteria
- Evaluation function returns sensible scores based on material and piece-square tables with tapered evaluation
- Alpha-beta search with quiescence search finds tactical solutions correctly
- Iterative deepening produces progressively better moves at increasing depths
- Move ordering (MVV-LVA, killers, PV) produces measurable node count reduction vs unordered search
- Engine plays legal, reasonable moves in any position (~1200 Elo level)
- All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`