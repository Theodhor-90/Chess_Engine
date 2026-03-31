Now I have all the information I need. Let me produce the detailed implementation plan.

## 1. Deliverables

- `tools/texel/Cargo.toml` — Crate manifest for the Texel tuning binary, depending on `chess-types`, `chess-board`, `chess-movegen`, and `chess-eval`
- `tools/texel/src/main.rs` — Texel tuner binary entry point: CLI argument parsing, dataset loading, K-constant optimization, iterative weight optimization loop, and formatted Rust `const` output
- `Cargo.toml` (workspace root) — Updated `members` list to include `tools/texel`

## 2. Dependencies

- `chess-eval` crate (path dependency `../../crates/eval`) — provides `evaluate()`, `PawnHashTable`, material values, PST tables, and all evaluation sub-modules
- `chess-board` crate (path dependency `../../crates/board`) — provides `Position::from_fen()`
- `chess-types` crate (path dependency `../../crates/types`) — provides `Color`
- `chess-movegen` crate (path dependency `../../crates/movegen`) — provides `generate_legal_moves()` for filtering quiet positions (positions where no captures are available and side is not in check)
- No new external crate dependencies required. The tuner uses only standard library types (`std::env`, `std::fs`, `std::io`).

## 3. Implementation Details

### `tools/texel/Cargo.toml`

```toml
[package]
name = "texel"
version = "0.1.0"
edition.workspace = true

[dependencies]
chess-types = { path = "../../crates/types" }
chess-board = { path = "../../crates/board" }
chess-movegen = { path = "../../crates/movegen" }
chess-eval = { path = "../../crates/eval" }
```

Follows the exact pattern of existing tools (`tactical`, `selfplay`, `perft`).

### `Cargo.toml` (workspace root)

Add `"tools/texel"` to the `members` list, after `"tools/selfplay"`.

### `tools/texel/src/main.rs`

**CLI interface:**
- `--data <path>` — path to the dataset file (required). Each line: `<FEN> | <result>` where result is `1.0`, `0.5`, or `0.0`.
- `--optimize-k` — flag to run K-constant optimization only (print optimal K and exit).
- `--iterations <N>` — number of optimization iterations (default: 100).

**Data structures:**
- `struct TuningEntry { pos: Position, result: f64 }` — a single training position with its game outcome.

**Dataset loading (`load_dataset`):**
- Reads the file line by line.
- For each line, splits on `|` to extract FEN and result.
- Parses FEN via `Position::from_fen()`, skipping lines that fail.
- Parses result as `f64` (1.0 / 0.5 / 0.0), skipping invalid values.
- Returns `Vec<TuningEntry>`.

**Static evaluation wrapper (`evaluate_position`):**
- Takes `&Position`, creates a fresh `PawnHashTable`, calls `chess_eval::evaluate(pos, &mut pawn_table)`.
- Returns `i32` (score from white's perspective). The `evaluate()` function returns score relative to side-to-move, so for black-to-move positions the sign must be flipped to normalize to white's perspective.

**Sigmoid function:**
- `fn sigmoid(eval: f64, k: f64) -> f64` — returns `1.0 / (1.0 + 10.0_f64.powf(-k * eval / 400.0))`. The division by 400 normalizes centipawn evaluation to the standard Texel sigmoid scale.

**MSE computation (`compute_mse`):**
- Takes `&[TuningEntry]` and `k: f64`.
- For each entry, evaluates the position, computes `sigmoid(eval, k)`, computes `(sigmoid - result)^2`.
- Returns the mean of all squared errors.

**K-constant optimization (`optimize_k`):**
- Uses a simple line search: start with K range `[0.0, 10.0]`, step size 1.0.
- Find the K with the lowest MSE, then refine with step 0.1, then 0.01, then 0.001.
- Each refinement narrows the search to `[best_k - prev_step, best_k + prev_step]`.
- Prints the optimal K to stdout and returns it.

**Weight parameter representation:**
- `struct EvalParams` — holds mutable copies of all tunable evaluation constants:
  - `material: [i32; 5]` — `[PAWN_VALUE, KNIGHT_VALUE, BISHOP_VALUE, ROOK_VALUE, QUEEN_VALUE]`
  - `mg_pst: [[i32; 64]; 6]` — middlegame PSTs for Pawn, Knight, Bishop, Rook, Queen, King
  - `eg_pst: [[i32; 64]; 6]` — endgame PSTs for Pawn, Knight, Bishop, Rook, Queen, King
  - `pawn_mg: [i32; 6]` — `[DOUBLED_PAWN_PENALTY_MG, ISOLATED_PAWN_PENALTY_MG, BACKWARD_PAWN_PENALTY_MG, CONNECTED_PAWN_BONUS_MG, BLOCKED_PASSED_PAWN_PENALTY_MG, KING_PROXIMITY_OWN_BONUS]` (Note: `KING_PROXIMITY_OWN_BONUS` and `KING_PROXIMITY_OPP_BONUS` are EG-only in usage, included as single scalar)
  - `pawn_eg: [i32; 6]` — corresponding endgame values
  - `passed_pawn_bonus_mg: [i32; 8]` — `PASSED_PAWN_BONUS_MG`
  - `passed_pawn_bonus_eg: [i32; 8]` — `PASSED_PAWN_BONUS_EG`
  - `king_proximity_own_bonus: i32` — `KING_PROXIMITY_OWN_BONUS`
  - `king_proximity_opp_bonus: i32` — `KING_PROXIMITY_OPP_BONUS`
  - `pawn_shield_mg: [i32; 4]` — `PAWN_SHIELD_BONUS_MG`
  - `pawn_shield_eg: [i32; 4]` — `PAWN_SHIELD_BONUS_EG`
  - `open_file_king_mg: i32`, `open_file_king_eg: i32`
  - `attack_weights: [i32; 4]` — `[KNIGHT_ATTACK_WEIGHT, BISHOP_ATTACK_WEIGHT, ROOK_ATTACK_WEIGHT, QUEEN_ATTACK_WEIGHT]`
  - `attack_scale_mg: i32`, `attack_scale_eg: i32`
  - `mobility_mg: [i32; 4]` — `[KNIGHT_MOBILITY_MG, BISHOP_MOBILITY_MG, ROOK_MOBILITY_MG, QUEEN_MOBILITY_MG]`
  - `mobility_eg: [i32; 4]` — corresponding endgame values
  - `bishop_pair_mg: i32`, `bishop_pair_eg: i32`
  - `rook_open_file_mg: i32`, `rook_open_file_eg: i32`
  - `rook_semi_open_file_mg: i32`, `rook_semi_open_file_eg: i32`
  - `rook_seventh_mg: i32`, `rook_seventh_eg: i32`
  - `knight_outpost_mg: i32`, `knight_outpost_eg: i32`
  - `king_center_weight: i32`, `king_passed_pawn_proximity_weight: i32`

- `impl EvalParams`:
  - `fn from_current() -> Self` — initializes from current eval crate constants by reading the public const values and public accessor functions (`chess_eval::PAWN_VALUE`, `chess_eval::pst::mg_table()`, etc.)
  - `fn as_param_vec(&self) -> Vec<i32>` — flattens all parameters into a single vector for iteration
  - `fn from_param_vec(vec: &[i32]) -> Self` — reconstructs from flat vector
  - `fn param_count() -> usize` — total number of tunable parameters

**Custom evaluation using `EvalParams` (`eval_with_params`):**
- Reimplements the core evaluation logic from `chess_eval::evaluate()` but using the mutable `EvalParams` instead of the hardcoded constants.
- Takes `&Position`, `&EvalParams` and returns `i32` (score from white's perspective).
- Reuses the same evaluation structure: material + PST loop, pawn structure, king safety, mobility, piece bonuses, king centralization, tapered interpolation.
- Does NOT use the pawn hash table (unnecessary for tuning — we need fresh evals with each parameter change).

**Iterative weight optimization (`optimize_weights`):**
- Uses coordinate descent (the standard approach in Texel tuning):
  1. Flatten `EvalParams` into a parameter vector.
  2. For each parameter, try `param += delta` and `param -= delta`. Keep whichever reduces MSE, or keep original if neither helps.
  3. Start with `delta = 10`, reduce to `delta = 1` as iterations progress.
  4. One "iteration" = one full pass over all parameters.
  5. After each iteration, print current MSE to stderr for progress monitoring.
  6. Stop after `--iterations` passes or when MSE improvement per iteration falls below 1e-8.

**Output formatting (`print_weights`):**
- Prints optimized weights as valid Rust `const` declarations to stdout, organized by eval module.
- Format matches existing conventions exactly:
  - `pub const PAWN_VALUE: i32 = <value>;`
  - `pub const MG_PAWN_TABLE: [i32; 64] = [ ... ];` (with `#[rustfmt::skip]` and rank comments)
  - Module-private constants use `const NAME: i32 = <value>;` (no `pub`)
- Groups output by source file with comment headers: `// material.rs`, `// pst.rs`, `// pawn.rs`, `// king_safety.rs`, `// mobility.rs`, `// pieces.rs`, `// king_centralization.rs`.

**`main()` flow:**
1. Parse CLI arguments.
2. Load dataset via `load_dataset()`.
3. Print dataset statistics (number of entries, win/draw/loss distribution) to stderr.
4. Run `optimize_k()` to find optimal K. If `--optimize-k` flag, print K and exit.
5. Run `optimize_weights()` with the optimal K and the specified number of iterations.
6. Call `print_weights()` to emit optimized constants to stdout.

## 4. API Contracts

**Dataset file format (input):**
```
rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1 | 0.5
r1bqkb1r/pppppppp/2n2n2/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3 | 1.0
```
Each line: FEN (full 6-field), pipe separator, game result (1.0 = white win, 0.5 = draw, 0.0 = white loss).

**CLI usage:**
```bash
# Optimize K only
cargo run --release -p texel -- --data positions.txt --optimize-k

# Full tuning run
cargo run --release -p texel -- --data positions.txt --iterations 100

# Redirect output to file for pasting into eval crate
cargo run --release -p texel -- --data positions.txt --iterations 100 > tuned_weights.rs
```

**Stdout output format:**
```rust
// material.rs
pub const PAWN_VALUE: i32 = 102;
pub const KNIGHT_VALUE: i32 = 318;
// ... etc

// pst.rs
#[rustfmt::skip]
pub const MG_PAWN_TABLE: [i32; 64] = [
      0,   0,   0,   0,   0,   0,   0,   0,  // rank 1
    -33,  -1, -18, -21, -13,  26,  40, -20,  // rank 2
    // ... etc
];
```

N/A for REST/HTTP.

## 5. Test Plan

The task spec's verification criteria are:

| Test | What it verifies |
|------|-----------------|
| `cargo build` succeeds with `tools/texel` in workspace | Binary compiles and all dependencies resolve (verification criterion 1) |
| Binary loads a sample dataset file | Dataset parsing works: FEN + result parsing, skip invalid lines (verification criterion 2) |
| K-constant optimization converges on sample data | `optimize_k()` runs and returns a stable K value, MSE decreases across refinement steps (verification criterion 3) |
| Optimization loop reduces MSE over iterations | After N iterations, MSE is measurably lower than initial MSE (verification criterion 4) |
| Output format matches eval crate constants | Printed constants are valid Rust syntax with correct names and types matching those in the eval crate source files (verification criterion 5) |
| `cargo clippy -- -D warnings` passes | No lint warnings (verification criterion 6) |
| `cargo fmt --check` passes | Formatting correct (verification criterion 6) |
| `cargo test` passes | All existing tests remain green (verification criterion 7) |

**Test setup:** Create a small sample dataset of ~20 positions with game outcomes (using known positions from the existing test suites in the eval crate) for manual verification. This dataset is for development testing only — the actual tuning in T02 uses 1M+ positions from CCRL.

**Note:** The tuner is a binary tool (like `perft`, `tactical`, `selfplay`), so verification is done via running the binary and checking outputs, not via unit tests in the source. This is consistent with how all other tools in `tools/` are structured — none of them contain `#[cfg(test)]` modules.

## 6. Implementation Order

1. `tools/texel/Cargo.toml` — create the crate manifest with dependencies
2. `Cargo.toml` (workspace root) — add `"tools/texel"` to workspace members
3. `tools/texel/src/main.rs` — implement in this order within the file:
   1. Data structures (`TuningEntry`, `EvalParams`)
   2. Dataset loading (`load_dataset`)
   3. Sigmoid function and MSE computation (`sigmoid`, `compute_mse`)
   4. Static evaluation wrapper (`evaluate_position`) and custom eval (`eval_with_params`)
   5. `EvalParams` construction from current constants and serialization (`from_current`, `as_param_vec`, `from_param_vec`)
   6. K-constant optimization (`optimize_k`)
   7. Iterative weight optimization (`optimize_weights`)
   8. Output formatting (`print_weights`)
   9. CLI parsing and `main()`
4. Verify: `cargo build`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test`
5. Manual test: create a small sample dataset, run `--optimize-k`, run full tuning with `--iterations 5`, verify output format

## 7. Verification Commands

```bash
# Build the workspace including the new texel tool
cargo build --release -p texel

# Lint check
cargo clippy -- -D warnings

# Format check
cargo fmt --check

# Run all existing tests to confirm no regressions
cargo test
```

## 8. Design Deviations

**Deviation 1: Custom `eval_with_params` reimplements evaluation logic instead of calling `chess_eval::evaluate()`.**

- **Parent spec requires:** "Links against the `eval` crate to access evaluation functions directly (in-process)"
- **Why direct `evaluate()` is problematic:** The eval crate's `evaluate()` uses hardcoded `const` values. To tune parameters, we need to evaluate positions with *modified* weights. Calling `evaluate()` directly would always use the original constants, making optimization impossible. Rust `const` values are compiled into the binary and cannot be mutated at runtime.
- **Alternative chosen:** The tuner reimplements the evaluation logic in `eval_with_params()`, reading the initial values from the eval crate's public constants/accessors to initialize `EvalParams`, then using the mutable params for evaluation during optimization. The eval crate is still a dependency (for reading current constants and for the `PawnHashTable` / `Position` types), satisfying the spirit of "links against the eval crate." The reimplemented evaluation follows the exact same algorithm as `chess_eval::evaluate()` to ensure equivalence.