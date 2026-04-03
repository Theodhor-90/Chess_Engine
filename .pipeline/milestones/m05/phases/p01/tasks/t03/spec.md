# T03 — TT Integration into Search

## Objective

Integrate the transposition table into the existing `negamax` search function in `crates/search/src/lib.rs`. This is where the TT delivers its value — avoiding redundant work by cutting off already-searched positions and improving move ordering with the TT best move.

## Deliverables

- **TT added to `SearchContext`** — add a `TranspositionTable` field to the `SearchContext` struct (or equivalent search state struct) so it persists across iterative deepening iterations
- **TT probe at node entry** — at the start of `negamax` (after repetition/draw checks, before move generation), probe the TT:
  - If hit and stored depth ≥ remaining depth:
    - `Exact` bound: return stored score directly
    - `LowerBound`: if stored score ≥ beta, return beta cutoff
    - `UpperBound`: if stored score ≤ alpha, return alpha cutoff
  - Optionally tighten alpha/beta bounds from TT info even when not cutting off
- **TT store after search** — after searching all moves at a node, store the result:
  - Determine bound type: `Exact` if best score improved alpha but didn't cause a beta cutoff at the original alpha; `LowerBound` if beta cutoff; `UpperBound` if no move improved alpha
  - Store: depth, score, bound type, best move, current generation age
- **Mate score adjustment** — when storing: convert root-relative mate scores to node-relative by subtracting ply. When retrieving: convert back by adding ply. This ensures mate scores remain correct when the same position is probed at a different distance from the root.
- **TT move ordering** — extract the best move from a TT hit and pass it to `order_moves` in `crates/search/src/ordering.rs` as the highest-priority move. Modify `order_moves` if needed to accept the TT move (it already accepts a `pv_move: Option<Move>` — generalize or add a `tt_move` parameter).
- **Generation management** — call `new_generation()` on the TT at the start of each `go` command / iterative deepening search to advance the age counter.

## Files to Modify

- `crates/search/src/lib.rs` — add TT to `SearchContext`, integrate probe/store into `negamax`
- `crates/search/src/ordering.rs` — accept TT move for highest-priority ordering
- `crates/search/src/tt.rs` — add any helper methods needed (e.g., mate score adjustment utilities)

## Implementation Constraints

- Mate score detection: scores beyond a threshold (e.g., `MATE_SCORE - MAX_PLY`) are treated as mate scores for adjustment purposes
- The TT probe must not cause incorrect results — verify that bound type logic is correct for the negamax framework (scores are from the current player's perspective)
- Do not probe TT at the root node for cutoffs (still probe for move ordering) — the root must always complete a full search to return a best move
- TT size should be configurable; default to a reasonable size (e.g., 64 MB or 128 MB)
- No `unwrap()` in library code — use `Option`/`Result` propagation

## Dependencies

- **T01** (this phase) — `TtEntry`, `BoundType`
- **T02** (this phase) — `TranspositionTable` with `probe`, `store`, `new_generation`
- **M03** (prior milestone) — existing `negamax`, `SearchContext`, `order_moves` in `crates/search/`

## Verification Criteria

1. **Node count reduction test**: search the same position to the same depth with and without TT. The TT-enabled search must visit fewer nodes.
2. **Mate score test**: search a known mate-in-N position. Verify the returned score is correct and that TT entries with mate scores retrieved at different plies still produce the correct mate distance.
3. **TT move ordering test**: after searching a position, probe the TT for that position and verify a best move is stored. Verify that `order_moves` places the TT move first.
4. **Correctness test**: search results (best move, score) must be identical or equivalent (within TT-induced ordering differences) to search without TT — the TT must not change the search result, only speed it up.
5. **Generation test**: after calling `new_generation()`, new stores use the updated age, and stale entries can be evicted.
6. `cargo test -p chess-search` passes
7. `cargo clippy -- -D warnings` produces no warnings
8. `cargo fmt --check` passes
9. The engine still responds correctly to UCI commands (`go depth N` returns a `bestmove`)