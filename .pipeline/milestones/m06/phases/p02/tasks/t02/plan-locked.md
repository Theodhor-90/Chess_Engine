I now have all the information needed to write the implementation plan. Here it is:

## 1. Deliverables

- **`crates/search/src/countermove.rs`** (new) — `CounterMoveTable` struct indexed by `[PieceKind][Square]` of the previous move, storing a single refutation `Move` per entry. Provides `store()`, `lookup()`, and `clear()` methods.
- **`crates/search/src/ordering.rs`** (modified) — Updated `score_move` and `order_moves` functions to accept a `CounterMoveTable` reference and an `Option<Move>` for the previous move; assign counter-move matches a score between `KILLER_SCORE` and plain history-scored quiet moves.
- **`crates/search/src/lib.rs`** (modified) — New `pub mod countermove;` declaration, `CounterMoveTable` field added to `SearchContext`, `negamax` updated to track the previous move and pass it to `order_moves`, and to store counter-moves on quiet-move beta cutoffs; `search` function and `test_ctx` helper updated to initialize the new field; `quiescence` call sites updated for the new `order_moves` signature.

## 2. Dependencies

No new external dependencies. The implementation uses only existing crate types:
- `chess_types::{Move, PieceKind, Square}` — for table indexing and move representation
- `chess_board::Position` — for `piece_on()` to extract the previous move's piece kind

## 3. Implementation Details

### `crates/search/src/countermove.rs`

**Purpose**: Store a single counter-move (the quiet move that refuted the opponent's last move) for each `(PieceKind, Square)` combination of the previous move.

**Exports**: `CounterMoveTable`

**Structure**:
```rust
use chess_types::{Move, PieceKind, Square};

pub struct CounterMoveTable {
    table: [[Option<Move>; Square::COUNT]; PieceKind::COUNT],
}
```

This is a `6 × 64 = 384` entry table, where each entry is `Option<Move>` (3 bytes each, ~1.2 KB total).

**Methods**:

- `new() -> CounterMoveTable` — Initialize all entries to `None`.
- `store(&mut self, piece: PieceKind, to: Square, counter_move: Move)` — Set `table[piece as usize][to.index() as usize] = Some(counter_move)`.
- `lookup(&self, piece: PieceKind, to: Square) -> Option<Move>` — Return `table[piece as usize][to.index() as usize]`.
- `clear(&mut self)` — Reset all entries to `None`.

**Trait impls**: `Default` (delegates to `new()`).

**Tests** (colocated `#[cfg(test)] mod tests`):
- `store_and_lookup_round_trip` — Store a move, verify lookup returns it.
- `lookup_returns_none_initially` — Verify fresh table returns `None` for any lookup.
- `clear_resets_all_entries` — Store several entries, clear, verify all return `None`.
- `overwrite_replaces_previous` — Store two different moves for the same `(piece, square)`, verify the second overwrites the first.

### `crates/search/src/ordering.rs`

**Purpose**: Integrate counter-move scoring into the move ordering pipeline.

**Changes**:

1. Add `use crate::countermove::CounterMoveTable;` import.

2. Add a new constant `COUNTER_MOVE_SCORE: i32 = 15_000` — this positions counter-moves between `KILLER_SCORE` (20,000) and history-scored quiet moves (which range in `[-16384, 16384]`).

3. Update `score_move` signature to accept two additional parameters:
   - `countermoves: &CounterMoveTable`
   - `prev_move: Option<Move>` (the previous move, needed for counter-move lookup)
   - Also needs `prev_piece: Option<PieceKind>` (the piece kind of the previous move)

   In the scoring chain, after the killer check and before the history fallback, add:
   ```rust
   } else if let (Some(pm_piece), Some(pm)) = (prev_piece, prev_move) {
       if countermoves.lookup(pm_piece, pm.to_sq()) == Some(mv) {
           COUNTER_MOVE_SCORE
       } else {
           history.score(side, mv.from_sq(), mv.to_sq())
       }
   } else {
       history.score(side, mv.from_sq(), mv.to_sq())
   }
   ```

   Actually, to keep the logic cleaner and avoid deep nesting, the counter-move check should be a distinct branch in the if-else chain. The full scoring order becomes:
   - `tt_move == Some(mv)` → `TT_SCORE`
   - `pv_move == Some(mv)` → `PV_SCORE`
   - `mv.is_capture()` → `CAPTURE_BASE + score_mvv_lva(mv, pos)`
   - `killers.is_killer(ply, mv)` → `KILLER_SCORE`
   - counter-move match → `COUNTER_MOVE_SCORE`
   - else → `history.score(side, mv.from_sq(), mv.to_sq())`

4. Update `order_moves` signature to accept the same additional parameters and pass them through to `score_move`.

### `crates/search/src/lib.rs`

**Purpose**: Wire the `CounterMoveTable` into the search context and search loop.

**Changes**:

1. Add `pub mod countermove;` to module declarations (after `pub mod history;`).

2. Add `use countermove::CounterMoveTable;` import.

3. Add `countermove_table: CounterMoveTable` field to `SearchContext`.

4. Update `negamax` function signature to accept an additional parameter `prev_move: Option<(PieceKind, Move)>` — this carries both the piece kind and the move itself for the previous move. Alternatively, since the previous move's piece information can be extracted from the position before the move is made, we pass a structured tuple.

   However, to minimize signature changes to `negamax` (which is recursive and called from many places), a cleaner approach is to store the previous move information in `SearchContext` or pass it only to `order_moves`. Looking at the codebase pattern:

   The cleanest approach: add a `prev_move: Option<(PieceKind, Move)>` parameter to `negamax`. In the search loop inside `negamax`, before each recursive call, we know the current move `mv` and can extract its piece kind via `pos.piece_on(mv.from_sq())` (before `make_move`). We pass `Some((piece_kind, mv))` as `prev_move` into the recursive call. The top-level call from `search` passes `None`.

5. In the `negamax` body:
   - Pass `&ctx.countermove_table`, `prev_move.map(|(_, m)| m)`, and `prev_move.map(|(pk, _)| pk)` to `order_moves`.
   - On beta cutoff for a quiet move, after the existing history and killer updates, add:
     ```rust
     if let Some((prev_piece, prev_mv)) = prev_move {
         ctx.countermove_table.store(prev_piece, prev_mv.to_sq(), mv);
     }
     ```
   - Each recursive `negamax` call needs to pass the current move as `prev_move`. Before `make_move`, extract the piece kind:
     ```rust
     let piece_kind = pos.piece_on(mv.from_sq()).expect("piece must exist on from_sq").kind;
     ```
     Then pass `Some((piece_kind, mv))` to the recursive `negamax` call.

6. Update the null-move pruning recursive call to pass `None` as `prev_move` (null move has no piece/square context).

7. Update the IID recursive call to pass `prev_move` (or `None` — passing the same `prev_move` from the current frame is more accurate).

8. Update `quiescence` call sites in `order_moves` — `quiescence` calls `order_moves` internally. Update that call to pass the new counter-move parameters (`&ctx.countermove_table`, `None`, `None` since quiescence doesn't track previous moves).

9. Update the `search` function to initialize `countermove_table: CounterMoveTable::new()` in the `SearchContext` constructor.

10. Update `test_ctx()` to include `countermove_table: CounterMoveTable::new()`.

## 4. API Contracts

**`CounterMoveTable::store(piece: PieceKind, to: Square, counter_move: Move)`**
- Input: The piece kind and destination square of the previous move, plus the move that refuted it.
- Effect: Overwrites the entry at `[piece][to]` with `counter_move`.

**`CounterMoveTable::lookup(piece: PieceKind, to: Square) -> Option<Move>`**
- Input: The piece kind and destination square of the previous move.
- Output: `Some(move)` if a counter-move is stored, `None` otherwise.

**`CounterMoveTable::clear()`**
- Effect: Resets all 384 entries to `None`.

**`order_moves` updated signature**:
```rust
pub fn order_moves(
    moves: &mut [Move],
    pos: &Position,
    killers: &KillerTable,
    history: &HistoryTable,
    countermoves: &CounterMoveTable,
    ply: u8,
    pv_move: Option<Move>,
    tt_move: Option<Move>,
    side: Color,
    prev_move: Option<Move>,
    prev_piece: Option<PieceKind>,
)
```

**`negamax` updated signature**:
```rust
pub fn negamax(
    pos: &mut Position,
    depth: u8,
    alpha: i32,
    beta: i32,
    ply: u8,
    allow_null: bool,
    ctx: &mut SearchContext,
    prev_move: Option<(PieceKind, Move)>,
) -> (i32, Option<Move>)
```

## 5. Test Plan

### `crates/search/src/countermove.rs` — Unit Tests

**Test setup**: Create `CounterMoveTable` instances and quiet `Move` values using `Move::new(Square, Square, MoveFlag::QUIET)`.

1. **`store_and_lookup_round_trip`**: Store a counter-move for `(PieceKind::Knight, Square::F3)`, verify `lookup` returns `Some(stored_move)`.

2. **`lookup_returns_none_initially`**: On a fresh table, verify `lookup(PieceKind::Pawn, Square::E4)` returns `None`.

3. **`clear_resets_all_entries`**: Store entries for several `(piece, square)` combinations, call `clear()`, verify all return `None`.

4. **`overwrite_replaces_previous`**: Store move A for `(PieceKind::Rook, Square::D4)`, then store move B for the same key, verify `lookup` returns `Some(B)`.

### `crates/search/src/ordering.rs` — Updated Tests

5. **`counter_move_ordered_between_killers_and_quiet`**: Set up a position with captures and quiet moves. Store a counter-move matching one of the quiet moves. Verify that after ordering, the counter-move appears after killer moves but before other quiet moves.

### Integration — existing `cargo test` must pass

6. All existing tests in `lib.rs`, `ordering.rs`, `history.rs`, and `killer.rs` must continue to pass after the signature changes. The existing ordering tests need their `order_moves` calls updated to pass the new `countermoves`, `prev_move`, and `prev_piece` parameters (using `CounterMoveTable::new()`, `None`, `None` for tests that don't exercise counter-move behavior).

## 6. Implementation Order

1. **Create `crates/search/src/countermove.rs`** — Implement `CounterMoveTable` with `new()`, `store()`, `lookup()`, `clear()`, `Default` impl, and unit tests. This is self-contained with no dependencies on other changes.

2. **Update `crates/search/src/ordering.rs`** — Add `COUNTER_MOVE_SCORE` constant, update `score_move` and `order_moves` signatures to accept `CounterMoveTable`, `prev_move`, and `prev_piece` parameters. Add the counter-move check in the scoring chain between killers and history. Update all existing tests to pass the new parameters (using defaults). Add the `counter_move_ordered_between_killers_and_quiet` test.

3. **Update `crates/search/src/lib.rs`** — Add `pub mod countermove;`, add `CounterMoveTable` field to `SearchContext`, update `negamax` signature to accept `prev_move`, update all `negamax` recursive calls to pass the previous move context, update `order_moves` call sites (both in `negamax` and `quiescence`) with the new parameters, add counter-move storage on beta cutoffs, update `search()` and `test_ctx()` to initialize the new field.

## 7. Verification Commands

```bash
# Check formatting
cargo fmt --check

# Check lints
cargo clippy -- -D warnings

# Run all tests (includes unit tests in countermove.rs, ordering.rs, lib.rs)
cargo test

# Run only search crate tests
cargo test -p chess-search

# Run specific countermove tests
cargo test -p chess-search countermove

# Run specific ordering tests
cargo test -p chess-search ordering

# Build the full project
cargo build
```

## 8. Design Deviations

**Deviation 1: `negamax` receives `prev_move` as a parameter instead of storing it in `SearchContext`**

- **Parent spec states**: "The search function must track and pass the previous move to the move ordering function."
- **Chosen approach**: Rather than storing a mutable `prev_move` field on `SearchContext` (which would need to be saved/restored around each recursive call, creating error-prone state management), the previous move is passed as an explicit `Option<(PieceKind, Move)>` parameter to `negamax`. This is cleaner because each stack frame naturally owns its own previous-move context without needing save/restore logic.
- **Why this is correct**: The parameter flows naturally through recursion — the parent call knows the move it's about to search and passes it down. This matches how `allow_null` is already handled in the existing `negamax` signature.

**Deviation 2: Counter-move table is per-search, not persistent across searches**

- **Parent spec states**: "Clear counter-move table on `ucinewgame` command."
- **Observation**: The current architecture creates a fresh `SearchContext` (including `HistoryTable` and `KillerTable`) for every call to `search()`. The counter-move table follows the same pattern — it is created fresh each search. The `ucinewgame` clearing is therefore already handled implicitly since no state persists between searches.
- **If in a future task the engine is refactored to persist `SearchContext` across searches**, then an explicit `clear()` call would be added to the `UciNewGame` handler, just as would be needed for the history and killer tables. No special handling is required in this task.