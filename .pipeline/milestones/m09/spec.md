## Milestone M09 — Opening Book & Endgame Tablebases

### Goal

External knowledge integration for stronger play in known positions. Target: ~2300 Elo.

### Phases

#### Phase P01 — Polyglot Opening Book
- **T01**: Polyglot `.bin` book format parser — read entries, hash positions with Polyglot Zobrist keys, look up book moves.
- **T02**: Book move selection — weighted random selection by frequency/weight. Support for `bestmove` (always pick highest weight) and `weighted` modes.
- **T03**: UCI option for book path and mode. Integrate into engine loop — consult book before searching.

#### Phase P02 — Syzygy Endgame Tablebases
- **T01**: Syzygy tablebase probing — integrate `Fathom` (C library) or a Rust port for WDL (win/draw/loss) and DTZ (distance to zeroing) probes.
- **T02**: Search integration — at root, use tablebase WDL to filter losing moves. During search, use WDL probes at low piece counts to return exact scores.
- **T03**: UCI options for Syzygy path and probe depth. Lazy loading of tablebase files.

### Exit Criteria
- Polyglot book parser correctly reads `.bin` format and looks up moves by position hash
- Book move selection works in both `bestmove` and `weighted` random modes
- UCI option controls book path and mode; engine consults book before searching
- Syzygy tablebase probing returns correct WDL and DTZ values for known positions
- Root search uses tablebase WDL to filter losing moves at low piece counts
- In-search tablebase probes return exact scores at supported piece counts
- UCI options control Syzygy path and probe depth; files are lazily loaded
- Engine plays at ~2300 Elo level
- All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`