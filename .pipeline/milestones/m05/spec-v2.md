## Milestone M05 — Transposition Table & Hash

### Goal

Implement a transposition table that caches search results keyed by Zobrist hash, enabling the engine to avoid re-searching positions reached via different move orders. Integrate the TT into the existing search framework for move ordering and cutoff decisions, add internal iterative deepening when no TT move is available, and implement repetition detection via position history tracking. Target: ~1500 Elo.

### Scope

- TT entry structure with Zobrist key verification, depth, score, bound type (exact/lower/upper), best move, and age
- Fixed-size hash table with power-of-two sizing and a replacement policy preferring deeper entries with age-based eviction
- TT probe before searching and TT store after searching, with mate score adjustment for entries stored at different plies
- TT-based move ordering — place the TT best move first in the move list
- Internal iterative deepening (IID) — perform a shallower search to find a move to order first when no TT move is available at high depth
- Repetition detection — track position history via Zobrist hashes, detect threefold repetition and fifty-move rule draws, integrate into search scoring

### Phases

#### Phase P01 — Transposition Table

- **T01**: TT entry structure — define the entry containing Zobrist key (or verification bits), search depth, score, bound type (exact/lower/upper), best move, and age field for replacement decisions.
- **T02**: TT implementation — fixed-size hash table using power-of-two sizing for fast index computation via bitmask. Implement a replacement policy that prefers deeper entries and uses age-based eviction to clear stale entries from prior searches.
- **T03**: TT integration into search — probe the TT before searching a node (use stored score for cutoffs when depth is sufficient), store results after searching. Use the TT best move as the highest-priority move for ordering. Handle mate score adjustment when storing and retrieving scores at different plies from root.

#### Phase P02 — Search Enhancements (TT-Dependent)

- **T01**: TT-based move ordering — when a TT probe returns a hit with a best move, place that move first in the move list to maximize alpha-beta cutoff efficiency.
- **T02**: Internal iterative deepening (IID) — at nodes with high remaining depth where no TT move is available, perform a reduced-depth search first to obtain a move for ordering, then search with the full depth using that move first.
- **T03**: Repetition detection — maintain a position history stack of Zobrist hashes across the game. Detect threefold repetition and fifty-move rule draws. Return a draw score when repetition is detected during search.

### Exit Criteria

1. TT correctly stores and retrieves entries with proper Zobrist key verification
2. Replacement policy evicts stale/shallow entries in favor of deeper/newer entries
3. TT integration reduces node counts compared to search without TT on the same positions and depths
4. TT move is placed first in move ordering when a TT hit provides a best move
5. IID reduces total node count compared to searching without IID on the same positions and depths at high-depth nodes that lack a TT entry
6. Mate scores are correctly adjusted when stored/retrieved from TT at different plies from root
7. Threefold repetition is correctly detected and returns a draw score during search
8. Fifty-move rule is correctly detected and returns a draw score during search
9. Engine plays at approximately 1500 Elo level
10. All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`

### Dependencies

- **M01** (Core Types & Board Representation): `Bitboard`, `Square`, `Move`, `Piece`, `Position`, Zobrist hashing infrastructure (already implemented in `crates/board/src/zobrist.rs`)
- **M02** (Move Generation): Legal move generation used by search
- **M03** (Basic Search & Evaluation): Negamax with alpha-beta, quiescence search, iterative deepening, and move ordering framework (implemented in `crates/search/src/lib.rs`)
- **M04** (UCI Protocol): UCI command loop and time management — the TT integrates into the existing search called by the UCI engine loop