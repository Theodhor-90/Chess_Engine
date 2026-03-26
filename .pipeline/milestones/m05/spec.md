## Milestone M05 — Transposition Table & Hash

### Goal

Massive search efficiency gain. The transposition table avoids re-searching positions reached via different move orders. Target: ~1500 Elo.

### Phases

#### Phase P01 — Transposition Table
- **T01**: TT entry structure — Zobrist key (or verification bits), depth, score, bound type (exact/lower/upper), best move, age.
- **T02**: TT implementation — fixed-size hash table with replacement policy (prefer deeper entries, age-based eviction). Power-of-two sizing for fast modulo.
- **T03**: TT integration into search — probe before searching, store after searching. Use TT move for move ordering (highest priority). Handle score adjustment for mate scores stored at different plies.

#### Phase P02 — Search Enhancements (TT-Dependent)
- **T01**: TT-based move ordering — TT hit provides best move from prior search; place it first in move list.
- **T02**: Internal iterative deepening (IID) — if no TT move available at high depth, do a shallower search first to find a move to order first.
- **T03**: Repetition detection — track position history via Zobrist hashes. Draw by threefold repetition and fifty-move rule. Integrate into search scoring.

### Exit Criteria
- Transposition table correctly stores and retrieves entries with proper key verification
- Replacement policy correctly evicts stale/shallow entries in favor of deeper/newer ones
- TT integration reduces node counts significantly vs search without TT
- TT move ordering places the hash move first, measurably improving search efficiency
- IID produces useful move ordering when no TT move is available
- Repetition detection correctly identifies threefold repetition and fifty-move draws
- Mate scores are correctly adjusted when stored/retrieved from TT at different plies
- Engine plays at ~1500 Elo level
- All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`