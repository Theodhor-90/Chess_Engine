# T01 — History Heuristic

## Objective

Implement a `HistoryTable` struct in the search crate that accumulates scores for quiet moves causing beta cutoffs, indexed by `[side][from_square][to_square]`. Integrate it into the move ordering pipeline so that quiet moves are ranked by their historical success at causing cutoffs.

## Deliverables

- A new file `crates/search/src/history.rs` containing the `HistoryTable` struct
- Updated `crates/search/src/ordering.rs` — `score_move` and `order_moves` functions use history scores for quiet move ordering
- Updated `crates/search/src/lib.rs` — `SearchContext` gains a `HistoryTable` field
- Updated search loop to call history table updates on beta cutoffs and on moves that fail to cause cutoffs

## Implementation Details

### HistoryTable Structure
- Indexed by `[side][from_square][to_square]` — 2 × 64 × 64 = 8,192 entries
- Each entry is a score (i32 or similar, approximately 4 bytes per entry, ~32 KB total)
- Provides methods: `update(side, from, to, bonus)`, `score(side, from, to) -> i32`, `clear()`

### Gravity Formula for Score Updates
- On each update: `score = score - score * |bonus| / 16384 + bonus`
- For moves causing beta cutoffs: `bonus = depth * depth`
- For moves that did NOT cause cutoffs (searched but failed to improve alpha): `bonus = -(depth * depth)`
- This naturally bounds scores within approximately `[-16384, 16384]` without requiring explicit clears between iterative deepening iterations
- Recent information naturally outweighs stale data

### Integration into Search
- Add `HistoryTable` as a field on `SearchContext`
- In the search loop, after a quiet move causes a beta cutoff, call `history.update(side, from, to, depth * depth)`
- Also apply negative bonus to all quiet moves searched before the cutoff move (fail moves): `history.update(side, from, to, -(depth * depth))`
- Clear history table on `ucinewgame` command

### Move Ordering Integration
- In `score_move`/`order_moves`, quiet moves (non-captures, non-killers, non-TT, non-PV) receive a score based on `history.score(side, from, to)`
- Ordering priority remains: TT move > PV move > good captures > killer moves > history-ordered quiet moves > bad captures

## Dependencies

- No prior tasks in this phase (this is T01)
- Depends on existing `SearchContext`, move ordering infrastructure (MVV-LVA, killer moves, PV move), and basic search loop from M03/M05
- Depends on M06/P01 (pruning techniques) being in place

## Verification Criteria

1. `HistoryTable` unit tests pass — verify gravity formula correctly bounds scores, verify clear resets all entries to zero
2. History scores are correctly updated on beta cutoffs (positive bonus) and on non-cutoff quiet moves (negative bonus)
3. Move ordering function uses history scores for quiet moves
4. `cargo test` passes with no regressions
5. `cargo clippy -- -D warnings` and `cargo fmt --check` pass
6. History heuristic contributes to the ≥5% node count reduction target (measured in combination with T02 counter-move heuristic at depth 10 on startpos, Kiwipete, and perft suite positions vs. baseline with only MVV-LVA, killer, TT, and PV move ordering)