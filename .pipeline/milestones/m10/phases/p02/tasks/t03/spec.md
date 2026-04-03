## Task T03 — Evaluation Switchover

### Objective

Integrate NNUE evaluation into the search loop in `crates/search`, replacing the handcrafted eval call with an NNUE forward pass when a network is loaded. Add the UCI option `EvalMode` to select between NNUE and classical evaluation modes. Retain the handcrafted eval as a fallback.

### Deliverables

- Search integration in `crates/search` that calls NNUE eval during search when a network is loaded
- UCI `EvalMode` option in `crates/uci` allowing selection between `nnue` and `classical` evaluation modes
- Fallback to handcrafted eval when no `.nnue` network is loaded or when classical mode is selected
- Engine wiring in `crates/engine` for NNUE network loading and evaluation mode selection

### Key Implementation Details

- **Search integration point**: In the search loop within `crates/search`, the call to the handcrafted evaluation function (from `crates/eval`) is replaced with a call to the NNUE inference engine (from `crates/nnue`, built in Phase P01) when NNUE mode is active and a network is loaded
- **UCI option `EvalMode`**: Add a new UCI option in `crates/uci` with possible values `nnue` (default when a network is loaded) and `classical`. This follows the UCI `option` command protocol (type combo)
- **Fallback logic**: If no `.nnue` file is loaded, the engine automatically falls back to the handcrafted eval regardless of the `EvalMode` setting. If `EvalMode` is set to `classical`, the handcrafted eval is used even if a network is available
- **Default behavior**: The engine defaults to NNUE evaluation when a network file is loaded
- **Handcrafted eval preservation**: The handcrafted evaluation in `crates/eval` must remain fully functional and unchanged — it is not removed or degraded
- The NNUE inference engine and accumulator update logic from Phase P01 are consumed here

### Files to Create or Modify

- **Modify**: `crates/search/` — replace handcrafted eval call with conditional NNUE/classical dispatch
- **Modify**: `crates/uci/` — add `EvalMode` UCI option (`nnue` | `classical`)
- **Modify**: `crates/engine/` — wire NNUE network loading and evaluation mode selection into the engine initialization
- **No changes to**: `crates/eval/` — handcrafted eval remains as-is

### Dependencies

- **T02 (Training Pipeline)** — a trained `.nnue` weights file must exist to validate NNUE evaluation
- **T01 (Training Data Generation)** — indirectly, via T02
- Phase P01 (NNUE Architecture) — inference engine (`crates/nnue`), accumulator updates, and weight loading must be complete
- `crates/eval` — existing handcrafted evaluation used as fallback
- `crates/search` — search framework being modified
- `crates/uci` — UCI protocol layer being modified

### Verification Criteria

1. With a `.nnue` file loaded and `EvalMode` set to `nnue`, the engine uses NNUE evaluation during search
2. With `EvalMode` set to `classical`, the engine uses the handcrafted eval during search
3. With no `.nnue` file loaded, the engine falls back to handcrafted eval regardless of `EvalMode` setting
4. The UCI `EvalMode` option appears in the engine's `uci` response and can be set via `setoption`
5. The handcrafted eval in `crates/eval` is unchanged and fully functional
6. The engine produces valid `bestmove` responses in both evaluation modes
7. `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` all pass
8. Search results with NNUE eval are stronger than handcrafted eval (validated by SPRT in T02)