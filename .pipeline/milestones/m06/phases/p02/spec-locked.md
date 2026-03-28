# Phase P02 — Move Ordering Improvements

## Goal

Enhance the search crate's move ordering pipeline by adding three new ordering signals — history heuristic, counter-move heuristic, and static exchange evaluation (SEE) — that increase alpha-beta cutoff rates for quiet moves and improve capture evaluation, building on the existing MVV-LVA, killer, TT move, and PV move infrastructure already in place.

## Design Decisions

1. **History table indexing**: The history heuristic table is indexed by `[side][from_square][to_square]` (2 × 64 × 64 = 8,192 entries). This captures move-specific patterns better than `[piece][to_square]` since it distinguishes which square a piece moves from, and the memory footprint (≈32 KB at 4 bytes per entry) is negligible.

2. **History score aging**: History scores use a gravity formula: on each update, `score = score - score * |bonus| / 16384 + bonus`, where `bonus = depth * depth` for moves causing beta cutoffs and `bonus = -(depth * depth)` for moves that did not cause cutoffs. This bounds scores within approximately `[-16384, 16384]` without requiring explicit clears between iterative deepening iterations, and ensures recent information naturally outweighs stale data.

3. **Counter-move table indexing**: The counter-move table is indexed by `[piece][to_square]` of the *previous* move (6 piece kinds × 64 squares = 384 entries), storing one move per entry. This is the standard approach: simple, low memory, and effective.

4. **SEE algorithm**: SEE uses a swap algorithm operating on bitboard attack/defend sets for a target square. It iterates through attackers from least valuable to most valuable, simulating the capture exchange to produce a final material gain/loss score.

5. **SEE dual usage**: SEE is used in two contexts — (a) ordering captures (replacing or augmenting MVV-LVA for distinguishing good from bad captures) and (b) pruning bad captures (negative SEE score) to skip losing exchanges entirely from the search.

6. **Move ordering priority**: The updated ordering hierarchy integrates the new signals into the existing pipeline: TT move > PV move > good captures (positive SEE) > killer moves > counter-move > history-ordered quiet moves > bad captures (negative SEE).

## Tasks

- **T01 — History Heuristic**: Implement a `HistoryTable` struct in the search crate that accumulates scores for quiet moves causing beta cutoffs, indexed by `[side][from_square][to_square]`. Integrate into `SearchContext` and the move ordering function. Use the gravity formula (`score = score - score * |bonus| / 16384 + bonus`) for updates. Clear on `ucinewgame`. Update `score_move` and `order_moves` to use history scores for quiet move ordering.

- **T02 — Counter-Move Heuristic**: Implement a `CounterMoveTable` struct that stores a single refutation move for each `(piece, to_square)` of the previous move. Integrate into `SearchContext`, update the search loop to pass the previous move into the ordering function, and assign counter-moves a priority between killers and plain history-ordered quiet moves.

- **T03 — Static Exchange Evaluation (SEE)**: Implement a `see` function that evaluates a capture sequence on a target square using bitboard attacker/defender sets and a least-valuable-attacker swap loop. Integrate SEE into move ordering (good captures scored above killers, bad captures scored below quiet moves) and into the search for bad capture pruning. Validate with unit tests covering the following specific scenarios: PxN (winning), NxB (roughly equal), BxR (winning), RxQ (winning), PxN with NxP recapture (equal), undefended piece capture (winning), a multi-piece exchange sequence on a single square (e.g., PxN, BxP, RxB, QxR — net losing for the initiator), and a capture into a defended pawn chain (losing).

## Exit Criteria

1. History and counter-move heuristics achieve ≥5% reduction in nodes searched at depth 10 on the standard benchmark position set (startpos, Kiwipete, perft suite positions) compared to baseline with only MVV-LVA, killer, TT, and PV move ordering
2. SEE correctly evaluates capture sequences, validated by unit tests covering equal exchanges, winning exchanges, and losing exchanges across the specific piece combinations listed in T03
3. SEE is used for both capture ordering and bad capture pruning in the search
4. No regression in solve rate on the WAC tactical test suite at fixed depth compared to the P01 baseline

## Dependencies

- **M06/P01 (Pruning Techniques)** — null move pruning, LMR, and futility pruning must be in place so that node count improvements from move ordering are measured on top of the pruned search tree
- **M05 (Transposition Table & Hash)** — TT move ordering must be functional
- **M03 (Basic Search & Evaluation)** — existing move ordering infrastructure (MVV-LVA, killer moves, PV move), negamax with alpha-beta, quiescence search, and iterative deepening must be in place
- **M01–M02 (Types, Board, Movegen)** — bitboard attack generation and piece/square types needed for SEE implementation and history/counter-move table indexing

## Artifacts

- `crates/search/src/history.rs` — `HistoryTable` struct with update, score lookup, aging, and clear methods
- `crates/search/src/countermove.rs` — `CounterMoveTable` struct with store, lookup, and clear methods
- `crates/search/src/see.rs` — `see` function implementing static exchange evaluation
- `crates/search/src/ordering.rs` — updated `score_move` and `order_moves` functions integrating history, counter-move, and SEE scoring
- `crates/search/src/lib.rs` — updated `SearchContext` (new fields for history and counter-move tables), updated search loop to pass previous move context and update history/counter-move tables on beta cutoffs