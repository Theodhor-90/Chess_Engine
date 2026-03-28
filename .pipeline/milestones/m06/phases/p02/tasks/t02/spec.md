# T02 — Counter-Move Heuristic

## Objective

Implement a `CounterMoveTable` struct that stores a single refutation move for each `(piece, to_square)` of the previous move. Integrate it into the move ordering pipeline as a signal ranked between killer moves and plain history-ordered quiet moves.

## Deliverables

- A new file `crates/search/src/countermove.rs` containing the `CounterMoveTable` struct
- Updated `crates/search/src/ordering.rs` — `score_move` and `order_moves` assign counter-moves a priority between killers and history-ordered quiet moves
- Updated `crates/search/src/lib.rs` — `SearchContext` gains a `CounterMoveTable` field
- Updated search loop to pass previous move context into the ordering function and to store counter-moves on beta cutoffs

## Implementation Details

### CounterMoveTable Structure
- Indexed by `[piece_kind][to_square]` of the *previous* move — 6 piece kinds × 64 squares = 384 entries
- Each entry stores a single `Move` (the counter-move that refuted the previous move)
- Provides methods: `store(piece, to_square, counter_move)`, `lookup(piece, to_square) -> Option<Move>`, `clear()`

### Integration into Search
- Add `CounterMoveTable` as a field on `SearchContext`
- In the search loop, when a quiet move causes a beta cutoff, store it as the counter-move for the previous move's `(piece, to_square)`
- The search function must track and pass the previous move to the move ordering function
- Clear counter-move table on `ucinewgame` command

### Move Ordering Integration
- In `score_move`/`order_moves`, if a quiet move matches the counter-move for the current previous-move context, assign it a priority between killer moves and plain history-ordered quiet moves
- Updated ordering hierarchy: TT move > PV move > good captures > killer moves > counter-move > history-ordered quiet moves > bad captures

## Dependencies

- **T01 (History Heuristic)** — history table and updated move ordering infrastructure must be in place so counter-move integrates cleanly into the ordering pipeline
- Depends on existing `SearchContext`, move ordering infrastructure, and basic search loop from M03/M05
- Requires piece type information from M01 types crate and move encoding

## Verification Criteria

1. `CounterMoveTable` unit tests pass — verify store/lookup round-trips, verify clear resets all entries
2. Counter-moves are stored on quiet move beta cutoffs with the correct previous-move context
3. Move ordering function correctly assigns counter-move priority between killers and history-ordered quiet moves
4. `cargo test` passes with no regressions
5. `cargo clippy -- -D warnings` and `cargo fmt --check` pass
6. Combined with T01, history and counter-move heuristics achieve ≥5% reduction in nodes searched at depth 10 on startpos, Kiwipete, and perft suite positions compared to baseline with only MVV-LVA, killer, TT, and PV move ordering