Now I have a complete picture. The current `main.rs` doesn't use `SyzygyProberAdapter` at all yet — the `None` passed as the last argument to `chess_search::search` confirms this. T02's deliverable says it modified `main.rs` to wire `SyzygyProberAdapter` into the `Go` handler, but the actual code passes `None`. Let me verify once more that the search call passes `None` for the tb_prober.

Looking at the current main.rs line 287: `None,` — this is the `tb_prober` parameter. So T02 didn't actually wire it in (or its spec said "Modify" but the implementation deferred the actual UCI option plumbing to T03). That makes sense — T03 is supposed to add the UCI options that control the path and probe depth, and the wiring depends on having those options set.

Let me also check if there's an `option_spin` helper in the UCI output module — there isn't one. I'll need to add it.

Now I have everything I need. Let me produce the implementation plan.

## 1. Deliverables

### Modify: `crates/uci/src/output.rs`
Add an `option_spin` helper function for advertising UCI spin (integer) options, following the existing pattern of `option_string` and `option_combo`.

### Modify: `crates/engine/src/syzygy/mod.rs`
Add lazy loading support to `SyzygyTablebase`. Wrap the probing state in an `Option` so initialization is deferred until the first probe call. Support re-initialization when the path changes.

### Modify: `crates/engine/src/main.rs`
- Add `syzygy_path` and `syzygy_probe_depth` fields to `EngineState`
- Advertise `SyzygyPath` (string) and `SyzygyProbeDepth` (spin, default 6, min 0, max 7) in the `uci` command response
- Handle `setoption name SyzygyPath value <path>` and `setoption name SyzygyProbeDepth value <n>` in the `SetOption` handler
- When `SyzygyPath` is set/changed, store the path and invalidate any existing `SyzygyTablebase` (so next probe triggers re-initialization)
- In the `Go` handler, construct a `SyzygyProberAdapter` with lazy-initialized tablebase and configured probe limit, passing it to `chess_search::search`

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

### `crates/engine/src/syzygy/mod.rs` — Lazy loading

**Purpose**: Defer tablebase file loading until first probe. Support invalidation on path change.

**Changes**:
1. Add a new struct `LazySyzygyTablebase` with fields:
   - `path: String` — configured directory path
   - `inner: Option<SyzygyTablebase>` — lazily initialized probing instance
   - `probe_limit: u8` — max piece count for probing

2. `LazySyzygyTablebase` methods:
   - `pub fn new(path: String, probe_limit: u8) -> Self` — stores path and probe_limit, sets `inner` to `None`
   - `fn ensure_init(&mut self) -> Option<&mut SyzygyTablebase>` — if `inner` is `None` and `path` is non-empty, calls `SyzygyTablebase::init(&self.path)`, stores the result in `inner` on success (logs error on failure), returns `Option<&mut SyzygyTablebase>`

3. Implement `chess_search::TbProber` for `LazySyzygyTablebase`:
   - `probe_wdl` calls `ensure_init()`, then delegates to `SyzygyTablebase::probe_wdl` with `self.probe_limit`
   - `probe_root` calls `ensure_init()`, then delegates to `SyzygyTablebase::probe_wdl` and `SyzygyTablebase::probe_dtz` with `self.probe_limit`

**Key contract**: When `path` is empty, `ensure_init` returns `None` — no probing is attempted. The existing `SyzygyProberAdapter` remains unchanged (it's still valid code from T02).

### `crates/engine/src/main.rs` — UCI options and wiring

**Purpose**: Expose Syzygy configuration via UCI, implement lazy loading, wire tablebase probing into search.

**Changes to `EngineState`**:
```rust
struct EngineState {
    // ... existing fields ...
    syzygy_path: String,
    syzygy_probe_depth: u8,
    syzygy_tb: Option<LazySyzygyTablebase>,
}
```

**Initialization** in `main()`:
```rust
syzygy_path: String::new(),
syzygy_probe_depth: 6,
syzygy_tb: None,
```

**UCI command response** (in `UciCommand::Uci` handler):
Add after existing option lines:
```rust
println!("{}", chess_uci::output::option_string("SyzygyPath", ""));
println!("{}", chess_uci::output::option_spin("SyzygyProbeDepth", 6, 0, 7));
```

**SetOption handler** — add two new arms to the `match name.to_lowercase().as_str()` block:

1. `"syzygypath"`:
   - If value is `Some(path)` and non-empty: store in `state.syzygy_path`, set `state.syzygy_tb = None` (invalidate to trigger re-init on next probe)
   - If value is `None` or empty string: set `state.syzygy_path` to empty, set `state.syzygy_tb = None`

2. `"syzygyprobeDepth"` (case-insensitive match as `"syzygyprobeDepth"` lowercases to `"syzygyprobeDepth"`) — actually since `to_lowercase()` is used, the match arm is `"syzygyprobeDepth"`. Wait — `"SyzygyProbeDepth".to_lowercase()` = `"syzygyprobeDepth"`. This needs to match correctly. Let me check: `"SyzygyProbeDepth"` → lowercased = `"syzygyprobeDepth"`. The existing code lowercases the name before matching. The match arm should be `"syzygyprobeDepth"`. However, per standard UCI behavior, the name comparison is case-insensitive, so the lowercase form `"syzygyprobeDepth"` is the correct match arm. Actually: `"SyzygyProbeDepth"` in lowercase is `"syzygyprobeDepth"` — but wait, `"D"` lowercases to `"d"`. So the full lowercase is `"syzygyprobeDepth"` → no, let me be precise: `S→s`, `y→y`, `z→z`, `y→y`, `g→g`, `y→y`, `P→p`, `r→r`, `o→o`, `b→b`, `e→e`, `D→d`, `e→e`, `p→p`, `t→t`, `h→h` = `"syzygyprobeDepth"`. Wait: `"SyzygyProbeDepth"` — the capital D is at position... `SyzygyProbeDepth` — lowercase = `syzygyprobeDepth`. Hmm, no: all characters are lowercased: `syzygyprobeDepth` → the `D` becomes `d`: `"syzygyprobeDepth"`. 

Let me just spell it out: `"SyzygyProbeDepth".to_lowercase()` = `"syzygyprobeDepth"`. No — wait. Lowercase converts ALL uppercase to lowercase. So `S→s`, `P→p`, `D→d`. Result: `"syzygyprobeDepth"` → no, `"syzygyprobeDepth"`. Let me just carefully go letter by letter:

`S` `y` `z` `y` `g` `y` `P` `r` `o` `b` `e` `D` `e` `p` `t` `h`

Lowercased: `s` `y` `z` `y` `g` `y` `p` `r` `o` `b` `e` `d` `e` `p` `t` `h`

= `"syzygyprobeDepth"` — no! It's `"syzygyprobeDepth"`. Let me write it without confusion: `s-y-z-y-g-y-p-r-o-b-e-d-e-p-t-h` = `"syzygyprobedepth"`.

So the match arm is `"syzygyprobedepth"`.

   - Parse the value as `u8`, clamp to 0–7, store in `state.syzygy_probe_depth`
   - If `state.syzygy_tb` is `Some`, update its `probe_limit` field

**Go handler** — before spawning the search thread, construct the lazy tablebase prober:

If `state.syzygy_path` is non-empty:
- If `state.syzygy_tb` is `None`, create `LazySyzygyTablebase::new(state.syzygy_path.clone(), state.syzygy_probe_depth)` and store in `state.syzygy_tb`
- Move/clone the `LazySyzygyTablebase` into the spawned thread and pass as `Some(&mut tb_prober)` to `chess_search::search`

Since `LazySyzygyTablebase` needs to be used across multiple `Go` calls (to avoid re-initializing on every search), it should live in `EngineState`. But because the search runs in a separate thread, we need to transfer ownership. The approach:
- Take the `LazySyzygyTablebase` out of `state.syzygy_tb` (using `.take()`), move it into the spawned thread
- After the search thread completes, put it back (this requires restructuring the join handle to return the prober)

Actually, a simpler approach: since `SyzygyTablebase` (and thus `LazySyzygyTablebase`) holds the Fathom instance which is not `Send` in a trivial way — let me check. The `fathom_syzygy::Fathom` type is created in T01 as a dependency. The `SyzygyTablebase` wraps it. For the spawned thread, we need `Send`.

Given the existing architecture where search runs in `std::thread::spawn(move || { ... })`, the simplest correct approach matching the T02 deliverable pattern (which passes `None` currently) is:

- Create a fresh `LazySyzygyTablebase` for each search invocation, moving it into the thread. This means the tablebase may be re-initialized on each `go` command, but lazy loading ensures it only happens once (on first probe within that search). This is acceptable because:
  - Fathom initialization is fast (it memory-maps files)
  - The alternative (shared mutable state across threads) adds significant complexity

So in the `Go` handler:
```rust
let tb_prober: Option<chess_engine::syzygy::LazySyzygyTablebase> = if !state.syzygy_path.is_empty() {
    Some(chess_engine::syzygy::LazySyzygyTablebase::new(
        state.syzygy_path.clone(),
        state.syzygy_probe_depth,
    ))
} else {
    None
};

// In the spawned thread:
let mut tb = tb_prober;
let result = chess_search::search(
    &mut search_pos,
    limits,
    &game_history,
    Some(&|...| { ... }),
    tb.as_mut().map(|t| t as &mut dyn chess_search::TbProber),
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
- **Output**: `LazySyzygyTablebase` with deferred initialization

### `LazySyzygyTablebase` as `TbProber`
- `probe_wdl(pos)` → `Option<i32>` — returns `None` if path is empty, initialization fails, or piece count exceeds limit
- `probe_root(pos)` → `Option<(i32, i32)>` — returns `None` under same conditions; returns `(wdl_score, dtz)` tuple on success

## 5. Test Plan

### `crates/uci/src/output.rs` — Unit test for `option_spin`
- **Test**: `test_option_spin` — verify `option_spin("SyzygyProbeDepth", 6, 0, 7)` produces `"option name SyzygyProbeDepth type spin default 6 min 0 max 7"`.
- **Rationale**: Traces to exit criterion 4 ("UCI options control Syzygy tablebase path and probe depth") — the option must be correctly formatted.

### `crates/engine/src/syzygy/mod.rs` — Unit tests for lazy loading
- **Test**: `lazy_no_path_returns_none` — create `LazySyzygyTablebase` with an empty path, call `probe_wdl` on a position, verify it returns `None` without error.
  - **Rationale**: Traces to verification criterion 6 ("When SyzygyPath is not set or is empty, no probing is attempted and no errors occur").

- **Test**: `lazy_init_deferred` — create `LazySyzygyTablebase` with a path, verify `inner` is `None` immediately after construction (tablebase not loaded at construction time).
  - **Rationale**: Traces to exit criterion 5 ("Tablebase files are lazily loaded, not at startup").

### Existing tests must continue to pass
- All existing tests in `crates/engine/src/syzygy/mod.rs`, `crates/uci/`, and `crates/search/` must pass unchanged.
- `cargo clippy -- -D warnings` must pass.
- `cargo fmt --check` must pass.

## 6. Implementation Order

1. **`crates/uci/src/output.rs`** — Add `option_spin` function and its unit test. This is a leaf dependency used by subsequent steps.

2. **`crates/engine/src/syzygy/mod.rs`** — Add `LazySyzygyTablebase` struct with `new`, `ensure_init`, and `TbProber` implementation. Add unit tests for lazy loading behavior.

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
- **Why that approach is problematic**: `SyzygyTablebase` is a clean, non-lazy struct that takes a valid path in its constructor and guarantees the probing library is initialized. Adding lazy/optional state to it would complicate its API for all callers (including tests) and break the existing `SyzygyProberAdapter` which assumes a fully initialized `SyzygyTablebase`. It would also require changing the constructor signature and all existing test code.
- **Alternative chosen**: Introduce a new `LazySyzygyTablebase` wrapper in the same module that composes `Option<SyzygyTablebase>` with a stored path and probe limit, implementing `TbProber` directly. This preserves the existing `SyzygyTablebase` and `SyzygyProberAdapter` unchanged while providing the lazy loading behavior the task requires. The new struct is used exclusively by `main.rs` for UCI-driven lazy initialization.

**Deviation 2: Fresh `LazySyzygyTablebase` per search invocation instead of persistent shared state**

- **Parent spec requires**: Lazy loading where "tablebase files are only loaded when first probed" and re-initialization on path change.
- **Why persistent state across threads is problematic**: The search runs in a `std::thread::spawn(move || ...)` closure, requiring `Send` ownership transfer. Sharing mutable state across the main thread and search thread would require `Arc<Mutex<...>>` or similar synchronization, adding complexity disproportionate to the benefit. The Fathom library performs memory-mapping which is fast to re-initialize.
- **Alternative chosen**: Create a new `LazySyzygyTablebase` for each `go` command, moved into the search thread. Lazy initialization still occurs (only on first probe within that search), and path changes take effect immediately on the next `go`. This is simpler, correct, and avoids shared mutable state. Re-initialization cost is negligible since Fathom uses memory-mapped files.