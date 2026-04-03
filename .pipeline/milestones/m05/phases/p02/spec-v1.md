Now I have a thorough understanding of the codebase. Let me produce the phase specification.

# Phase P02 — Search Enhancements (TT-Dependent)

## Goal

Build on the transposition table delivered in Phase P01 to implement three TT-dependent search enhancements: dedicated TT-based move ordering that ensures the TT best move is always searched first, internal iterative deepening (IID) that performs a reduced-depth search to find a move for ordering when no TT move is available, and repetition detection that maintains a position history of Zobrist hashes to detect threefold repetition and fifty-move rule draws during search.

## Design Decisions

1. **TT move ordering refines the existing `ordering.rs` integration** — Phase P01 already passes the TT move through `order_moves` with the highest-priority score (`TT_SCORE = 200_000`). Task T01 ensures this path is robust: the TT move is validated as a legal move before being used for ordering, and the ordering contract is verified by dedicated tests that confirm the TT move always appears first regardless of whether it is a capture, quiet move, or promotion.

2. **IID triggers above a depth threshold when no TT move is available** — IID performs a reduced-depth search (depth reduced by 2) at nodes where the remaining depth exceeds a threshold (e.g., depth >= 4) and no TT move was found on probe. The reduced search populates the TT, so the subsequent full-depth search benefits from TT move ordering. IID reuses the existing `negamax` function — no separate search routine is needed.

3. **Position history is a `Vec<u64>` of Zobrist hashes stored in `SearchContext`** — The history is initialized from the game's move history (provided via UCI `position` command with moves) before search begins. During search, `make_move` pushes the new hash and `unmake_move` pops it. Threefold repetition is detected by counting occurrences of the current hash in the history. The search function signature for `search()` gains a `game_history: &[u64]` parameter to receive the pre-search position hashes from the engine binary.

4. **Repetition check scans backward from the current position** — Only positions since the last irreversible move (capture or pawn move, indicated by halfmove clock reset) need to be checked, since a capture or pawn move makes it impossible to repeat an earlier position. This bounds the scan to at most 100 entries (the fifty-move rule limit) rather than the entire game history.

5. **Draw score is 0** — When repetition or the fifty-move rule is detected, the search returns a score of 0 (draw). No contempt value is applied at this stage.

6. **All changes are in `crates/search/` and `crates/engine/`** — T01 and T02 modify `crates/search/src/lib.rs`. T03 adds the history tracking to `SearchContext` in `crates/search/src/lib.rs` and updates `crates/engine/src/main.rs` to build the game history from UCI position moves.

## Tasks

### T01 — TT-Based Move Ordering

Verify and harden the TT move ordering path established in P01. Add a legality check: before passing the TT move to `order_moves`, confirm it appears in the legal move list for the current position (a TT collision could produce an invalid move). Add tests that confirm: (a) the TT move is always ordered first across varied positions, (b) an invalid TT move from a hash collision is discarded rather than ordered first, (c) TT move ordering produces fewer nodes searched than ordering without the TT move on a set of test positions.

**Deliverables:**
- Legality validation for TT moves before passing to `order_moves` in `crates/search/src/lib.rs`
- Tests verifying TT move is ordered first, invalid TT moves are discarded, and node reduction from TT move ordering

### T02 — Internal Iterative Deepening (IID)

Implement IID in the `negamax` function. At nodes where the remaining depth is at or above a threshold and no TT move was found on probe, perform a reduced-depth search to populate the TT, then re-probe the TT to obtain a move for ordering. Add tests that confirm: (a) IID reduces total node count compared to search without IID on positions at sufficient depth, (b) IID does not trigger at shallow depths, (c) IID correctly finds a move that the subsequent full-depth search uses for ordering.

**Deliverables:**
- IID logic in `negamax` in `crates/search/src/lib.rs` with configurable depth threshold
- Tests comparing node counts with/without IID and verifying threshold behavior

### T03 — Repetition Detection

Add a position history stack (`Vec<u64>`) to `SearchContext` that tracks Zobrist hashes. Update the `search` function signature to accept a `game_history: &[u64]` parameter containing hashes from the UCI position setup. During search, push/pop hashes around `make_move`/`unmake_move`. At the start of `negamax`, before the TT probe, check for threefold repetition and fifty-move rule — return a draw score of 0 if detected. Update `crates/engine/src/main.rs` to build the game history from the UCI `position` command's move list and pass it to `search`. Add tests that confirm: (a) threefold repetition is detected and returns draw score, (b) fifty-move rule is detected and returns draw score, (c) the engine avoids repeating positions when winning, (d) the engine seeks repetition when losing.

**Deliverables:**
- Position history stack in `SearchContext` in `crates/search/src/lib.rs`
- Repetition detection (threefold) and fifty-move rule check in `negamax`
- Updated `search` function signature accepting `game_history: &[u64]`
- Updated `crates/engine/src/main.rs` to build and pass game history
- Tests for threefold repetition, fifty-move rule, and search behavior around repetitions

## Exit Criteria

1. TT move is placed first in move ordering when a TT hit provides a best move
2. Invalid TT moves from hash collisions are discarded before ordering
3. IID reduces total node count compared to searching without IID at high-depth nodes that lack a TT entry
4. Threefold repetition is correctly detected and returns a draw score during search
5. Fifty-move rule is correctly detected and returns a draw score during search
6. All existing tests in `crates/search/` continue to pass
7. `cargo test` passes across the full workspace

## Dependencies

- **M05/P01** (Transposition Table): TT data structure, probe/store integration, TT move ordering infrastructure, and mate score adjustment — all prerequisite for T01 (refines TT move ordering), T02 (relies on TT probes), and T03 (complements TT-based search scoring)
- **M01** (Core Types & Board Representation): `Bitboard`, `Square`, `Move`, `Piece`, `Position`, Zobrist hashing (`crates/board/src/zobrist.rs`)
- **M02** (Move Generation): Legal move generation used by search
- **M03** (Basic Search & Evaluation): Negamax with alpha-beta, quiescence search, iterative deepening, and move ordering framework (`crates/search/src/lib.rs`)
- **M04** (UCI Protocol): UCI command loop and time management — the engine binary builds game history from the UCI `position` command

## Artifacts

- `crates/search/src/lib.rs` — Modified: IID logic in `negamax`, position history stack in `SearchContext`, repetition and fifty-move rule detection, updated `search` function signature, TT move legality validation
- `crates/search/src/ordering.rs` — Unchanged (TT move ordering already functional from P01)
- `crates/engine/src/main.rs` — Modified: build game history `Vec<u64>` from UCI position moves, pass to `search`