# Phase P03 — Move Ordering (Basic)

## Goal

Add three complementary move ordering techniques — MVV-LVA for captures, killer move heuristic for quiet moves, and PV move ordering from iterative deepening — to the existing search framework, so that alpha-beta pruning achieves significantly more cutoffs and the engine explores fewer nodes to reach the same depth.

## Design Decisions

1. **Move scoring approach**: Assign each move a numeric score, then sort the move list before searching. The priority order is: PV move (highest), captures scored by MVV-LVA, killer moves, remaining quiet moves (lowest). This avoids changing the move generation interface — ordering happens between generation and search iteration.

2. **MVV-LVA scoring formula**: Score captures as `victim_value * 100 - attacker_value`. This ensures that capturing a high-value piece with a low-value attacker always scores higher than capturing a low-value piece with any attacker, while breaking ties in favor of the least valuable attacker. Uses piece values from `chess_eval::material::piece_value`.

3. **Killer move storage**: Two killer slots per ply, stored in a fixed-size array indexed by ply. When a quiet move causes a beta cutoff, it replaces the second slot and the former first slot shifts down. Killers are validated as pseudo-legal before being given priority (since they originate from sibling nodes).

4. **PV table structure**: A triangular PV table (array of arrays) that stores the principal variation at each ply during search. After each iterative deepening iteration, the root PV is extracted. At each node, if the current position's PV move matches an available move, it gets highest ordering priority.

5. **Integration point**: Move ordering lives in the `search` crate alongside the existing negamax/quiescence functions. A `SearchContext` (or extension of the existing one) holds the killer table and PV table, cleared at the start of each new search.

## Tasks

- **T01 — MVV-LVA capture ordering**: Implement a move scoring function that assigns MVV-LVA scores to capture moves using piece values from the `eval` crate. Integrate into the search so that captures are sorted by this score before being searched. Quiescence search also benefits from this ordering.

- **T02 — Killer move heuristic**: Add a killer move table (2 slots per ply) to `SearchContext`. Update killers when a quiet move causes a beta cutoff during negamax. During move ordering, give killer moves priority below captures but above other quiet moves. Validate killers as pseudo-legal in the current position before promoting them.

- **T03 — PV move ordering**: Implement a triangular PV table that collects the principal variation during search. After each iterative deepening iteration, the PV is available for the next iteration. During move ordering, the PV move for the current ply gets highest priority (above captures and killers). Clear the PV table at the start of each new search.

## Exit Criteria

1. Captures are ordered by MVV-LVA scoring — capturing a higher-value piece with a lower-value attacker is searched before capturing a lower-value piece with a higher-value attacker.
2. Killer moves that caused beta cutoffs are stored per ply and used to prioritize quiet moves in sibling nodes, validated as pseudo-legal before use.
3. The principal variation from the previous iterative deepening iteration is used to order the PV move first at each ply.
4. Move ordering produces a measurable node-count reduction compared to unordered search at the same depth.
5. All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.

## Dependencies

- **M01 (Core Types & Board Representation)**: `types` crate provides `Move`, `MoveFlag`, `PieceKind`, `Square`, `Color`. `board` crate provides `Position` with `make_move`/`unmake_move`.
- **M02 (Move Generation)**: `movegen` crate provides `generate_legal_moves`.
- **M03/P01 (Material Evaluation & Piece-Square Tables)**: `eval` crate provides `piece_value(kind)` for MVV-LVA scoring.
- **M03/P02 (Negamax with Alpha-Beta)**: `search` crate provides the negamax, quiescence, and iterative deepening framework that move ordering integrates into, including `SearchContext`.

## Artifacts

- `crates/search/src/ordering.rs` — move scoring and sorting logic (MVV-LVA, killer integration, PV move priority)
- `crates/search/src/killer.rs` — killer move table implementation
- `crates/search/src/pv_table.rs` — triangular PV table for collecting and retrieving the principal variation
- `crates/search/src/lib.rs` — modified to integrate move ordering into negamax, quiescence, and iterative deepening; extended `SearchContext` with killer table and PV table