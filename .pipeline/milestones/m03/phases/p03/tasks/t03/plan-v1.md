Now I have all the information needed. Let me produce the implementation plan.

## 1. Deliverables

| File | Action | Purpose |
|------|--------|---------|
| `crates/search/src/pv_table.rs` | Create | Triangular PV table implementation: stores the principal variation at each ply during search, provides methods to update PV lines, extract the root PV, and clear the table. |
| `crates/search/src/ordering.rs` | Modify | Integrate PV move scoring into the `score_move` function so the PV move for the current ply gets the highest ordering priority (above captures and killers). |
| `crates/search/src/lib.rs` | Modify | Declare `pv_table` module; extend `SearchContext` with a `PvTable` field and a `pv_line` field (the PV from the previous iteration); update `negamax` to populate the PV table during search; update `search` (iterative deepening) to extract the PV after each iteration and pass it to the next; pass PV move to `order_moves`; clear PV table at the start of each search. |

## 2. Dependencies

No new crate dependencies required. The implementation uses only `chess_types::Move` (already available) and standard library types. The `crates/search/Cargo.toml` remains unchanged.

## 3. Implementation Details

### `crates/search/src/pv_table.rs`

**Purpose**: Triangular PV table that collects the principal variation during search.

**Structure**:
```rust
use chess_types::Move;

const MAX_PLY: usize = 128;

pub struct PvTable {
    table: [[Option<Move>; MAX_PLY]; MAX_PLY],
    length: [usize; MAX_PLY],
}
```

The triangular property: at ply `p`, the PV line can have at most `MAX_PLY - p` moves. Row `p` stores the PV line from ply `p` onward. `length[p]` tracks how many moves are stored in row `p`.

**Exports and methods**:

- `PvTable::new() -> PvTable` — Creates a zeroed-out PV table (all `None`, all lengths 0).
- `PvTable::update(&mut self, ply: u8, mv: Move)` — Sets the PV at `ply` to `mv` followed by the PV from `ply + 1`. Specifically: `table[ply][0] = Some(mv)`, then copies `table[ply+1][0..length[ply+1]]` into `table[ply][1..]`, and sets `length[ply] = 1 + length[ply+1]`.
- `PvTable::get_pv_move(&self, ply: u8) -> Option<Move>` — Returns `table[ply][0]`, the PV move at the given ply.
- `PvTable::extract_pv(&self) -> Vec<Move>` — Returns the root PV line: `table[0][0..length[0]]` collected into a `Vec<Move>`, filtering out `None` values.
- `PvTable::clear(&mut self)` — Resets all entries to `None` and all lengths to 0.
- `PvTable::clear_ply(&mut self, ply: u8)` — Resets `length[ply] = 0`. Called at each node before searching children, so that if no child improves alpha, the PV at this ply is empty.
- Implement `Default` for `PvTable` delegating to `new()`.

**Key logic**:
- `update` is called in `negamax` whenever a move improves alpha. It records `mv` as the best move at this ply and copies the child's PV line (from `ply+1`) to form the full PV from this ply.
- `clear_ply` is called at the start of each node to reset the PV length for this ply, ensuring stale data from previous branches doesn't persist.

### `crates/search/src/ordering.rs`

**Purpose**: Add PV move as the highest-priority score in move ordering.

**Changes**:
- Add a `pv_move: Option<Move>` parameter to `score_move` and `order_moves`.
- Define a `PV_SCORE` constant with a value higher than any possible MVV-LVA score. Since the maximum MVV-LVA score is `900 * 100 - 100 = 89_900`, use `const PV_SCORE: i32 = 100_000;`.
- In `score_move`, check if `pv_move == Some(mv)` first. If so, return `PV_SCORE`. Otherwise, fall through to existing capture/killer/quiet scoring logic.

**Updated signatures**:
```rust
fn score_move(mv: Move, pos: &Position, killers: &KillerTable, ply: u8, pv_move: Option<Move>) -> i32
pub fn order_moves(moves: &mut [Move], pos: &Position, killers: &KillerTable, ply: u8, pv_move: Option<Move>)
```

### `crates/search/src/lib.rs`

**Purpose**: Wire PV table into the search framework.

**Changes to `SearchContext`**:
```rust
pub struct SearchContext {
    start: Instant,
    time_budget: Duration,
    nodes: u64,
    aborted: bool,
    killers: KillerTable,
    pv_table: PvTable,
    prev_pv: Vec<Move>,
}
```

- `pv_table`: The triangular PV table populated during search.
- `prev_pv`: The PV line extracted from the previous iterative deepening iteration. Used to determine the PV move at each ply during the current iteration.

**Add module declaration**: `pub mod pv_table;` alongside existing `pub mod killer;` and `pub mod ordering;`.

**Add import**: `use pv_table::PvTable;`.

**Helper method on `SearchContext`**:
- `fn pv_move_at(&self, ply: u8) -> Option<Move>` — Returns `self.prev_pv.get(ply as usize).copied()`. This gives the PV move for the current ply based on the previous iteration's PV.

**Changes to `negamax`**:
1. At the start of the function (after the abort/depth-0 checks, before iterating moves), call `ctx.pv_table.clear_ply(ply)`.
2. Update the `order_moves` call to pass `ctx.pv_move_at(ply)` as the `pv_move` argument.
3. When a move improves alpha (`score > alpha`), call `ctx.pv_table.update(ply, mv)` to record this move and its child's PV as the best line from this ply.

**Changes to `quiescence`**:
1. Update the `order_moves` call to pass `None` as the `pv_move` argument (PV move ordering is not applied in quiescence search — it uses only MVV-LVA and killers for tactical moves).

**Changes to `search` (iterative deepening)**:
1. Initialize `SearchContext` with `pv_table: PvTable::new()` and `prev_pv: Vec::new()`.
2. After each completed iteration (when `!ctx.aborted`), extract the PV: `ctx.prev_pv = ctx.pv_table.extract_pv();`.
3. Before each iteration, clear the PV table: `ctx.pv_table.clear();` (inside the loop, before calling `negamax`).

**Changes to `test_ctx` helper in tests**:
- Add `pv_table: PvTable::new()` and `prev_pv: Vec::new()` to the `SearchContext` construction.

## 4. API Contracts

**`PvTable::update(ply: u8, mv: Move)`**
- Input: ply index (0–127), the best move found at this ply
- Effect: Stores `mv` at `table[ply][0]`, copies `table[ply+1][0..length[ply+1]]` into `table[ply][1..]`, updates `length[ply] = 1 + length[ply+1]`

**`PvTable::extract_pv() -> Vec<Move>`**
- Input: none
- Output: Vector of moves representing the root PV line, e.g. `[e2e4, e7e5, g1f3]`

**`PvTable::get_pv_move(ply: u8) -> Option<Move>`**
- Input: ply index
- Output: `Some(move)` if a PV move exists at this ply, `None` otherwise

**`order_moves(moves: &mut [Move], pos: &Position, killers: &KillerTable, ply: u8, pv_move: Option<Move>)`**
- Input: mutable move slice, position for MVV-LVA scoring, killer table, current ply, optional PV move
- Effect: Sorts `moves` in descending priority: PV move (100,000) > captures by MVV-LVA > killers (50) > quiet moves (0)

## 5. Test Plan

### `crates/search/src/pv_table.rs` — Unit tests

**Test: `update_and_extract_single_ply`**
- Setup: Create `PvTable`, call `update(0, move_a)`.
- Assert: `extract_pv()` returns `[move_a]`. `get_pv_move(0)` returns `Some(move_a)`.

**Test: `update_chains_child_pv`**
- Setup: Create `PvTable`, call `update(1, move_b)`, then `update(0, move_a)`.
- Assert: `extract_pv()` returns `[move_a, move_b]` — the root PV includes the child's PV.

**Test: `clear_resets_table`**
- Setup: Create `PvTable`, call `update(0, move_a)`, then `clear()`.
- Assert: `extract_pv()` returns empty vec. `get_pv_move(0)` returns `None`.

**Test: `clear_ply_resets_single_ply`**
- Setup: Create `PvTable`, call `update(0, move_a)`, then `clear_ply(0)`.
- Assert: `extract_pv()` returns empty vec.

### `crates/search/src/ordering.rs` — Unit test additions

**Test: `pv_move_ordered_first`**
- Setup: Generate legal moves from a position with both captures and quiet moves. Pick one quiet move as the PV move. Call `order_moves` with `pv_move: Some(that_move)`.
- Assert: The PV move is at index 0 in the sorted list, before all captures.

### `crates/search/src/lib.rs` — Integration test

**Test: `pv_ordering_reduces_nodes`**
- Setup: Search a tactical position (e.g. Kiwipete or a middlegame) at depth 4+ with PV ordering active (normal code path). Record `ctx.nodes`. Then search the same position at the same depth but with `prev_pv` always empty (no PV move hint). Record that node count.
- Assert: The PV-ordered search visits fewer nodes than the non-PV search. This verifies exit criterion #4 ("measurable node-count reduction").

## 6. Implementation Order

1. **Create `crates/search/src/pv_table.rs`** — Implement the `PvTable` struct with `new`, `update`, `get_pv_move`, `extract_pv`, `clear`, `clear_ply`, and `Default`. Add unit tests.

2. **Modify `crates/search/src/ordering.rs`** — Add `PV_SCORE` constant, add `pv_move: Option<Move>` parameter to `score_move` and `order_moves`, update scoring logic to return `PV_SCORE` when `pv_move == Some(mv)`. Update existing tests to pass `None` for `pv_move` where PV ordering isn't being tested. Add `pv_move_ordered_first` test.

3. **Modify `crates/search/src/lib.rs`** — Declare `pub mod pv_table;`, import `PvTable`, extend `SearchContext` with `pv_table` and `prev_pv` fields, add `pv_move_at` helper, update `negamax` (clear_ply, pass pv_move to order_moves, update PV table on alpha improvement), update `quiescence` (pass `None` for pv_move), update `search` (init fields, clear PV table per iteration, extract PV after each iteration), update `test_ctx`, add `pv_ordering_reduces_nodes` test.

## 7. Verification Commands

```bash
# Format check
cargo fmt --check --manifest-path crates/search/Cargo.toml

# Lint check
cargo clippy -p chess-search -- -D warnings

# Run all search crate tests
cargo test -p chess-search

# Run all workspace tests to verify no regressions
cargo test --workspace

# Run specific PV table tests
cargo test -p chess-search pv_table

# Run specific ordering tests
cargo test -p chess-search ordering

# Run the node reduction test specifically
cargo test -p chess-search pv_ordering_reduces_nodes
```

## 8. Design Deviations

**Deviation: PV move not applied in quiescence search**

- **Parent spec states**: "At each node, if the current position's PV move matches an available move, it gets highest ordering priority."
- **Why literal interpretation is problematic**: Quiescence search does not follow the same ply-indexed PV line as the main search. The `prev_pv` vector from iterative deepening tracks the main search PV at specific ply depths. In quiescence, the ply counter continues incrementing beyond the main search depth, but those plies don't correspond to entries in `prev_pv` (which only has `depth` entries from the last iteration). Passing PV moves into quiescence would always yield `None` (since `prev_pv.get(ply)` returns `None` for plies beyond the PV length) and add unnecessary branching.
- **Alternative chosen**: Pass `None` as the `pv_move` in quiescence. This is functionally equivalent (the PV never extends into quiescence plies) but makes the intent explicit and avoids a pointless lookup. This matches standard engine practice — PV move ordering applies only to the main search, not quiescence.