Now I have a complete picture of the current codebase state. Let me produce the implementation plan.

## 1. Deliverables

| File | Purpose |
|------|---------|
| `crates/search/src/lib.rs` | Modified — add `SearchContext` struct (holds search state: start time, time budget, node counter, abort flag), modify `negamax` and `quiescence` to accept `&mut SearchContext` for node counting and time-abort checking, add iterative deepening loop, add top-level `pub fn search(pos: &mut Position, time_budget: Duration) -> Option<Move>` entry point, add tests |

No new files are created. The `search` crate already exists from T01/T02. This task modifies `lib.rs` to add iterative deepening with time management and wire it into the existing negamax/quiescence functions.

## 2. Dependencies

No new crate dependencies are needed. The `search` crate already depends on `chess-types`, `chess-board`, `chess-movegen`, and `chess-eval`. Time management uses `std::time::{Duration, Instant}` from the standard library.

## 3. Implementation Details

### SearchContext struct

```rust
struct SearchContext {
    start: Instant,
    time_budget: Duration,
    nodes: u64,
    aborted: bool,
}
```

- **Purpose**: Holds mutable search state threaded through negamax and quiescence calls.
- **Fields**:
  - `start`: `Instant` recorded at search start via `Instant::now()`
  - `time_budget`: `Duration` — the fixed time budget for the entire search
  - `nodes`: `u64` — incremented once per node visited (in both negamax and quiescence)
  - `aborted`: `bool` — set to `true` when time is exhausted; once set, all recursive calls return immediately

**Method**:
- `fn check_time(&mut self)`: Called every 1024 nodes (when `self.nodes & 1023 == 0`). Sets `self.aborted = true` if `self.start.elapsed() >= self.time_budget`.

### Modified `negamax` signature

```rust
pub fn negamax(
    pos: &mut Position,
    depth: u8,
    mut alpha: i32,
    beta: i32,
    ply: u8,
    ctx: &mut SearchContext,
) -> (i32, Option<Move>)
```

Changes from existing implementation:
- Add `ctx: &mut SearchContext` parameter
- Increment `ctx.nodes` at function entry
- Call `ctx.check_time()` when `ctx.nodes & 1023 == 0`
- If `ctx.aborted` is `true` at entry or after time check, return `(0, None)` immediately (the score is irrelevant since the result will be discarded)
- Pass `ctx` through to recursive `negamax` calls and the `quiescence` call at depth 0

### Modified `quiescence` signature

```rust
pub fn quiescence(
    pos: &mut Position,
    mut alpha: i32,
    beta: i32,
    ply: u8,
    ctx: &mut SearchContext,
) -> i32
```

Changes from existing implementation:
- Add `ctx: &mut SearchContext` parameter
- Increment `ctx.nodes` at function entry
- Call `ctx.check_time()` when `ctx.nodes & 1023 == 0`
- If `ctx.aborted`, return `0` immediately
- Pass `ctx` through to recursive `quiescence` calls

### Iterative deepening loop (`search` function)

```rust
pub fn search(pos: &mut Position, time_budget: Duration) -> Option<Move> {
    let mut ctx = SearchContext {
        start: Instant::now(),
        time_budget,
        nodes: 0,
        aborted: false,
    };

    let mut best_move: Option<Move> = None;
    let mut depth: u8 = 1;

    loop {
        ctx.aborted = false;
        let (score, mv) = negamax(pos, depth, -INFINITY, INFINITY, 0, &mut ctx);

        if ctx.aborted {
            // Incomplete iteration — discard result, return best from previous completed depth
            break;
        }

        // Completed this depth
        if mv.is_some() {
            best_move = mv;
        }

        // Stop if mate found (no need to search deeper)
        if score.abs() >= MATE_SCORE - 100 {
            break;
        }

        // Prevent overflow of u8 depth (though in practice time will run out first)
        if depth == u8::MAX {
            break;
        }

        depth += 1;
    }

    best_move
}
```

Key logic:
- Start at depth 1, increment by 1 each iteration
- Before each iteration, reset `ctx.aborted` to `false` (but do NOT reset `ctx.nodes` — the node counter is cumulative and the time-check interval logic `nodes & 1023 == 0` works regardless)
- After each `negamax` call, check `ctx.aborted`:
  - If `true`: the iteration was incomplete — discard results, break, return `best_move` from the last fully completed depth
  - If `false`: the iteration completed — update `best_move` if a move was returned
- If a mate score is found (absolute value within `MATE_SCORE - 100`), stop iterating (no point searching deeper)
- Return `None` only if no legal moves exist (checkmate or stalemate in the root position)

### `king_square` helper

The existing `king_square` helper function remains unchanged — it's a private utility already used by `negamax`.

## 4. API Contracts

### Top-level entry point

```rust
/// Search the position for the best move within the given time budget.
///
/// Returns `Some(best_move)` if legal moves exist, `None` for checkmate/stalemate.
pub fn search(pos: &mut Position, time_budget: Duration) -> Option<Move>
```

- **Input**: A mutable reference to a `Position` (position is restored to its original state after search), and a `Duration` representing the time budget.
- **Output**: `Option<Move>` — the best move found by the deepest fully completed iteration, or `None` if the position has no legal moves.

**Example usage**:
```rust
use std::time::Duration;
use chess_board::Position;
use chess_search::search;

let mut pos = Position::startpos();
let best = search(&mut pos, Duration::from_secs(5));
assert!(best.is_some()); // startpos has legal moves
```

### Modified public functions

Both `negamax` and `quiescence` gain a `ctx: &mut SearchContext` parameter. Since `SearchContext` is a private struct, these functions' `ctx` parameter is effectively internal. However, since they are currently `pub`, they retain their public visibility with the new parameter. Tests construct `SearchContext` instances directly (within the same module).

## 5. Test Plan

### Test setup

All tests are colocated in `#[cfg(test)] mod tests` within `crates/search/src/lib.rs`. Tests use `Position::from_fen()` and `Position::startpos()` to create positions. Helper function to create a non-aborting context for unit tests of `negamax`/`quiescence`:

```rust
fn test_ctx() -> SearchContext {
    SearchContext {
        start: Instant::now(),
        time_budget: Duration::from_secs(60), // generous budget for tests
        nodes: 0,
        aborted: false,
    }
}
```

### Existing tests — update signatures

All 11 existing tests must be updated to pass `&mut SearchContext` to `negamax` and `quiescence`. They use `test_ctx()` with a generous time budget so behavior is unchanged. No logic changes to existing test assertions.

### New tests

1. **`iterative_deepening_returns_legal_move`**: Call `search(&mut pos, Duration::from_secs(5))` on startpos. Assert `Some(move)` is returned. Assert the returned move is in the legal move list.

2. **`iterative_deepening_finds_better_moves_at_depth`**: Use a tactical position (e.g., `"r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4"` — Scholar's Mate Qxf7# available). Call `search` with generous time. Assert the returned move delivers mate (or is the known best move Qxf7#).

3. **`search_respects_time_budget`**: Call `search(&mut pos, Duration::from_millis(50))` on startpos. Measure elapsed time. Assert elapsed time is less than 200ms (allows some overhead beyond the 50ms budget). Assert a legal move is still returned.

4. **`search_returns_none_for_checkmate`**: Use a checkmated position (e.g., `"rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3"`). Call `search`. Assert `None` is returned.

5. **`search_returns_none_for_stalemate`**: Use a stalemate position (e.g., `"k7/1R6/K7/8/8/8/8/8 b - - 0 1"`). Call `search`. Assert `None` is returned.

6. **`node_counter_increments`**: Create a `SearchContext`, call `negamax` at depth 2 on startpos, assert `ctx.nodes > 0`.

## 6. Implementation Order

1. **Add `use` imports**: Add `use std::time::{Duration, Instant};` at the top of `lib.rs`.

2. **Add `SearchContext` struct**: Define the struct and its `check_time` method.

3. **Modify `quiescence` function**: Add `ctx: &mut SearchContext` parameter, add node counting, time-check, and early abort logic. Pass `ctx` to recursive calls.

4. **Modify `negamax` function**: Add `ctx: &mut SearchContext` parameter, add node counting, time-check, and early abort logic. Pass `ctx` to recursive `negamax` and `quiescence` calls.

5. **Add `search` function**: Implement the iterative deepening loop with time management as described.

6. **Update existing tests**: Add `test_ctx()` helper, update all 11 existing tests to pass `&mut ctx` to `negamax`/`quiescence`.

7. **Add new tests**: Add the 6 new tests specified in the test plan.

8. **Run verification commands**.

## 7. Verification Commands

```bash
cargo fmt --check --manifest-path crates/search/Cargo.toml
cargo clippy -p chess-search -- -D warnings
cargo test -p chess-search
cargo test --workspace
```

## 8. Design Deviations

**Deviation 1: `SearchContext` struct instead of separate parameters**

- **Parent spec requires**: The task spec says "node counter with periodic time checks" and "accept a fixed time budget as `std::time::Duration`" but does not prescribe how to thread this state through recursive calls.
- **Why a struct**: Threading `start`, `time_budget`, `nodes`, and `aborted` as separate parameters to `negamax` and `quiescence` would make signatures unwieldy (8+ parameters). Grouping them into a `SearchContext` struct is the standard Rust pattern for threading mutable search state.
- **Alternative chosen**: A private `SearchContext` struct passed as `&mut SearchContext` to `negamax` and `quiescence`. This keeps the public `search()` entry point clean with only `(pos, time_budget)` as specified.

**Deviation 2: Not resetting node counter between iterations**

- **Parent spec requires**: "Node counter accurately tracks nodes visited."
- **Why cumulative**: The node counter serves dual purpose — time-check interval (every 1024 nodes) and total node tracking. Resetting it between iterations would not affect the `& 1023` check correctness and would lose the total count. Keeping it cumulative is standard practice in chess engines and more useful for reporting.
- **Alternative chosen**: Cumulative node counter across all iterations. The `nodes` field reflects total nodes searched across all depths.