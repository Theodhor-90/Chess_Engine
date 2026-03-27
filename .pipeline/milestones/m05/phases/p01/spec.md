## Phase P01 — Transposition Table

### Goal

Design and implement a transposition table (TT) that caches search results keyed by Zobrist hash, and integrate it into the existing alpha-beta search framework. The TT enables the engine to avoid redundant work by recognizing positions reached via different move orders, providing both score cutoffs and a best-move signal for move ordering. This phase delivers the core TT data structure, the hash table container with its replacement policy, and the full search integration including mate-score adjustment.

### Deliverables

- **T01 — TT entry structure**: Define the entry struct containing Zobrist key (or verification bits), search depth, score, bound type (exact/lower/upper), best move, and an age field used for replacement decisions.
- **T02 — TT implementation**: A fixed-size hash table using power-of-two sizing for fast index computation via bitmask. Implement a replacement policy that prefers deeper entries and uses age-based eviction to clear stale entries from prior searches.
- **T03 — TT integration into search**: Probe the TT before searching a node — use the stored score for cutoffs when the entry depth is sufficient. Store results after searching. Use the TT best move as the highest-priority move for ordering. Handle mate score adjustment when storing and retrieving scores at different plies from root.

### Technical Decisions & Constraints

- Table must use power-of-two sizing so index computation is a fast bitmask (`hash & (size - 1)`), not a modulo.
- Replacement policy must prefer deeper entries and evict stale entries from prior searches using the age field.
- Mate scores must be adjusted relative to the root ply when stored and un-adjusted when retrieved, so that mate-in-N information remains correct regardless of which node stored the entry.
- The TT best move must be placed first in the move list when a probe returns a hit.
- The TT lives in `crates/search/` and builds on the Zobrist hashing infrastructure already implemented in `crates/board/src/zobrist.rs`.
- No external crate dependencies for the TT — standard library only for the core data structure.

### Dependencies

- **M01** (Core Types & Board Representation): `Bitboard`, `Square`, `Move`, `Piece`, `Position`, Zobrist hashing (`crates/board/src/zobrist.rs`)
- **M02** (Move Generation): Legal move generation used by search
- **M03** (Basic Search & Evaluation): Negamax with alpha-beta, quiescence search, iterative deepening, and move ordering framework (`crates/search/src/lib.rs`)
- **M04** (UCI Protocol): UCI command loop and time management — the TT integrates into the existing search called by the UCI engine loop
- No dependency on P02 (this phase comes first).