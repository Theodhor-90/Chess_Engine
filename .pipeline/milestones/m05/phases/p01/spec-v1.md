Now I have all the context I need. Let me produce the phase specification.

# Phase P01 — Transposition Table

## Goal

Design and implement the transposition table (TT) that caches search results keyed by Zobrist hash, and integrate it into the existing alpha-beta search in `crates/search/`, enabling the engine to avoid redundant work by recognizing transpositions — positions reached via different move orders. This phase delivers the TT entry structure, the hash table container with its replacement policy, and full search integration including TT-based move ordering and mate-score adjustment.

## Design Decisions

1. **TT lives in `crates/search/`** — The TT is a search-layer concern. It will be a new module (`tt.rs`) alongside the existing `ordering.rs`, `killer.rs`, and `pv_table.rs`. No new crate is needed.

2. **Power-of-two sizing with bitmask indexing** — The table size is always a power of two so that the index can be computed as `hash & (num_entries - 1)` instead of an expensive modulo operation. The caller specifies the table size in megabytes; the constructor computes the largest power-of-two entry count that fits.

3. **Entry packing** — Each TT entry stores: a 16-bit key verification fragment (upper bits of the Zobrist hash not used for indexing), 8-bit depth, 32-bit score, 2-bit bound type (exact/lower/upper), 16-bit best move (raw `Move` bits), and 8-bit age. This allows the full entry to fit in a compact, cache-friendly struct. The key fragment is sufficient for collision filtering; full 64-bit key storage is unnecessary given the hash quality.

4. **Replacement policy: depth-preferred with age-based eviction** — When a collision occurs, the new entry replaces the existing one if (a) the existing entry is from a previous search generation (stale), or (b) the new entry has equal or greater depth. This balances preserving deep, expensive results while clearing out stale data.

5. **Mate score adjustment at store/retrieve time** — Mate scores are relative to the root in search but must be stored relative to the current node so they remain correct when retrieved at a different ply. On store: adjust away from root. On retrieve: adjust toward root. The adjustment is `±ply` added to or subtracted from the score.

6. **TT move as highest-priority in move ordering** — The existing `order_moves` function in `ordering.rs` already accepts a `pv_move: Option<Move>` parameter. The TT move will be passed through a similar mechanism or the existing parameter will be generalized to accept the TT move when no PV move is available, giving it the highest ordering priority.

7. **No external dependencies** — The TT is implemented using the standard library only, consistent with the project convention for core engine logic.

## Tasks

### T01 — TT Entry Structure

Define the `TtEntry` struct and the `BoundType` enum in a new `crates/search/src/tt.rs` module. The entry contains: key verification bits, search depth, score, bound type (exact/lower/upper), best move, and age. Provide methods to construct, read, and compare entries. Include unit tests verifying encode/decode round-trips and field accessors.

**Deliverables:**
- `crates/search/src/tt.rs` with `TtEntry` struct and `BoundType` enum
- Constructor and accessor methods for all fields
- Unit tests for round-trip correctness of all fields

### T02 — TT Hash Table with Replacement Policy

Implement the `TranspositionTable` struct that wraps a `Vec<TtEntry>` with power-of-two sizing. Provide `probe(hash: u64) -> Option<TtEntry>` and `store(hash: u64, entry: TtEntry)` methods. The `store` method implements the replacement policy: prefer deeper entries, evict stale entries from prior search generations. Provide `clear()` and `resize(mb: usize)` methods, and a method to increment the search age/generation. Include unit tests verifying: correct probe/store round-trips, replacement policy behavior (deeper replaces shallow, new-age replaces old-age, shallow does not replace deeper same-age), and power-of-two sizing.

**Deliverables:**
- `TranspositionTable` struct with `probe`, `store`, `clear`, `resize`, and `new_generation` methods
- Replacement policy: depth-preferred with age-based eviction
- Unit tests for probe/store, replacement policy, and sizing

### T03 — TT Integration into Search

Integrate the transposition table into the existing `negamax` function in `crates/search/src/lib.rs`. Before searching a node, probe the TT — if the entry depth is sufficient and the bound type permits a cutoff, return the stored score. After searching, store the result in the TT. Pass the TT best move to `order_moves` as the highest-priority move. Implement mate score adjustment: when storing, convert mate scores from root-relative to node-relative; when retrieving, convert back. Add the `TranspositionTable` to `SearchContext` so it persists across iterative deepening iterations. Include tests verifying: TT cutoffs reduce the node count compared to searching without TT on the same position and depth, mate scores are correctly adjusted across plies, and the TT move is ordered first.

**Deliverables:**
- TT probe at the start of `negamax` with cutoff logic and bound-type handling
- TT store at the end of `negamax` with the searched result
- Mate score adjustment on store and retrieve
- TT best move passed to move ordering as highest priority
- `TranspositionTable` added to `SearchContext`
- Tests comparing node counts with/without TT, mate score adjustment, and TT move ordering

## Exit Criteria

1. TT correctly stores and retrieves entries with proper Zobrist key verification
2. Replacement policy evicts stale/shallow entries in favor of deeper/newer entries
3. TT integration reduces node counts compared to search without TT on the same positions and depths
4. TT move is placed first in move ordering when a TT hit provides a best move
5. Mate scores are correctly adjusted when stored/retrieved from TT at different plies from root

## Dependencies

- **M01** (Core Types & Board Representation): `Bitboard`, `Square`, `Move`, `Piece`, `Position`, Zobrist hashing (`crates/board/src/zobrist.rs`)
- **M02** (Move Generation): Legal move generation used by search
- **M03** (Basic Search & Evaluation): Negamax with alpha-beta, quiescence search, iterative deepening, and move ordering framework (`crates/search/src/lib.rs`)
- **M04** (UCI Protocol): UCI command loop and time management — the TT integrates into the existing search called by the UCI engine loop
- No dependency on M05/P02 (this phase comes first)

## Artifacts

- `crates/search/src/tt.rs` — New module containing `TtEntry`, `BoundType`, and `TranspositionTable`
- `crates/search/src/lib.rs` — Modified to integrate TT probe/store into `negamax`, add `TranspositionTable` to `SearchContext`
- `crates/search/src/ordering.rs` — Modified to accept TT move for highest-priority ordering