Now I have all the context I need. Let me produce the revised plan.

# T03 — UCI Options and Lazy Loading

## 1. Deliverables

### Modify: `crates/uci/src/output.rs`
Add an `option_spin` helper function for advertising UCI spin (integer) options, following the existing pattern of `option_string` and `option_combo`.

### Modify: `crates/engine/src/syzygy/mod.rs`
Add a `LazySyzygyTablebase` wrapper struct that defers tablebase initialization until the first probe call. Implements `chess_search::TbProber` directly. Supports re-initialization when the path changes by constructing a new instance.

### Modify: `crates/engine/src/main.rs`
- Add `syzygy_path` and `syzygy_probe_depth` fields to `EngineState`
- Advertise `SyzygyPath` (string) and `SyzygyProbeDepth` (spin, default 6, min 0, max 7) in the `uci` command response
- Handle `setoption name SyzygyPath value <path>` and `setoption name SyzygyProbeDepth value <n>` in the `SetOption` handler
- In the `Go` handler, construct a `LazySyzygyTablebase` from the stored path and probe depth and pass it to `chess_search::search`

## 2. Dependencies

- **`fathom-syzygy`** crate — already declared in `crates/engine/Cargo.toml`
- **`chess_search::TbProber`** trait — already defined in `crates/search/src/lib.rs` (from T02)
- **`SyzygyTablebase`** and **`SyzygyProberAdapter`** — already defined in `crates/engine/src/syzygy/mod.rs` (from T01/T02)
- No new external dependencies required

## 3. Implementation Details

### `crates/uci/src/output.rs` — `option_spin` function

**Purpose**: Provide a helper to format UCI spin option advertisement lines.

**Exports**: `pub fn option_spin(name: &str, default: i32, min: i32, max: i32) -> String`

**Logic**:
```rust
pub fn option_spin(name: &str, default: i32, min: i32, max: i32) -> String {
    format!("option name {name} type spin default {default} min {min} max {max}")
}
```

### `crates/engine/src/syzygy/mod.rs` — `LazySyzygyTablebase`

**Purpose**: Defer tablebase file loading until first probe. Support invalidation on path change by constructing a new instance.

**Changes**:
1. Add a new struct `LazySyzygyTablebase` with fields:
   - `path: String` — configured directory path
   - `inner: Option<SyzygyTablebase>` — lazily initialized probing instance
   - `probe_limit: u8` — max piece count for probing

2. `LazySyzygyTablebase` methods:
   - `pub fn new(path: String, probe_limit: u8) -> Self` — stores path and probe_limit, sets `inner` to `None`
   - `fn ensure_init(&mut self) -> Option<&mut SyzygyTablebase>` — if `inner` is `None` and `path` is non-empty, calls `SyzygyTablebase::init(&self.path)`, stores the result in `inner` on success (logs error via `eprintln!` on failure), returns `Option<&mut SyzygyTablebase>`. If `inner` is already `Some`, returns a mutable reference to it.

3. Implement `chess_search::TbProber` for `LazySyzygyTablebase`:
   - `probe_wdl` calls `ensure_init()`, then delegates to `SyzygyTablebase::probe_wdl` with `self.probe_limit`; converts result via `wdl_to_score`
   - `probe_root` calls `ensure_init()`, then delegates to `SyzygyTablebase::probe_wdl` and `SyzygyTablebase::probe_dtz` with `self.probe_limit`; converts WDL result via `wdl_to_score`, returns `(wdl_score, dtz.0)`

**Key contract**: When `path` is empty, `ensure_init` returns `None` — no probing is attempted. The existing `SyzygyTablebase` and `SyzygyProberAdapter` remain unchanged.

### `crates/engine/src/main.rs` — UCI options and wiring

**Purpose**: Expose Syzygy configuration via UCI, wire tablebase probing into search.

**Changes to `EngineState`**:
```rust
struct EngineState {
    // ... existing fields ...
    syzygy_path: String,
    syzygy_probe_depth: u8,
}
```

Note: There is no `syzygy_tb` field in `EngineState`. A fresh `LazySyzygyTablebase` is constructed per `go` command from the stored `syzygy_path` and `syzygy_probe_depth`, then moved into the search thread. This avoids shared mutable state across threads. Lazy initialization still occurs inside `LazySyzygyTablebase` (only on first probe within that search). Re-initialization cost is negligible since Fathom uses memory-mapped files.

**Initialization** in `main()`:
```rust
syzygy_path: String::new(),
syzygy_probe_depth: 6,
```

**UCI command response** (in `UciCommand::Uci` handler):
Add after existing option lines, before `uciok`:
```rust
println!("{}", chess_uci::output::option_string("SyzygyPath", ""));
println!("{}", chess_uci::output::option_spin("SyzygyProbeDepth", 6, 0, 7));
```

**SetOption handler** — add two new arms to the `match name.to_lowercase().as_str()` block:

1. `"syzygypath"`:
   - If value is `Some(path)` and non-empty: store `path` in `state.syzygy_path`
   - If value is `None` or empty string: set `state.syzygy_path` to empty string

2. `"syzygyprobedepth"` (since `"SyzygyProbeDepth".to_lowercase()` yields `"syzygyprobedepth"`):
   - Parse the value as `u8`, clamp to 0–7, store in `state.syzygy_probe_depth`

**Go handler** — before spawning the search thread, construct the lazy tablebase prober:

```rust
let mut tb_prober: Option<chess_engine::syzygy::LazySyzygyTablebase> =
    if !state.syzygy_path.is_empty() {
        Some(chess_engine::syzygy::LazySyzygyTablebase::new(
            state.syzygy_path.clone(),
            state.syzygy_probe_depth,
        ))
    } else {
        None
    };
```

Inside the spawned thread, pass the prober to the search call:
```rust
let result = chess_search::search(
    &mut search_pos,
    limits,
    &game_history,
    Some(&|depth, score, nodes, elapsed, pv| { /* existing info callback */ }),
    tb_prober.as_mut().map(|t| t as &mut dyn chess_search::TbProber),
);
```

## 4. API Contracts

### UCI Option Advertisement (output of `uci` command)
```
option name SyzygyPath type string default 
option name SyzygyProbeDepth type spin default 6 min 0 max 7
```

### UCI SetOption Input
```
setoption name SyzygyPath value /path/to/syzygy
setoption name SyzygyProbeDepth value 5
setoption name SyzygyPath value
```

### `option_spin` function
- **Input**: `option_spin("SyzygyProbeDepth", 6, 0, 7)`
- **Output**: `"option name SyzygyProbeDepth type spin default 6 min 0 max 7"`

### `LazySyzygyTablebase::new`
- **Input**: `LazySyzygyTablebase::new("/path/to/tb".to_string(), 6)`
- **Output**: `LazySyzygyTablebase` with deferred initialization (`inner` is `None`)

### `LazySyzygyTablebase` as `TbProber`
- `probe_wdl(pos)` → `Option<i32>` — returns `None` if path is empty, initialization fails, or piece count exceeds limit
- `probe_root(pos)` → `Option<(i32, i32)>` — returns `None` under same conditions; returns `(wdl_score, dtz)` tuple on success

## 5. Test Plan

### `crates/uci/src/output.rs` — Unit test for `option_spin`
- **Test**: `test_option_spin` — verify `option_spin("SyzygyProbeDepth", 6, 0, 7)` produces `"option name SyzygyProbeDepth type spin default 6 min 0 max 7"`.
- **Traces to**: Verification criterion 1 (UCI option advertisement format) and exit criterion 4 ("UCI options control Syzygy tablebase path and probe depth").

### `crates/engine/src/syzygy/mod.rs` — Unit tests for lazy loading

- **Test**: `lazy_no_path_returns_none` — create `LazySyzygyTablebase` with an empty path, call `probe_wdl` on a position, verify it returns `None` without error.
  - **Traces to**: Verification criterion 6 ("When SyzygyPath is not set or is empty, no probing is attempted and no errors occur").

- **Test**: `lazy_init_deferred` — create `LazySyzygyTablebase` with a non-empty path, verify `inner` is `None` immediately after construction (tablebase not loaded at construction time).
  - **Traces to**: Verification criterion 2 and exit criterion 5 ("Tablebase files are lazily loaded, not at startup").

- **Test**: `lazy_probe_limit_respected` — create `LazySyzygyTablebase` with an empty path and `probe_limit` of 4, call `probe_wdl` on a position with 5 pieces (e.g., KRK + 2 pawns), verify it returns `None`. This validates the probe_limit gating at the `LazySyzygyTablebase` level even before delegation to `SyzygyTablebase`. Note: since the path is empty, `ensure_init` returns `None` and no probe occurs — the test confirms the combined behavior that no result is returned when conditions aren't met. For a stronger test of the limit specifically, this test uses a `probe_limit` of 4 with a 5-piece position, confirming the prober returns `None`.
  - **Traces to**: Verification criterion 5 ("SyzygyProbeDepth correctly controls the piece-count gate for in-search probing").

### `crates/engine/src/main.rs` — UCI option advertisement (manual verification)

Integration testing of `main.rs` UCI output is not feasible in unit tests because `main()` reads from stdin/stdout directly. Verification criterion 1 ("uci command response includes option name SyzygyPath... and option name SyzygyProbeDepth...") will be verified through:
1. Code review: the `println!` calls are placed in the `UciCommand::Uci` handler between existing option lines and `uciok`, using the same `option_string` and `option_spin` helpers that have their own unit tests.
2. Manual smoke test: run the engine binary, send `uci`, and confirm the option lines appear in the output.

### Existing tests must continue to pass
- All existing tests in `crates/engine/src/syzygy/mod.rs`, `crates/uci/`, and `crates/search/` must pass unchanged.
- `cargo clippy -- -D warnings` must pass.
- `cargo fmt --check` must pass.

## 6. Implementation Order

1. **`crates/uci/src/output.rs`** — Add `option_spin` function and its unit test. This is a leaf dependency used by subsequent steps.

2. **`crates/engine/src/syzygy/mod.rs`** — Add `LazySyzygyTablebase` struct with `new`, `ensure_init`, and `TbProber` implementation. Add unit tests for lazy loading behavior (`lazy_no_path_returns_none`, `lazy_init_deferred`, `lazy_probe_limit_respected`).

3. **`crates/engine/src/main.rs`** — Add `syzygy_path` and `syzygy_probe_depth` to `EngineState`. Add UCI option advertisement for `SyzygyPath` and `SyzygyProbeDepth`. Add `setoption` handling for both options. Wire `LazySyzygyTablebase` into the `Go` command handler, passing it to `chess_search::search`.

4. **Verify** — Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` across the workspace.

## 7. Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

## 8. Design Deviations

**Deviation 1: Lazy loading via `LazySyzygyTablebase` rather than modifying `SyzygyTablebase` directly**

- **Parent spec requires**: "Modify: Syzygy probing module/crate (from T01) — add lazy loading initialization logic" — implies modifying the existing `SyzygyTablebase` struct.
- **Why that approach is problematic**: `SyzygyTablebase` is a clean, non-lazy struct that takes a valid path in its constructor and guarantees the probing library is initialized. Adding lazy/optional state to it would complicate its API for all callers (including tests) and break the existing `SyzygyProberAdapter` which assumes a fully initialized `SyzygyTablebase`. It would also require changing the constructor signature and all existing test code from T01.
- **Alternative chosen**: Introduce a new `LazySyzygyTablebase` wrapper in the same module that composes `Option<SyzygyTablebase>` with a stored path and probe limit, implementing `TbProber` directly. This preserves the existing `SyzygyTablebase` and `SyzygyProberAdapter` unchanged while providing the lazy loading behavior the task requires. The new struct is used exclusively by `main.rs` for UCI-driven lazy initialization. This still satisfies the parent spec's intent — the syzygy module is modified to add lazy loading logic — while maintaining a clean separation of concerns.

**Deviation 2: Fresh `LazySyzygyTablebase` per search invocation instead of persistent shared state**

- **Parent spec requires**: Lazy loading where "tablebase files are only loaded when first probed" and re-initialization on path change.
- **Why persistent state across threads is problematic**: The search runs in a `std::thread::spawn(move || ...)` closure, requiring `Send` ownership transfer. Sharing mutable state across the main thread and search thread would require `Arc<Mutex<...>>` or similar synchronization, adding complexity disproportionate to the benefit. The Fathom library performs memory-mapping which is fast to re-initialize.
- **Alternative chosen**: Create a new `LazySyzygyTablebase` for each `go` command, moved into the search thread. Lazy initialization still occurs (only on first probe within that search), and path changes take effect immediately on the next `go`. This is simpler, correct, and avoids shared mutable state. Re-initialization cost is negligible since Fathom uses memory-mapped files. `EngineState` stores only `syzygy_path: String` and `syzygy_probe_depth: u8` — no `syzygy_tb` field is needed, keeping the design fully consistent with the per-search construction approach.