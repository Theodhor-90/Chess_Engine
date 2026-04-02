I now have all the information needed. Let me produce the plan.

## 1. Deliverables

- **Modify** `crates/search/Cargo.toml` — Add `chess-nnue` dependency so the search crate can call the NNUE inference engine.
- **Modify** `crates/search/src/lib.rs` — Add NNUE-related fields to `SearchContext`, add an `EvalMode` enum, modify the `search()` function signature to accept an optional `&Network` and an `EvalMode`, and replace the two `chess_eval::evaluate()` call sites with a conditional dispatch that calls either NNUE forward pass or handcrafted eval based on mode and network availability.
- **Modify** `crates/engine/Cargo.toml` — Add `chess-nnue` dependency so the engine binary can load the network file.
- **Modify** `crates/engine/src/main.rs` — Add `nnue_network` field to `EngineState`, add `eval_mode` field, add `EvalFile` UCI string option (network file path) and `EvalMode` UCI combo option (`nnue`/`classical`), load the network via `chess_nnue::load()` when `EvalFile` is set, pass network reference and eval mode to `chess_search::search()`.

## 2. Dependencies

- `chess-nnue` crate (already in workspace at `crates/nnue`) — added as a dependency to `crates/search` and `crates/engine`.
- A trained `nets/nn.nnue` file from T02 is required for runtime NNUE evaluation. The code must gracefully handle its absence (fallback to classical).
- No new external crates are needed.

## 3. Implementation Details

### `crates/search/Cargo.toml`

Add `chess-nnue = { path = "../nnue" }` to `[dependencies]`.

### `crates/search/src/lib.rs`

**New public type — `EvalMode`:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalMode {
    Nnue,
    Classical,
}
```

**`SearchContext` modifications:**
Add two new fields:
```rust
network: Option<&'a Network>,  // reference to loaded NNUE network (None if not loaded)
eval_mode: EvalMode,
```

Since `SearchContext` currently has no lifetime parameter, and the network is passed from outside into the search function, the cleanest approach (matching existing patterns like `tb_prober`) is to **not** store the network in `SearchContext` but instead pass it as a separate parameter to `negamax` and `quiescence`, or store it as `Option<Arc<Network>>`. However, looking at how `tb_prober` is handled — it's passed as a parameter to `negamax`, not stored in `SearchContext` — and considering that the network is immutable and shared, the simplest approach is:

1. Store `eval_mode: EvalMode` in `SearchContext`.
2. Store the network reference via a raw pointer wrapped in a safe type, or more idiomatically, pass it through a new field. Since `SearchContext` is a local struct with no lifetime annotation, the simplest and cleanest approach is to add an `Option<std::sync::Arc<chess_nnue::Network>>` field plus an `accumulator: chess_nnue::Accumulator` field. The `Arc<Network>` is cheap to clone and lets the network be shared across the thread boundary (the engine spawns a search thread).

New fields on `SearchContext`:
```rust
network: Option<Arc<chess_nnue::Network>>,
eval_mode: EvalMode,
accumulator: chess_nnue::Accumulator,
```

**Evaluation dispatch helper** (private function):
```rust
fn eval_position(pos: &Position, ctx: &mut SearchContext) -> i32 {
    if ctx.eval_mode == EvalMode::Nnue {
        if let Some(ref network) = ctx.network {
            ctx.accumulator.refresh(pos, network);
            return chess_nnue::forward(&ctx.accumulator, network, pos.side_to_move());
        }
    }
    chess_eval::evaluate(pos, &mut ctx.pawn_table)
}
```

This function does a full accumulator refresh per evaluation call. Incremental accumulator updates are deferred to Phase P03 (T01) per the milestone plan. The full refresh is correct, just not maximally efficient.

**Modification at line 145** (quiescence `stand_pat`):
Replace `chess_eval::evaluate(pos, &mut ctx.pawn_table)` with `eval_position(pos, ctx)`.

**Modification at line 364** (negamax `static_eval`):
Replace `chess_eval::evaluate(pos, &mut ctx.pawn_table)` with `eval_position(pos, ctx)`.

**`search()` function signature change:**
```rust
pub fn search(
    pos: &mut Position,
    limits: SearchLimits,
    game_history: &[u64],
    on_depth: Option<DepthCallback<'_>>,
    mut tb_prober: Option<&mut dyn TbProber>,
    network: Option<Arc<chess_nnue::Network>>,
    eval_mode: EvalMode,
) -> Option<Move>
```

The two new parameters `network` and `eval_mode` are stored into `SearchContext` during construction (lines 685–704). The `accumulator` field is initialized via `chess_nnue::Accumulator::new()`.

**Test helper `test_ctx()`** (line 854):
Add the new fields with defaults: `network: None`, `eval_mode: EvalMode::Classical`, `accumulator: chess_nnue::Accumulator::new()`.

### `crates/engine/Cargo.toml`

Add `chess-nnue = { path = "../nnue" }` to `[dependencies]`.

### `crates/engine/src/main.rs`

**`EngineState` modifications:**
Add two new fields:
```rust
nnue_network: Option<Arc<chess_nnue::Network>>,
eval_mode: chess_search::EvalMode,
```

Initialize in `main()`:
```rust
nnue_network: None,
eval_mode: chess_search::EvalMode::Classical,
```

**UCI command handler — `Uci` response:**
Add two new option lines after the existing options:
```rust
println!("{}", chess_uci::output::option_string("EvalFile", ""));
println!("{}", chess_uci::output::option_combo("EvalMode", "nnue", &["nnue", "classical"]));
```

Note: The default for EvalMode is `"nnue"` as per the task spec ("defaults to NNUE when a network is loaded"). If no network is loaded, the fallback logic applies regardless.

**UCI command handler — `SetOption`:**
Add two new match arms:
```rust
"evalfile" => {
    if let Some(path) = value {
        if path.is_empty() {
            state.nnue_network = None;
        } else {
            match chess_nnue::load(std::path::Path::new(&path)) {
                Ok(net) => {
                    state.nnue_network = Some(Arc::new(net));
                    println!("{}", chess_uci::output::info_string(&format!(
                        "loaded NNUE network from {path}"
                    )));
                    io::stdout().flush().ok();
                }
                Err(e) => {
                    println!("{}", chess_uci::output::info_string(&format!(
                        "failed to load NNUE network: {e}"
                    )));
                    io::stdout().flush().ok();
                }
            }
        }
    } else {
        state.nnue_network = None;
    }
}
"evalmode" => {
    if let Some(val) = value {
        match val.to_lowercase().as_str() {
            "nnue" => state.eval_mode = chess_search::EvalMode::Nnue,
            "classical" => state.eval_mode = chess_search::EvalMode::Classical,
            _ => {}
        }
    }
}
```

**Search invocation (line 299–326):**
Pass the new parameters to `chess_search::search()`:
```rust
let network = state.nnue_network.clone();  // Arc clone, cheap
let eval_mode = if state.nnue_network.is_some() && state.eval_mode == chess_search::EvalMode::Nnue {
    chess_search::EvalMode::Nnue
} else {
    chess_search::EvalMode::Classical
};
// ... inside spawn:
let result = chess_search::search(
    &mut search_pos,
    limits,
    &game_history,
    Some(&|depth, score, nodes, elapsed, pv| { ... }),
    tb_prober.as_mut().map(|t| t as &mut dyn chess_search::TbProber),
    network,
    eval_mode,
);
```

The fallback logic: if `state.nnue_network` is `None`, `eval_mode` is forced to `Classical` regardless of the UCI setting. If `eval_mode` is `Classical`, the network is still passed but ignored by the dispatch function.

## 4. API Contracts

**`chess_search::EvalMode`** (new public enum):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalMode { Nnue, Classical }
```

**`chess_search::search()` (modified signature):**
```rust
pub fn search(
    pos: &mut Position,
    limits: SearchLimits,
    game_history: &[u64],
    on_depth: Option<DepthCallback<'_>>,
    tb_prober: Option<&mut dyn TbProber>,
    network: Option<Arc<chess_nnue::Network>>,
    eval_mode: EvalMode,
) -> Option<Move>
```

**UCI options** (new):
- `EvalFile` — `type string default <empty>` — path to `.nnue` weights file
- `EvalMode` — `type combo default nnue var nnue var classical` — evaluation mode selection

## 5. Test Plan

**Test 1: Eval dispatch uses classical when no network is loaded**
Setup: Create a `SearchContext` with `network: None`, `eval_mode: EvalMode::Nnue`. Call `eval_position()` on startpos.
Expected: Returns the same value as `chess_eval::evaluate()` (fallback to classical).

**Test 2: Eval dispatch uses classical when mode is Classical**
Setup: Create a `SearchContext` with `network: Some(...)` (zeroed network), `eval_mode: EvalMode::Classical`. Call `eval_position()` on startpos.
Expected: Returns the same value as `chess_eval::evaluate()`.

**Test 3: Eval dispatch uses NNUE when network is loaded and mode is Nnue**
Setup: Create a `SearchContext` with `network: Some(...)` (zeroed network), `eval_mode: EvalMode::Nnue`. Call `eval_position()` on startpos.
Expected: Returns 0 (zeroed network produces 0). This differs from classical eval which also returns 0 for startpos, so additionally test on a position where classical eval is non-zero (e.g., white up a queen) and verify the NNUE result differs from classical.

**Test 4: Search produces valid bestmove in both modes**
Setup: Run `search()` on startpos to depth 4 with `eval_mode: Classical` and `network: None`. Then run with `eval_mode: Nnue` and `network: None`.
Expected: Both return `Some(move)` (a valid move). Since no network is loaded, both should produce identical results.

**Test 5: Existing search tests pass unmodified**
Expected: All existing tests in `crates/search/src/lib.rs` continue to pass after updating `test_ctx()` with the new fields.

## 6. Implementation Order

1. **`crates/search/Cargo.toml`** — Add `chess-nnue` dependency.
2. **`crates/search/src/lib.rs`** — Add `EvalMode` enum, add `eval_position()` helper, add new fields to `SearchContext`, modify `search()` signature, replace eval calls in `quiescence()` and `negamax()`, update `test_ctx()`, add new tests.
3. **`crates/engine/Cargo.toml`** — Add `chess-nnue` dependency.
4. **`crates/engine/src/main.rs`** — Add `nnue_network` and `eval_mode` to `EngineState`, add UCI options `EvalFile` and `EvalMode`, handle `setoption` for both, pass network and eval mode to `chess_search::search()`.
5. **Verify** — Run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.

## 7. Verification Commands

```bash
# Format check
cargo fmt --check

# Clippy
cargo clippy -- -D warnings

# Run all tests
cargo test

# Run search-specific tests
cargo test -p chess-search

# Run engine-specific tests (compile check — engine has no unit tests)
cargo test -p engine

# Build the engine binary to verify it compiles
cargo build -p engine
```

## 8. Design Deviations

**Deviation 1: `Arc<Network>` instead of a borrow/reference**

- **Parent spec requires**: "Search integration in `crates/search` that calls NNUE eval during search when a network is loaded" — no specific ownership model prescribed.
- **Why a reference is problematic**: The engine spawns the search on a separate thread (`std::thread::spawn(move || { ... })`). A borrow `&Network` cannot be moved into a `'static` thread closure. The existing codebase clones the `Position` and other state before spawning. The `Network` is ~20 MB and must not be cloned per search.
- **Alternative chosen**: `Arc<chess_nnue::Network>` is used. The engine holds `Option<Arc<Network>>`, clones the `Arc` (cheap pointer copy) before spawning the search thread, and the search stores `Option<Arc<Network>>` in `SearchContext`. This is safe, efficient, and follows Rust's ownership model for cross-thread shared immutable data.

**Deviation 2: Full accumulator refresh per eval call (no incremental updates)**

- **Parent spec requires**: T03 says "NNUE forward pass when a network is loaded." Phase P03/T01 separately specifies "Accumulator incremental updates — only recompute affected features on make/unmake."
- **Why incremental updates here are premature**: Incremental updates require tracking which features changed on each `make_move`/`unmake_move`, maintaining an accumulator stack, and modifying the board crate or search crate to hook into move/unmove. This is explicitly deferred to Milestone M10, Phase P03, Task T01.
- **Alternative chosen**: Each evaluation call does a full `Accumulator::refresh()` from the current position. This is correct but not maximally fast. It is the intended approach for T03 (integration), with optimization following in P03.