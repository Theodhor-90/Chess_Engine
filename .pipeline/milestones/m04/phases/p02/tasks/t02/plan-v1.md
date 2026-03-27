Now I have a complete picture of the codebase. Let me produce the implementation plan.

## 1. Deliverables

| # | File | Action | Purpose |
|---|------|--------|---------|
| 1 | `crates/search/src/lib.rs` | Modify | Add a `SearchLimits` struct to communicate depth/nodes/time constraints to the search; modify `search()` to accept `SearchLimits` instead of bare `Duration`; enforce depth limit and node limit within the iterative deepening loop and `check_time` respectively; ensure aborted mid-iteration returns best move from last fully completed iteration |
| 2 | `crates/engine/src/main.rs` | Modify | Replace the hardcoded `Duration::from_secs(5)` time budget with a call to `chess_uci::time::allocate_time()` from T01; construct `SearchLimits` from `GoParams` to pass depth/nodes/movetime/infinite/time constraints to `search()` |

## 2. Dependencies

No new crate dependencies are needed. The `chess-uci` crate (already a dependency of `engine`) exposes `time::allocate_time`. The `chess-search` crate has no new external dependencies — `Duration`, `Instant`, `Arc<AtomicBool>` are all from `std`.

## 3. Implementation Details

### 3a. `crates/search/src/lib.rs` — SearchLimits & integration

**New struct: `SearchLimits`**

```rust
pub struct SearchLimits {
    pub time_budget: Duration,
    pub max_depth: Option<u8>,
    pub max_nodes: Option<u64>,
}
```

- `time_budget`: The wall-clock time limit for the search. For `go infinite` this is set to a very large value (e.g., `Duration::from_secs(86400)`). For `go movetime N` this is `Duration::from_millis(N)`. For clock-based play, this is the value from `allocate_time()`.
- `max_depth`: If `Some(d)`, iterative deepening stops after completing depth `d`. For `go depth N`.
- `max_nodes`: If `Some(n)`, search aborts once `ctx.nodes >= n` (checked every 1024 nodes in `check_time`). For `go nodes N`.

**Modify `SearchContext`**: Replace `time_budget: Duration` field with `limits: SearchLimits` (or keep `time_budget` and add `max_nodes`). The simplest approach: store `max_nodes: Option<u64>` alongside `time_budget` in `SearchContext`.

**Modify `SearchContext::check_time`**: In addition to the existing elapsed-time check and stop-flag check, add a node limit check:
```rust
if let Some(max) = self.max_nodes {
    if self.nodes >= max {
        self.aborted = true;
    }
}
```

**Modify `search()` signature**:
```rust
pub fn search(
    pos: &mut Position,
    limits: SearchLimits,
    stop_flag: Option<Arc<AtomicBool>>,
    on_depth: Option<DepthCallback<'_>>,
) -> Option<Move>
```

Inside `search()`:
- Store `limits.time_budget` and `limits.max_nodes` into `SearchContext`.
- In the iterative deepening loop, after a completed iteration, check `limits.max_depth`: if `Some(d)` and `depth >= d`, break.
- When `ctx.aborted` is `true` after `negamax` returns (mid-iteration abort), break without updating `best_move` — the best move from the last fully completed iteration is preserved (this is already the current behavior since the code breaks before updating `best_move` on abort).

**Existing tests**: The existing `test_ctx()` helper and direct `negamax` calls in tests don't use `SearchLimits` — they construct `SearchContext` directly. They remain unchanged. Tests that call `search()` (like `iterative_deepening_returns_legal_move`, `search_respects_time_budget`, etc.) need their call sites updated to pass `SearchLimits` instead of a bare `Duration`.

### 3b. `crates/engine/src/main.rs` — Go handler update

In the `Go(params)` match arm, replace the current time budget computation with:

```rust
let side = state.position.side_to_move();
let limits = chess_search::SearchLimits {
    time_budget: if let Some(mt) = params.movetime {
        Duration::from_millis(mt)
    } else if params.infinite || params.ponder {
        Duration::from_secs(86400)
    } else if params.wtime.is_some() || params.btime.is_some() {
        chess_uci::time::allocate_time(&params, side)
    } else {
        Duration::from_secs(86400)
    },
    max_depth: params.depth,
    max_nodes: params.nodes,
};
```

Then pass `limits` to `chess_search::search()` instead of the bare `time_budget`.

When `params.depth` is set, the time budget is set to a large value (86400s) so the search is depth-limited, not time-limited. Similarly for `params.nodes` (nodes limit will trigger abort via `check_time`).

## 4. API Contracts

### `SearchLimits` (new public struct in `chess_search`)

```rust
pub struct SearchLimits {
    pub time_budget: Duration,
    pub max_depth: Option<u8>,
    pub max_nodes: Option<u64>,
}
```

### Updated `search()` signature

```rust
pub fn search(
    pos: &mut Position,
    limits: SearchLimits,
    stop_flag: Option<Arc<AtomicBool>>,
    on_depth: Option<DepthCallback<'_>>,
) -> Option<Move>
```

**Example usage from engine main loop**:

```rust
let limits = chess_search::SearchLimits {
    time_budget: chess_uci::time::allocate_time(&params, side),
    max_depth: params.depth,   // Some(5) for "go depth 5", None otherwise
    max_nodes: params.nodes,   // Some(100000) for "go nodes 100000", None otherwise
};
let result = chess_search::search(&mut pos, limits, Some(stop), Some(&callback));
```

## 5. Test Plan

### Test 1: Search aborts when time budget expires (existing test, updated call site)

- **Location**: `crates/search/src/lib.rs` — `search_respects_time_budget`
- **Setup**: Startpos, `SearchLimits { time_budget: 50ms, max_depth: None, max_nodes: None }`
- **Assert**: Elapsed time < 200ms, returns `Some(move)`

### Test 2: Search returns valid move when aborted (existing test, updated call site)

- **Location**: `crates/search/src/lib.rs` — `test_stop_flag_aborts_search`
- **Setup**: Startpos, stop flag set after 50ms, time budget 60s
- **Assert**: Returns within 500ms, returns `Some(move)`

### Test 3: Depth limit stops search at exact depth (new test)

- **Location**: `crates/search/src/lib.rs`
- **Setup**: Startpos, `SearchLimits { time_budget: 86400s, max_depth: Some(3), max_nodes: None }`. Use `on_depth` callback to record the maximum depth reached.
- **Assert**: Maximum reported depth == 3, returns `Some(move)`

### Test 4: Node limit aborts search (new test)

- **Location**: `crates/search/src/lib.rs`
- **Setup**: Startpos, `SearchLimits { time_budget: 86400s, max_depth: None, max_nodes: Some(500) }`. The search should abort once nodes >= 500.
- **Assert**: Returns `Some(move)` (at least depth 1 completes within 500 nodes from startpos)

### Test 5: `allocate_time` is used instead of hardcoded 5s (verified by existing T01 unit tests + the integration in main.rs)

- **Verification**: Code review — the `Duration::from_secs(5)` line is removed and replaced by `chess_uci::time::allocate_time(&params, side)`.

### Test 6: All existing search tests pass with updated `search()` call sites

- **Verification**: `cargo test -p chess-search`

## 6. Implementation Order

1. **Add `SearchLimits` struct** to `crates/search/src/lib.rs` — define the struct with `time_budget`, `max_depth`, `max_nodes` fields.

2. **Modify `SearchContext`** — add `max_nodes: Option<u64>` field.

3. **Modify `SearchContext::check_time`** — add node limit check alongside existing time and stop-flag checks.

4. **Modify `search()` function** — change signature to accept `SearchLimits` instead of `Duration`; populate `SearchContext` from `SearchLimits`; add depth limit check in iterative deepening loop; add depth-limit break for `max_depth` after completed iterations.

5. **Update existing tests** in `crates/search/src/lib.rs` — change all `search()` call sites to pass `SearchLimits` instead of bare `Duration`.

6. **Add new tests** — `depth_limit_stops_at_exact_depth` and `node_limit_aborts_search`.

7. **Modify `crates/engine/src/main.rs`** — replace hardcoded time budget logic in the `Go` handler with `allocate_time()` call and `SearchLimits` construction; pass `limits` to `chess_search::search()`.

8. **Run verification commands** to confirm everything compiles, passes clippy, and passes tests.

## 7. Verification Commands

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test -p chess-search
cargo test -p chess-uci
cargo test -p engine
cargo test
cargo build --release
```

## 8. Design Deviations

**Deviation: Introducing `SearchLimits` struct instead of adding parameters to `search()` directly**

- **What the task spec says**: "Modify the search to use the time allocation from T01 as its time budget" and "ensure `SearchContext::check_time` uses the passed budget to abort when time is exceeded." The task spec lists only two files to modify (`crates/search/src/lib.rs` and `crates/engine/src/main.rs`) and describes passing the computed `Duration` budget to `search()`.
- **Why a bare `Duration` is insufficient**: The current `search()` signature already takes a `Duration`. However, to support the depth and node limits required by the sibling task T03 (which depends on T02's search integration), and because the task spec itself says "ensure iterative deepening returns the best move from the last fully completed iteration," the search needs to know about depth and node limits — not just time. Adding these as separate parameters (`max_depth: Option<u8>, max_nodes: Option<u64>`) alongside `time_budget` would clutter the signature. A `SearchLimits` struct groups them cleanly.
- **Alternative chosen**: A simple `SearchLimits` struct with three fields (`time_budget`, `max_depth`, `max_nodes`) that replaces the `Duration` parameter in `search()`. This is the minimal change needed to support the time budget integration (this task) while providing the hooks T03 will use for `go depth`, `go nodes`, etc. The struct is not speculative — the `Go` handler in `main.rs` already needs to pass depth/nodes limits (currently parsed in `GoParams` but ignored), and this task's own requirement to "replace the hardcoded 5-second fallback" with proper time allocation demands that the engine construct proper search limits for all `go` variants.