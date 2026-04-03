## Phase P02 — Search Enhancements (TT-Dependent)

### Goal

Build on the transposition table from Phase P01 to add three TT-dependent search enhancements: refined TT-based move ordering, internal iterative deepening (IID) for nodes without a TT move, and repetition detection via position history tracking. Together these enhancements improve search efficiency and correctness, contributing to the ~1500 Elo target for Milestone M05.

### Deliverables

- **T01 — TT-based move ordering**: When a TT probe returns a hit with a best move, place that move first in the move list to maximize alpha-beta cutoff efficiency. This refines the basic TT move ordering from P01/T03 into a clean, dedicated move-ordering integration point.
- **T02 — Internal iterative deepening (IID)**: At nodes with high remaining depth where no TT move is available, perform a reduced-depth search first to obtain a move for ordering, then search with the full depth using that move first. This reduces overall node counts at deep search levels.
- **T03 — Repetition detection**: Maintain a position history stack of Zobrist hashes across the game and during search. Detect threefold repetition and fifty-move rule draws. Return a draw score when repetition is detected during search.

### Technical Decisions & Constraints

- TT move ordering (T01) must ensure the TT best move is always searched first among all generated moves, ahead of MVV-LVA captures, killers, and history moves.
- IID (T02) should only trigger at nodes above a configurable depth threshold where no TT move exists. The reduced-depth search uses the same alpha-beta framework.
- Repetition detection (T03) requires a position history stack that persists across `make_move`/`unmake_move` during search and is initialized from the game's move history (provided via UCI `position` command). Threefold repetition should count occurrences across the full game history plus the search tree.
- The fifty-move rule check uses the halfmove clock already tracked in `Position`.
- Draw score should typically be 0 (or a small contempt value) to avoid repetition in winning positions.
- All enhancements integrate into the existing search in `crates/search/src/lib.rs`.

### Dependencies

- **Phase P01** (Transposition Table): The TT data structure and search integration are prerequisite — T01 refines TT move ordering, T02 relies on TT probes to determine if a TT move exists, and T03 complements TT-based search scoring.
- **M01–M04**: Same foundational dependencies as P01 (core types, move generation, search framework, UCI protocol).