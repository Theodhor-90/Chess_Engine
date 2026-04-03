# Milestone M09 — Opening Book & Endgame Tablebases

## Goal

Integrate external chess knowledge into the engine by implementing Polyglot opening book support and Syzygy endgame tablebase probing, enabling the engine to play known book lines in the opening and achieve perfect play in low-piece-count endgames, targeting approximately 2300 Elo.

## Scope

- Polyglot `.bin` opening book format parser with position hashing and move lookup
- Book move selection with `bestmove` and `weighted` random modes
- UCI option for book path and mode, with book consultation integrated into the engine loop before search
- Syzygy tablebase probing for WDL and DTZ values via `Fathom` (C library) or a Rust port
- Root search tablebase integration to filter losing moves at low piece counts
- In-search tablebase WDL probes returning exact scores at supported piece counts
- UCI options for Syzygy tablebase path and probe depth with lazy loading of tablebase files

## Phases

### Phase P01 — Polyglot Opening Book

- **T01**: Polyglot `.bin` book format parser — read entries, hash positions with Polyglot Zobrist keys, look up book moves.
- **T02**: Book move selection — weighted random selection by frequency/weight. Support for `bestmove` (always pick highest weight) and `weighted` modes.
- **T03**: UCI option for book path and mode. Integrate into engine loop — consult book before searching.

### Phase P02 — Syzygy Endgame Tablebases

- **T01**: Syzygy tablebase probing — integrate `Fathom` (C library) or a Rust port for WDL (win/draw/loss) and DTZ (distance to zeroing) probes.
- **T02**: Search integration — at root, use tablebase WDL to filter losing moves. During search, use WDL probes at low piece counts to return exact scores.
- **T03**: UCI options for Syzygy path and probe depth. Lazy loading of tablebase files.

## Exit Criteria

1. Polyglot book parser correctly reads `.bin` format and looks up moves by position hash using Polyglot Zobrist keys.
2. Book move selection works in both `bestmove` (highest weight) and `weighted` (random by frequency) modes.
3. UCI option controls book path and selection mode; the engine consults the book before initiating search.
4. Syzygy tablebase probing returns correct WDL and DTZ values for known endgame positions.
5. Root search uses tablebase WDL to filter losing moves at low piece counts.
6. In-search tablebase probes return exact scores at supported piece counts.
7. UCI options control Syzygy tablebase path and probe depth; tablebase files are lazily loaded.
8. Engine plays at approximately 2300 Elo level.

## Dependencies

- **M08 (Testing, Tuning & Strength Validation)** must be complete — the engine must have a tuned evaluation, functioning search, and the testing infrastructure required to validate Elo improvements.
- **M04 (UCI Protocol)** must be complete — UCI option support and the engine main loop are required for book/tablebase integration.
- **M05 (Transposition Table & Hash)** must be complete — Zobrist hashing is required for Polyglot book position matching and for search integration of tablebase probes.
- External: Polyglot `.bin` opening book file(s) for testing.
- External: Syzygy tablebase files (3-5 piece) for testing.
- External: `Fathom` C library or equivalent Rust port for Syzygy probing.