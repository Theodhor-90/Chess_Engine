Now I have a thorough understanding of the codebase. Let me also check the `search` function signature closely to understand what parameters it accepts, and understand the current `SearchContext` fields.

Now I have everything I need. Let me produce the implementation plan.

## 1. Deliverables

| # | File | Action | Purpose |
|---|------|--------|---------|
| 1 | `crates/uci/src/lib.rs` | Modify | Add `PonderHit` variant to the `UciCommand` enum; add `"ponderhit"` to the parser's match arm |
| 2 | `crates/search/src/lib.rs` | Modify | Add `SearchLimits` struct with `max_depth`, `max_nodes`, and `max_time` fields; modify `search()` to accept `SearchLimits` and enforce `depth`, `nodes`, and `movetime` limits; modify `SearchContext` and `check_time()` to enforce `max_nodes` |
| 3 | `crates/engine/src/main.rs` | Modify | Update the `Go` handler to detect which `go` variant is active and construct appropriate `SearchLimits`; add `PonderHit` command handling that switches a pondering search to time-managed mode; handle `stop` during ponder to emit `bestmove` |

## 2. Dependencies

No new external crates or packages are required. All changes use existing dependencies:

- `crates/search` already depends on `std::time::{Duration, Instant}` and `std::sync::atomic`
- `crates/engine` already depends on `chess-uci`, `chess-search`, `chess-board`, `chess-types`, and `anyhow`
- `crates/uci` has no new dependencies

## 3. Implementation Details

### 3a. `crates/uci/src/lib.rs` — Add `PonderHit` command

**Purpose**: The UCI protocol defines `ponderhit` as a standalone command. The parser must recognize it.

**Changes**:
- Add `PonderHit` to the `UciCommand` enum.
- Add `"ponderhit" => Ok(UciCommand::PonderHit)` to the `match` in `pub fn parse(...)`.
- Add a unit test `parse_ponderhit` verifying `parse("ponderhit")` returns `UciCommand::PonderHit`.

### 3b. `crates/search/src/lib.rs` — Add `SearchLimits` and enforce them

**Purpose**: The search needs to support depth, node, and time limits beyond the existing time budget. A `SearchLimits` struct parameterises these.

**New type — `SearchLimits`**:
```rust
#[derive(Debug, Clone)]
pub struct SearchLimits {
    pub max_time: Duration,
    pub max_depth: Option<u8>,
    pub max_nodes: Option<u64>,
    pub stop_flag: Option<Arc<AtomicBool>>,
}
```

**Changes to `SearchContext`**:
- Add a `max_nodes: Option<u64>` field.
- In `check_time()`, also check `self.max_nodes`: if `self.nodes >= max_nodes`, set `self.aborted = true`.

**Changes to `search()`**:
- Change the signature from `search(pos, time_budget, stop_flag, on_depth)` to `search(pos, limits: SearchLimits, on_depth)`. The `time_budget` and `stop_flag` are now part of `SearchLimits`.
- Initialize `SearchContext` from `limits.max_time`, `limits.max_nodes`, and `limits.stop_flag`.
- In the iterative deepening loop, after incrementing `depth`, check if `limits.max_depth` is `Some(d)` and `depth > d`; if so, break. This ensures `go depth N` searches to exactly depth N (all depths 1..=N are completed, then the loop exits).

**Backward-compatible note**: The previous `search()` signature is replaced. All callers (only `crates/engine/src/main.rs`) must be updated simultaneously (see 3c).

### 3c. `crates/engine/src/main.rs` — `Go` variant routing and `PonderHit`

**Purpose**: The `Go` handler must detect which variant is active and construct the correct `SearchLimits`. The `ponderhit` command must switch a pondering search to time-managed mode.

**Changes to `EngineState`**:
- Add a `pondering: bool` field (default `false`), to track whether the current search was started with `go ponder`.
- Add a `ponder_params: Option<GoParams>` field, to store the `GoParams` from the `go ponder` command for use on `ponderhit`.

**Changes to the `Go` match arm**:
The handler inspects `params` to determine which variant is active. The priority:

1. `params.ponder == true` → set `limits.max_time = Duration::from_secs(86400)` (no time limit, search until `stop` or `ponderhit`), `max_depth = None`, `max_nodes = None`. Set `state.pondering = true` and `state.ponder_params = Some(params)`.
2. `params.depth.is_some()` → set `limits.max_depth = params.depth`, `limits.max_time = Duration::from_secs(86400)`, `max_nodes = None`.
3. `params.nodes.is_some()` → set `limits.max_nodes = params.nodes`, `limits.max_time = Duration::from_secs(86400)`, `max_depth = None`.
4. `params.movetime.is_some()` → set `limits.max_time = Duration::from_millis(params.movetime.unwrap())`, `max_depth = None`, `max_nodes = None`.
5. `params.infinite == true` → set `limits.max_time = Duration::from_secs(86400)`, `max_depth = None`, `max_nodes = None`.
6. `params.wtime.is_some() || params.btime.is_some()` → compute time budget via `chess_uci::time::allocate_time(&params, side)`, set as `limits.max_time`.
7. Default → `Duration::from_secs(86400)` (same as current behavior).

**New `PonderHit` match arm**:
When `PonderHit` is received:
- If `state.pondering` is `true`:
  - Compute the time budget from `state.ponder_params` using `chess_uci::time::allocate_time()`.
  - The search is already running. To apply a time limit, the simplest correct approach is: do nothing to the running search immediately, but note that `ponderhit` effectively means "the opponent played the expected move." The standard approach is to set a deadline by which the search must stop. Since the search thread is already running and checking `stop_flag`, we can spawn a timer thread that sets `stop_flag` after the computed budget elapses from the moment `ponderhit` is received.
  - Set `state.pondering = false`.

**Implementation of `ponderhit` timer approach**:
When `ponderhit` is received, spawn a thread:
```rust
let stop = Arc::clone(&state.stop_flag);
let budget = chess_uci::time::allocate_time(&params, side);
std::thread::spawn(move || {
    std::thread::sleep(budget);
    stop.store(true, Ordering::Relaxed);
});
state.pondering = false;
```
This reuses the existing stop flag mechanism and requires no changes to the search internals for ponder support. The search thread, already running, will detect the flag on its next check_time() call and abort.

**`Stop` during ponder**:
The existing `Stop` handler already calls `stop_search()`, which sets the flag and joins the thread. The search thread already emits `bestmove` on completion. No special-casing needed — `stop` during ponder naturally causes the search to abort and emit `bestmove`. Set `state.pondering = false` in the `Stop` handler.

## 4. API Contracts

### `SearchLimits` (new public struct in `crates/search/src/lib.rs`)

```rust
pub struct SearchLimits {
    pub max_time: Duration,       // Maximum time for the search
    pub max_depth: Option<u8>,    // If Some(d), stop after completing depth d
    pub max_nodes: Option<u64>,   // If Some(n), abort once nodes >= n
    pub stop_flag: Option<Arc<AtomicBool>>,  // External stop signal
}
```

### Updated `search()` signature

```rust
pub fn search(
    pos: &mut Position,
    limits: SearchLimits,
    on_depth: Option<DepthCallback<'_>>,
) -> Option<Move>
```

**Input**: A mutable position, search limits, and an optional per-depth callback.
**Output**: `Some(Move)` if a legal move was found, `None` if the position has no legal moves (checkmate/stalemate).

### `UciCommand::PonderHit` (new variant in `crates/uci/src/lib.rs`)

```rust
pub enum UciCommand {
    // ... existing variants ...
    PonderHit,
}
```

## 5. Test Plan

### 5a. `crates/uci/src/lib.rs` — Parser test

| Test | Description |
|------|-------------|
| `parse_ponderhit` | `parse("ponderhit")` returns `UciCommand::PonderHit` |

### 5b. `crates/search/src/lib.rs` — Search limits tests

| Test | Description |
|------|-------------|
| `search_respects_depth_limit` | Call `search()` with `max_depth = Some(3)`. Use the `on_depth` callback to record the maximum depth reached. Assert the maximum depth is exactly 3. |
| `search_respects_node_limit` | Call `search()` with `max_nodes = Some(500)`. After search completes, verify it returns `Some(Move)`. The test trusts the implementation since exact node count is internal. Use a very low node limit on startpos to verify the search terminates quickly (elapsed < 1 second). |
| `search_respects_movetime` | Call `search()` with `max_time = Duration::from_millis(100)`, no depth/node limits. Assert elapsed time is < 500ms (allowing overhead). Assert a move is returned. |

### 5c. Existing tests

The existing tests in `crates/search/src/lib.rs` construct `SearchContext` directly. They must be updated to remain compilable after the `SearchContext` field changes (`max_nodes` added). The existing `test_ctx()` helper and any direct `SearchContext` construction in tests must include the new `max_nodes: None` field.

The existing `search()` call sites in tests (`iterative_deepening_returns_legal_move`, `iterative_deepening_finds_better_moves_at_depth`, `search_respects_time_budget`, `search_returns_none_for_checkmate`, `search_returns_none_for_stalemate`, `test_stop_flag_aborts_search`) must be updated to pass a `SearchLimits` instead of separate `time_budget` and `stop_flag` arguments.

## 6. Implementation Order

1. **`crates/uci/src/lib.rs`** — Add `PonderHit` variant to `UciCommand` and parser. Add `parse_ponderhit` test. This is independent and can be done first.

2. **`crates/search/src/lib.rs`** — Add `SearchLimits` struct. Add `max_nodes` field to `SearchContext`. Update `check_time()` to enforce `max_nodes`. Change `search()` signature to accept `SearchLimits`. Enforce `max_depth` in the iterative deepening loop. Update all existing tests to use the new API. Add three new tests (`search_respects_depth_limit`, `search_respects_node_limit`, `search_respects_movetime`).

3. **`crates/engine/src/main.rs`** — Add `pondering` and `ponder_params` fields to `EngineState`. Rewrite the `Go` handler to construct `SearchLimits` based on the active variant. Add `PonderHit` match arm with timer-based stop. Update `Stop` handler to clear pondering state.

## 7. Verification Commands

```bash
# Format check
cargo fmt --check

# Lint check
cargo clippy -- -D warnings

# Run all tests
cargo test

# Run only search crate tests (includes new limit tests)
cargo test -p chess-search

# Run only uci crate tests (includes ponderhit parse test)
cargo test -p chess-uci

# Run only engine crate tests (if any)
cargo test -p engine
```

## 8. Design Deviations

### Deviation 1: `ponderhit` implementation via timer thread instead of modifying search internals

**Parent spec requires**: "On `ponderhit`, switch to normal time management for the remaining budget."

**Why direct approach is problematic**: The search thread is already running when `ponderhit` arrives. To "switch to normal time management" would require mutating the `SearchContext`'s `time_budget` and `start` fields from the main thread while the search thread owns them. This would require either `Arc<Mutex<...>>` wrapping around the time fields (adding lock contention overhead on every 1024-node check) or `Arc<AtomicU64>` for the deadline (adding complexity). Both approaches require significant refactoring of `SearchContext` internals.

**Alternative chosen**: Spawn a timer thread on `ponderhit` that sleeps for the computed time budget, then sets the existing `stop_flag`. This reuses the already-established atomic stop mechanism, requires zero changes to `SearchContext` internals, and has identical observable behavior: the search stops after the allocated time and emits `bestmove`. The timer thread approach is a common pattern in UCI engines (e.g., used by Stockfish's timer thread).

### Deviation 2: `ponderhit` uses `ponder_params` stored at `go ponder` time

**Parent spec requires**: "On `ponderhit`, switch to normal time management for the remaining budget."

**Why this needs stored params**: When `ponderhit` arrives, the engine needs the original `GoParams` (with `wtime`, `btime`, etc.) to compute the time allocation. The UCI protocol sends these params with the original `go ponder` command, not with `ponderhit`. Therefore, the `GoParams` must be stored in `EngineState` at `go ponder` time and retrieved on `ponderhit`. This is not a deviation from intent, but clarifies an implicit requirement in the spec.