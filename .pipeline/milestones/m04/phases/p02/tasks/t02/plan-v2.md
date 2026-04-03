Now I have full context. Let me produce the revised plan.

## 1. Deliverables

| # | File | Action | Purpose |
|---|------|--------|---------|
| 1 | `crates/engine/src/main.rs` | Modify | Replace the hardcoded `Duration::from_secs(5)` time budget with a call to `chess_uci::time::allocate_time()` when `wtime`/`btime` are present |

## 2. Dependencies

No new crate dependencies are needed. The `chess-uci` crate (already a dependency of `engine`) exposes `time::allocate_time`. The `chess-search` crate requires no changes.

## 3. Implementation Details

### 3a. `crates/engine/src/main.rs` — Go handler update

The only code change is in the `Go(params)` match arm (lines 117–165). The current time budget computation:

```rust
let time_budget = if let Some(mt) = params.movetime {
    Duration::from_millis(mt)
} else if params.infinite || params.depth.is_some() {
    Duration::from_secs(86400)
} else if params.wtime.is_some() || params.btime.is_some() {
    Duration::from_secs(5)   // ← hardcoded, to be replaced
} else {
    Duration::from_secs(86400)
};
```

Is replaced with:

```rust
let time_budget = if let Some(mt) = params.movetime {
    Duration::from_millis(mt)
} else if params.infinite || params.depth.is_some() {
    Duration::from_secs(86400)
} else if params.wtime.is_some() || params.btime.is_some() {
    let side = state.position.side_to_move();
    chess_uci::time::allocate_time(&params, side)
} else {
    Duration::from_secs(86400)
};
```

**What changes**: The `Duration::from_secs(5)` on line 126 is replaced with a call to `chess_uci::time::allocate_time(&params, side)`, which computes a proper per-move budget using the formula `time_left / moves_to_go + increment` (with `moves_to_go` defaulting to 25 when absent).

**What stays the same**: The `movetime`, `infinite`, `depth`, and fallback branches remain unchanged. The `search()` signature remains `search(pos, time_budget, stop_flag, on_depth)` — no struct wrapper needed. The `SearchContext`, `check_time`, and iterative deepening logic in `crates/search/src/lib.rs` remain untouched.

**Side to move**: `state.position.side_to_move()` is read before the position is cloned into `search_pos`. This is correct because the Go command applies to the current position, and `side_to_move()` tells `allocate_time` whether to use `wtime`/`winc` or `btime`/`binc`.

### 3b. No changes to `crates/search/src/lib.rs`

The search crate already has the correct behavior for T02:

- `SearchContext::check_time` (line 33–42) checks elapsed time every 1024 nodes and sets `aborted = true` when the time budget is exceeded.
- The iterative deepening loop in `search()` (lines 193–221) breaks when `ctx.aborted` is true after `negamax` returns, preserving the best move from the last fully completed iteration (the `best_move` variable is only updated after successful, non-aborted iterations on line 203).
- The `search()` function already accepts a `Duration` parameter and the existing mechanism correctly enforces it.

No changes to the search crate are needed for T02. The `SearchLimits` struct with `max_depth` and `max_nodes` fields, and any signature changes, belong to T03 which handles `go depth`, `go nodes`, and `go ponder` variants.

## 4. API Contracts

No API changes. The existing public interface is preserved:

### `chess_search::search()` (unchanged)

```rust
pub fn search(
    pos: &mut Position,
    time_budget: Duration,
    stop_flag: Option<Arc<AtomicBool>>,
    on_depth: Option<DepthCallback<'_>>,
) -> Option<Move>
```

### `chess_uci::time::allocate_time()` (from T01, consumed by T02)

```rust
pub fn allocate_time(params: &GoParams, side: Color) -> Duration
```

**Usage in engine main loop**:

```rust
let side = state.position.side_to_move();
let time_budget = chess_uci::time::allocate_time(&params, side);
```

## 5. Test Plan

### Test 1: Search aborts when time budget expires (existing test, unchanged)

- **Location**: `crates/search/src/lib.rs` — `search_respects_time_budget` (line 371)
- **Setup**: Startpos, `Duration::from_millis(50)` time budget
- **Assert**: Elapsed time < 200ms, returns `Some(move)`
- **No changes needed**: This test already validates the check_time mechanism.

### Test 2: Search returns valid move when aborted via stop flag (existing test, unchanged)

- **Location**: `crates/search/src/lib.rs` — `test_stop_flag_aborts_search` (line 457)
- **Setup**: Startpos, stop flag set after 50ms, time budget 60s
- **Assert**: Returns within 500ms, returns `Some(move)`
- **No changes needed**: The stop flag behavior is unaffected by this change.

### Test 3: `allocate_time` returns correct budgets (existing tests from T01, unchanged)

- **Location**: `crates/uci/src/time.rs` — six unit tests covering sudden death, increment, movestogo, very low time, and zero increment scenarios
- **No changes needed**: These tests validate the function T02 now calls.

### Test 4: Hardcoded 5-second fallback is replaced (code review verification)

- **Verification**: Confirm that `Duration::from_secs(5)` no longer appears in `crates/engine/src/main.rs` and is replaced by `chess_uci::time::allocate_time(&params, side)`.

### Test 5: All existing tests continue to pass

- **Verification**: `cargo test` (full workspace). No test call sites change because T02 does not modify the `search()` signature.

## 6. Implementation Order

1. **Modify `crates/engine/src/main.rs`** — In the `Go(params)` match arm, replace `Duration::from_secs(5)` with `let side = state.position.side_to_move(); chess_uci::time::allocate_time(&params, side)`.

2. **Run verification commands** to confirm everything compiles, passes clippy, and passes tests.

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

None. This plan follows the task spec exactly:

- **"Modify the search to use the time allocation from T01 as its time budget"**: The engine main loop now calls `allocate_time()` instead of using a hardcoded `Duration::from_secs(5)`, and passes the result to `search()` as the `time_budget` parameter.
- **"SearchContext::check_time uses the passed budget to abort when time is exceeded"**: Already implemented — no changes needed.
- **"Iterative deepening returns the best move from the last fully completed iteration"**: Already implemented — no changes needed.
- **"Replace the hardcoded Duration::from_secs(5) fallback with the T01 allocation function"**: Done in the single code change described above.
- **Files modified**: Only `crates/engine/src/main.rs`, as listed in the task spec. `crates/search/src/lib.rs` requires no changes because the existing `check_time` and iterative deepening behavior already satisfy T02's requirements.