## Phase P01 — Polyglot Opening Book

### Goal

Implement a complete Polyglot `.bin` opening book integration so the engine can look up and play known book lines before falling back to search. This involves parsing the binary book format, hashing positions with Polyglot-specific Zobrist keys, selecting moves via configurable strategies, and wiring book consultation into the UCI engine loop.

### Deliverables

- **T01 — Polyglot `.bin` book format parser**: Read Polyglot book entries from the binary `.bin` format. Implement Polyglot Zobrist key hashing (which differs from the engine's internal Zobrist scheme) to map positions to book entries. Provide a lookup interface that, given a position, returns all matching book moves with their weights.
- **T02 — Book move selection**: Implement two selection modes: `bestmove` (always pick the move with the highest weight) and `weighted` (random selection proportional to entry frequency/weight). The selection interface should be configurable at runtime.
- **T03 — UCI integration**: Add a UCI option for the book file path and selection mode. Integrate book consultation into the engine main loop so the engine checks the book before initiating search. If a book move is found, return it immediately as `bestmove` without searching.

### Technical Decisions & Constraints

- Polyglot Zobrist keys are a specific set of random values defined by the Polyglot specification — they are NOT the same as the engine's internal Zobrist keys. A separate hashing function is required.
- The `.bin` format stores entries as 16-byte records (8 bytes key, 2 bytes move, 2 bytes weight, 4 bytes learn) sorted by key for binary search.
- Move encoding in Polyglot format differs from the engine's internal `Move` encoding and must be translated.
- External Polyglot `.bin` book file(s) are required for testing.
- The engine must have a functioning UCI protocol (M04 complete) for option support and engine loop integration.
- Zobrist hashing infrastructure (M05 complete) is required for position matching.

### Dependencies

- **M08** (Testing, Tuning & Strength Validation) must be complete — tuned evaluation and search.
- **M04** (UCI Protocol) must be complete — UCI option support and engine main loop.
- **M05** (Transposition Table & Hash) must be complete — Zobrist hashing infrastructure.
- No dependency on Phase P02 (Syzygy); this phase is independent.