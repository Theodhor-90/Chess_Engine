Now I have all the context I need. Let me produce the specification.

## Milestone M03 — Basic Search & Evaluation

### Goal

Deliver a functional chess engine that evaluates positions using material counting and piece-square tables with tapered evaluation, searches game trees via negamax with alpha-beta pruning and quiescence search, uses iterative deepening with basic time management, and orders moves using MVV-LVA, killer moves, and principal variation — producing legal, reasonable moves at approximately 1200 Elo strength.

### Scope

This milestone creates the `eval` and `search` crates. The evaluation function covers material values, piece-square tables (separate middlegame/endgame), and tapered evaluation based on game phase. The search covers negamax with alpha-beta pruning, quiescence search (captures and promotions with stand-pat), iterative deepening with fixed time-per-move time management, and basic move ordering (MVV-LVA, killer moves, PV move ordering). No UCI integration, no transposition table, no advanced pruning or search extensions.

### Phases

#### Phase P01 — Material Evaluation & Piece-Square Tables

- **T01**: Material counting evaluation — sum piece values for each side using standard values (P=100, N=320, B=330, R=500, Q=900). Return score relative to side to move.
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

1. Evaluation function returns sensible scores based on material and piece-square tables with tapered evaluation interpolating between middlegame and endgame.
2. Alpha-beta search with quiescence search finds tactical solutions correctly (does not miss hanging pieces or simple tactics within search depth).
3. Iterative deepening produces progressively better moves at increasing depths.
4. Move ordering (MVV-LVA, killers, PV) produces measurable node count reduction vs unordered search.
5. Engine plays legal, reasonable moves in any position (~1200 Elo level).

### Dependencies

- **M01 (Core Types & Board Representation)**: `types` crate provides `Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`, `Move`, `File`, `Rank`. `board` crate provides `Position` with `make_move`/`unmake_move`, FEN parsing, Zobrist hashing, and `is_square_attacked`.
- **M02 (Move Generation)**: `movegen` crate provides legal move generation for all piece types, including special moves (castling, en passant, promotions), validated by perft.