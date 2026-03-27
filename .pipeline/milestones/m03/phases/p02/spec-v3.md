Good, I have all the context. The only feedback issue is that line 1 contains agent preamble text that must be removed. The rest of the spec-v2 content is fine. Let me output the clean specification.

# Phase P02 — Negamax with Alpha-Beta

## Goal

Implement the core search algorithm for the chess engine — negamax with alpha-beta pruning, quiescence search to resolve tactical positions at leaf nodes, and iterative deepening with basic time management — wiring the `eval` crate's `evaluate` function into a tree search that can select the best move in any position.

## Design Decisions

1. **New `search` crate**: Create `crates/search/` as a library crate depending on `types`, `board`, `movegen`, and `eval`. This follows the project's workspace structure and keeps search logic isolated.

2. **Negamax convention**: Scores are returned from the perspective of the side to move. The evaluation function already accounts for side-to-move, returning positive for advantage. Negation at each recursive level implements the minimax property.

3. **Mate and stalemate scoring**: Use a `MATE_SCORE` constant of 30000, adjusted by ply distance (`-MATE_SCORE + ply` for being mated, `MATE_SCORE - ply` for delivering mate) so the search prefers shorter mates. Stalemate returns 0 (draw).

4. **Quiescence search bound**: Quiescence search has no explicit depth limit but terminates naturally because the number of sequential captures is finite. Stand-pat evaluation provides the lower bound (the side to move can choose not to capture).

5. **Time management via `Instant`**: Use `std::time::Instant` to track elapsed time. The search checks elapsed time every 1024 nodes and aborts cleanly if the time budget is exceeded, returning the best move from the last fully completed iteration.

6. **Score type**: Use `i32` for scores, consistent with the `eval` crate's return type.

## Tasks

### T01 — Negamax Search with Alpha-Beta Pruning

Create the `search` crate with the core negamax search function. The function takes a `Position`, depth, alpha, beta, and returns `(i32, Option<Move>)`. At depth 0, call `eval::evaluate`. Generate legal moves via `movegen::generate_legal_moves`, detect checkmate (no legal moves and king in check) returning `-MATE_SCORE + ply`, detect stalemate (no legal moves and king not in check) returning `0`. Recursively search each move with alpha-beta pruning.

**Deliverables**: `crates/search/` crate with `Cargo.toml`, `src/lib.rs`, negamax function, score constants (`MATE_SCORE`, `INFINITY`), and unit tests verifying correct mate/stalemate detection and basic search behavior.

### T02 — Quiescence Search

Implement quiescence search called at depth 0 instead of a raw static evaluation. The quiescence search uses stand-pat evaluation as a lower bound, generates only capture and promotion moves, and recursively searches them with alpha-beta pruning. Integrate into the negamax function so that leaf nodes resolve tactical exchanges.

**Deliverables**: Quiescence search function, capture/promotion move filtering logic, integration into negamax at depth 0, and tests verifying the search does not miss hanging pieces or simple captures.

### T03 — Iterative Deepening with Basic Time Management

Implement iterative deepening that searches at depth 1, then depth 2, incrementing by 1 each iteration, collecting the best move at each completed depth. Add basic time management: accept a fixed time budget, check elapsed time every 1024 nodes during search, and abort cleanly when time is exhausted. Return the best move from the last fully completed depth. Expose a top-level search entry point that takes a `Position` and time budget.

**Deliverables**: Iterative deepening loop, time-checking mechanism, top-level search entry point, and tests verifying that deeper searches produce equal or better moves than shallower ones and that the search respects time limits.

## Exit Criteria

1. Alpha-beta search with quiescence search finds tactical solutions correctly — does not miss hanging pieces or simple captures within search depth.
2. Checkmate and stalemate are detected and scored correctly (mate score adjusted by ply, stalemate returns draw score).
3. Iterative deepening produces progressively better moves at increasing depths.
4. Search respects the time budget and returns a legal move from the last completed iteration.

## Dependencies

- **M01 (Core Types & Board Representation)**: `types` crate provides `Move`, `Square`, `Piece`, `PieceKind`, `Color`, `Bitboard`. `board` crate provides `Position` with `make_move`/`unmake_move`, `UndoInfo`, `is_square_attacked`, `side_to_move`, FEN parsing.
- **M02 (Move Generation)**: `movegen` crate provides `generate_legal_moves(pos: &mut Position) -> Vec<Move>`.
- **M03/P01 (Material Evaluation & Piece-Square Tables)**: `eval` crate provides `evaluate(pos: &Position) -> i32` with material counting and tapered piece-square table evaluation.

## Artifacts

- `crates/search/Cargo.toml` — new crate manifest with dependencies on `types`, `board`, `movegen`, `eval`
- `crates/search/src/lib.rs` — negamax search, quiescence search, iterative deepening, time management, score constants
- `Cargo.toml` (workspace root) — updated to include `crates/search` as a workspace member