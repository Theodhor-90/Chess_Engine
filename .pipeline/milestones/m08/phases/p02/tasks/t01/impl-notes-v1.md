**Files created/modified:**

- **`tools/texel/Cargo.toml`** — Created crate manifest with dependencies on `chess-types`, `chess-board`, `chess-eval`, `chess-movegen`
- **`Cargo.toml`** (workspace root) — Added `"tools/texel"` to workspace members list
- **`tools/texel/src/main.rs`** — Created Texel tuner binary (~1400 lines):
  - Non-tunable constants (file masks, adjacent file masks, bitboard helpers)
  - `TuningEntry` and `EvalParams` structs with 841 tunable parameters
  - `EvalParams::from_current()` with hardcoded private constant values + PST accessor reads
  - `EvalParams::as_param_vec()` / `from_param_vec()` / `param_count()` for coordinate descent
  - `load_dataset()` — parses `FEN | result` format
  - `sigmoid()` and `compute_mse()` — Texel loss function
  - `evaluate_position()` — wrapper using engine's `evaluate()` for K optimization
  - `eval_with_params()` — full evaluation reimplementation using mutable `EvalParams`: endgame probe, material+PST, pawn structure, king safety, mobility, piece bonuses, king centralization, tapered interpolation
  - `optimize_k()` — 4-pass line search over [0.0, 10.0] with refinement
  - `optimize_weights()` — coordinate descent with delta=10→1 transition
  - `print_weights()` — outputs valid Rust `const` declarations grouped by eval module
  - `main()` — CLI parsing (`--data`, `--optimize-k`, `--iterations`)