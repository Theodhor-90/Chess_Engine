Now I have all the information needed. Let me produce the revised plan addressing both feedback issues:

1. **Correctness**: Restore `chess-movegen` as a dependency since `eval_with_params` needs `KNIGHT_ATTACKS`, `KING_ATTACKS`, `bishop_attacks()`, and `rook_attacks()`.
2. **Completeness**: Add the endgame probe (`chess_eval::endgame::probe_endgame()`) as step 0 in `eval_with_params`.

## 1. Deliverables

- `tools/texel/Cargo.toml` — Crate manifest for the Texel tuning binary, depending on `chess-types`, `chess-board`, `chess-eval`, and `chess-movegen`
- `tools/texel/src/main.rs` — Texel tuner binary entry point: CLI argument parsing, dataset loading, K-constant optimization, iterative weight optimization loop, and formatted Rust `const` output
- `Cargo.toml` (workspace root) — Updated `members` list to include `tools/texel`

## 2. Dependencies

- `chess-eval` crate (path dependency `../../crates/eval`) — provides `evaluate()`, `PawnHashTable`, material values, PST tables, `endgame::probe_endgame()`, `phase::compute_phase()`, `pst::mg_table()` / `pst::eg_table()` / `pst::mirror_square()`
- `chess-board` crate (path dependency `../../crates/board`) — provides `Position::from_fen()`
- `chess-types` crate (path dependency `../../crates/types`) — provides `Color`, `PieceKind`, `Piece`, `Bitboard`, `Square`
- `chess-movegen` crate (path dependency `../../crates/movegen`) — provides `knights::KNIGHT_ATTACKS`, `king::KING_ATTACKS`, `magic::bishop_attacks()`, `magic::rook_attacks()`. Required by `eval_with_params` for mobility evaluation (knight, bishop, rook, queen attack lookups) and king safety evaluation (king zone computation, opponent piece attack zone intersection).
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
chess-eval = { path = "../../crates/eval" }
chess-movegen = { path = "../../crates/movegen" }
```

Follows the exact pattern of existing tools (`perft`, `tactical`, `selfplay`). The `chess-movegen` dependency is required because `eval_with_params` reimplements the mobility and king safety evaluation terms, which use attack table lookups from the movegen crate: `KNIGHT_ATTACKS` (from `chess_movegen::knights`), `KING_ATTACKS` (from `chess_movegen::king`), and `bishop_attacks()`/`rook_attacks()` (from `chess_movegen::magic`).

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
- Prints the optimal K to stderr and returns it.

**Weight parameter representation (`EvalParams`):**

`struct EvalParams` holds mutable copies of all tunable evaluation constants. The struct fields and their exact mapping to eval crate constants are:

| Field | Type | Source Constants (file → name) |
|-------|------|-------------------------------|
| `material` | `[i32; 5]` | `material.rs` → `PAWN_VALUE` [0], `KNIGHT_VALUE` [1], `BISHOP_VALUE` [2], `ROOK_VALUE` [3], `QUEEN_VALUE` [4] |
| `mg_pst` | `[[i32; 64]; 6]` | `pst.rs` → `MG_PAWN_TABLE` [0], `MG_KNIGHT_TABLE` [1], `MG_BISHOP_TABLE` [2], `MG_ROOK_TABLE` [3], `MG_QUEEN_TABLE` [4], `MG_KING_TABLE` [5] |
| `eg_pst` | `[[i32; 64]; 6]` | `pst.rs` → `EG_PAWN_TABLE` [0], `EG_KNIGHT_TABLE` [1], `EG_BISHOP_TABLE` [2], `EG_ROOK_TABLE` [3], `EG_QUEEN_TABLE` [4], `EG_KING_TABLE` [5] |
| `doubled_pawn_mg` | `i32` | `pawn.rs` → `DOUBLED_PAWN_PENALTY_MG` |
| `doubled_pawn_eg` | `i32` | `pawn.rs` → `DOUBLED_PAWN_PENALTY_EG` |
| `isolated_pawn_mg` | `i32` | `pawn.rs` → `ISOLATED_PAWN_PENALTY_MG` |
| `isolated_pawn_eg` | `i32` | `pawn.rs` → `ISOLATED_PAWN_PENALTY_EG` |
| `backward_pawn_mg` | `i32` | `pawn.rs` → `BACKWARD_PAWN_PENALTY_MG` |
| `backward_pawn_eg` | `i32` | `pawn.rs` → `BACKWARD_PAWN_PENALTY_EG` |
| `connected_pawn_mg` | `i32` | `pawn.rs` → `CONNECTED_PAWN_BONUS_MG` |
| `connected_pawn_eg` | `i32` | `pawn.rs` → `CONNECTED_PAWN_BONUS_EG` |
| `blocked_passed_pawn_mg` | `i32` | `pawn.rs` → `BLOCKED_PASSED_PAWN_PENALTY_MG` |
| `blocked_passed_pawn_eg` | `i32` | `pawn.rs` → `BLOCKED_PASSED_PAWN_PENALTY_EG` |
| `passed_pawn_bonus_mg` | `[i32; 8]` | `pawn.rs` → `PASSED_PAWN_BONUS_MG` |
| `passed_pawn_bonus_eg` | `[i32; 8]` | `pawn.rs` → `PASSED_PAWN_BONUS_EG` |
| `king_proximity_own` | `i32` | `pawn.rs` → `KING_PROXIMITY_OWN_BONUS` |
| `king_proximity_opp` | `i32` | `pawn.rs` → `KING_PROXIMITY_OPP_BONUS` |
| `pawn_shield_mg` | `[i32; 4]` | `king_safety.rs` → `PAWN_SHIELD_BONUS_MG` |
| `pawn_shield_eg` | `[i32; 4]` | `king_safety.rs` → `PAWN_SHIELD_BONUS_EG` |
| `open_file_king_mg` | `i32` | `king_safety.rs` → `OPEN_FILE_NEAR_KING_PENALTY_MG` |
| `open_file_king_eg` | `i32` | `king_safety.rs` → `OPEN_FILE_NEAR_KING_PENALTY_EG` |
| `attack_weights` | `[i32; 4]` | `king_safety.rs` → `KNIGHT_ATTACK_WEIGHT` [0], `BISHOP_ATTACK_WEIGHT` [1], `ROOK_ATTACK_WEIGHT` [2], `QUEEN_ATTACK_WEIGHT` [3] |
| `attack_scale_mg` | `i32` | `king_safety.rs` → `ATTACK_SCORE_SCALE_MG` |
| `attack_scale_eg` | `i32` | `king_safety.rs` → `ATTACK_SCORE_SCALE_EG` |
| `mobility_mg` | `[i32; 4]` | `mobility.rs` → `KNIGHT_MOBILITY_MG` [0], `BISHOP_MOBILITY_MG` [1], `ROOK_MOBILITY_MG` [2], `QUEEN_MOBILITY_MG` [3] |
| `mobility_eg` | `[i32; 4]` | `mobility.rs` → `KNIGHT_MOBILITY_EG` [0], `BISHOP_MOBILITY_EG` [1], `ROOK_MOBILITY_EG` [2], `QUEEN_MOBILITY_EG` [3] |
| `bishop_pair_mg` | `i32` | `pieces.rs` → `BISHOP_PAIR_BONUS_MG` |
| `bishop_pair_eg` | `i32` | `pieces.rs` → `BISHOP_PAIR_BONUS_EG` |
| `rook_open_file_mg` | `i32` | `pieces.rs` → `ROOK_OPEN_FILE_BONUS_MG` |
| `rook_open_file_eg` | `i32` | `pieces.rs` → `ROOK_OPEN_FILE_BONUS_EG` |
| `rook_semi_open_file_mg` | `i32` | `pieces.rs` → `ROOK_SEMI_OPEN_FILE_BONUS_MG` |
| `rook_semi_open_file_eg` | `i32` | `pieces.rs` → `ROOK_SEMI_OPEN_FILE_BONUS_EG` |
| `rook_seventh_mg` | `i32` | `pieces.rs` → `ROOK_SEVENTH_RANK_BONUS_MG` |
| `rook_seventh_eg` | `i32` | `pieces.rs` → `ROOK_SEVENTH_RANK_BONUS_EG` |
| `knight_outpost_mg` | `i32` | `pieces.rs` → `KNIGHT_OUTPOST_BONUS_MG` |
| `knight_outpost_eg` | `i32` | `pieces.rs` → `KNIGHT_OUTPOST_BONUS_EG` |
| `king_center_weight` | `i32` | `king_centralization.rs` → `KING_CENTER_BONUS_WEIGHT` |
| `king_passed_pawn_proximity_weight` | `i32` | `king_centralization.rs` → `KING_PASSED_PAWN_PROXIMITY_WEIGHT` |

**Total tunable parameters:** 5 (material) + 768 (PSTs: 6×64×2) + 14 (pawn scalars) + 16 (passed pawn arrays) + 2 (king proximity) + 8 (pawn shield) + 2 (open file king) + 4 (attack weights) + 2 (attack scale) + 8 (mobility) + 10 (piece bonuses) + 2 (king centralization) = **841 parameters**.

**Initialization strategy for private constants (`EvalParams::from_current`):**

The eval crate constants have two visibility levels:
- **Public** (accessible directly): 5 material values (`chess_eval::PAWN_VALUE`, etc.) and 12 PST tables (via `chess_eval::pst::mg_table()` / `chess_eval::pst::eg_table()` accessors).
- **Private** (~60+ constants in `pawn.rs`, `king_safety.rs`, `mobility.rs`, `pieces.rs`, `king_centralization.rs`): These are `const` (not `pub const`) and cannot be read from outside their module.

The `from_current()` method initializes all fields by **hardcoding the current values** directly in the tuner source code. This avoids modifying the eval crate's visibility. The hardcoded values are:

- **material.rs** (public, but hardcoded for consistency): `[100, 320, 330, 500, 900]`
- **PST tables** (public): Read via `chess_eval::pst::mg_table(kind)` and `chess_eval::pst::eg_table(kind)` for each `PieceKind`, copying the 64-element arrays into `EvalParams`.
- **pawn.rs** (private): `doubled_pawn_mg: -10`, `doubled_pawn_eg: -20`, `isolated_pawn_mg: -15`, `isolated_pawn_eg: -20`, `backward_pawn_mg: -10`, `backward_pawn_eg: -15`, `connected_pawn_mg: 5`, `connected_pawn_eg: 7`, `blocked_passed_pawn_mg: -5`, `blocked_passed_pawn_eg: -10`, `passed_pawn_bonus_mg: [0, 0, 5, 10, 20, 35, 60, 0]`, `passed_pawn_bonus_eg: [0, 0, 10, 20, 40, 70, 120, 0]`, `king_proximity_own: 5`, `king_proximity_opp: 3`
- **king_safety.rs** (private): `pawn_shield_mg: [15, 8, 3, 0]`, `pawn_shield_eg: [10, 5, 2, 0]`, `open_file_king_mg: -15`, `open_file_king_eg: -10`, `attack_weights: [2, 2, 3, 5]`, `attack_scale_mg: 1`, `attack_scale_eg: 1`
- **mobility.rs** (private): `mobility_mg: [4, 5, 2, 1]`, `mobility_eg: [4, 5, 3, 2]`
- **pieces.rs** (private): `bishop_pair_mg: 30`, `bishop_pair_eg: 50`, `rook_open_file_mg: 20`, `rook_open_file_eg: 10`, `rook_semi_open_file_mg: 10`, `rook_semi_open_file_eg: 5`, `rook_seventh_mg: 20`, `rook_seventh_eg: 30`, `knight_outpost_mg: 20`, `knight_outpost_eg: 15`
- **king_centralization.rs** (private): `king_center_weight: 10`, `king_passed_pawn_proximity_weight: 5`

A `// NOTE: keep in sync with eval crate constants` comment will mark the hardcoded block. When T02 applies tuned weights to the eval crate, these values will be updated there — the tuner's hardcoded initial values only matter for the first tuning pass.

**`impl EvalParams` methods:**
- `fn from_current() -> Self` — initializes from hardcoded values (private constants) and PST accessor functions (public constants), as described above
- `fn as_param_vec(&self) -> Vec<i32>` — flattens all 841 parameters into a single vector for coordinate descent iteration
- `fn from_param_vec(vec: &[i32]) -> Self` — reconstructs `EvalParams` from the flat vector (inverse of `as_param_vec`)
- `fn param_count() -> usize` — returns 841

**Custom evaluation using `EvalParams` (`eval_with_params`):**
- Reimplements the core evaluation logic from `chess_eval::evaluate()` but using the mutable `EvalParams` instead of the hardcoded constants.
- Takes `&Position`, `&EvalParams` and returns `i32` (score from white's perspective).
- Reuses the same evaluation structure: endgame probe, material + PST loop, pawn structure, king safety, mobility, piece bonuses, king centralization, tapered interpolation.
- Does NOT use the pawn hash table (unnecessary for tuning — we need fresh evals with each parameter change).
- The evaluation logic is structured as follows, matching `chess_eval::evaluate()` in `lib.rs`:
  0. **Endgame probe**: Call `chess_eval::endgame::probe_endgame(pos)` first. If it returns `Some(score)`, return that score directly (already from white's perspective, as the engine's `evaluate()` returns it relative to white after the side-to-move flip in `lib.rs:20-24`). This matches the engine's `evaluate()` which short-circuits at `lib.rs:19` before any tunable terms are evaluated. Endgame scores (`KNOWN_WIN ± bonuses` for KR/KQ/KBN vs K, `0` for drawn endgames) are position-dependent calculations that do not use any tunable parameters, so they are not part of the optimization — but they must be present for MSE correctness.
  1. Material + PST: loop over all piece kinds and squares, sum `params.material[kind] + params.mg_pst[kind][sq]` for MG and similarly for EG. Flip sign for black. Uses `chess_eval::pst::mirror_square()` for black pieces.
  2. Pawn structure: reimplement `evaluate_pawns()` and `evaluate_passed_pawn_extras()` using params instead of consts. Uses bitboard operations from `chess_types::Bitboard`. File masks and adjacent file masks are non-tunable constants — hardcode them in the tuner (same values as in `pawn.rs`).
  3. King safety: reimplement `evaluate_king_safety()` using `params.pawn_shield_mg/eg`, `params.open_file_king_mg/eg`, `params.attack_weights`, `params.attack_scale_mg/eg`. Uses `chess_movegen::king::KING_ATTACKS` for king zone computation, `chess_movegen::knights::KNIGHT_ATTACKS` for opponent knight attack lookups, and `chess_movegen::magic::bishop_attacks()` / `chess_movegen::magic::rook_attacks()` for opponent bishop, rook, and queen attack lookups.
  4. Mobility: reimplement `evaluate_mobility()` using `params.mobility_mg/eg`. Uses `chess_movegen::knights::KNIGHT_ATTACKS` for knight mobility, `chess_movegen::magic::bishop_attacks()` for bishop mobility, and `chess_movegen::magic::rook_attacks()` for rook and queen mobility.
  5. Piece bonuses: reimplement `evaluate_piece_bonuses()` using `params.bishop_pair_mg/eg`, `params.rook_open_file_mg/eg`, etc.
  6. King centralization: reimplement `evaluate_king_centralization()` using `params.king_center_weight`, `params.king_passed_pawn_proximity_weight`.
  7. Tapered interpolation: `((mg * phase) + (eg * (MAX_PHASE - phase))) / MAX_PHASE`. Phase computation uses `chess_eval::phase::compute_phase()` (public function, does not depend on tunable parameters).

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
- Each constant uses the exact name and type from the eval crate (e.g., `const DOUBLED_PAWN_PENALTY_MG: i32 = <value>;`).

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

**Test setup:** Create a small sample dataset of ~20 positions with game outcomes (using known positions from the existing test suites) for manual verification. This dataset is for development testing only — the actual tuning in T02 uses 1M+ positions from CCRL.

**Note:** The tuner is a binary tool (like `perft`, `tactical`, `selfplay`), so verification is done via running the binary and checking outputs, not via unit tests in the source. This is consistent with how all other tools in `tools/` are structured — none of them contain `#[cfg(test)]` modules.

## 6. Implementation Order

1. `tools/texel/Cargo.toml` — create the crate manifest with dependencies (`chess-types`, `chess-board`, `chess-eval`, `chess-movegen`)
2. `Cargo.toml` (workspace root) — add `"tools/texel"` to workspace members
3. `tools/texel/src/main.rs` — implement in this order within the file:
   1. Non-tunable constants (file masks, adjacent file masks, bitboard helpers)
   2. Data structures (`TuningEntry`, `EvalParams` with the full field list from section 3)
   3. `EvalParams` methods (`from_current` with hardcoded values, `as_param_vec`, `from_param_vec`, `param_count`)
   4. Dataset loading (`load_dataset`)
   5. Sigmoid function and MSE computation (`sigmoid`, `compute_mse`)
   6. Custom eval (`eval_with_params`) — endgame probe first, then reimplementing each eval term using `EvalParams` and movegen attack tables
   7. K-constant optimization (`optimize_k`)
   8. Iterative weight optimization (`optimize_weights`)
   9. Output formatting (`print_weights`)
   10. CLI parsing and `main()`
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

- **Parent spec states:** "Links against the `eval` crate to access evaluation functions directly (in-process)"
- **Why direct `evaluate()` is insufficient:** The eval crate's `evaluate()` uses hardcoded `const` values baked into the binary at compile time. To tune parameters, we need to evaluate positions with *modified* weights. Calling `evaluate()` directly would always use the original constants, making optimization impossible — Rust `const` values cannot be mutated at runtime.
- **What we do instead:** The tuner reimplements the evaluation logic in `eval_with_params()`, using the mutable `EvalParams` struct instead of hardcoded constants. The eval crate is still a dependency — we use `chess_eval::endgame::probe_endgame()` for the endgame short-circuit, `chess_eval::pst::mg_table()` / `chess_eval::pst::eg_table()` to read PST values, `chess_eval::pst::mirror_square()` for black piece indexing, and `chess_eval::phase::compute_phase()` for phase calculation. The movegen crate is also a dependency — we use `chess_movegen::knights::KNIGHT_ATTACKS`, `chess_movegen::king::KING_ATTACKS`, `chess_movegen::magic::bishop_attacks()`, and `chess_movegen::magic::rook_attacks()` for the mobility and king safety evaluation terms. This satisfies the spirit of "links against the eval crate" while enabling runtime parameter mutation.

**Deviation 2: Private eval constants are hardcoded in the tuner rather than making them `pub`.**

- **Parent spec states:** The tuner "links against the `eval` crate to access evaluation functions directly"
- **Why not make them `pub`:** The ~60 private constants in `pawn.rs`, `king_safety.rs`, `mobility.rs`, `pieces.rs`, and `king_centralization.rs` are deliberately private to encapsulate evaluation internals. Making them all `pub` just for the tuner would pollute the eval crate's public API and violate the existing design where only material values and PST tables are exported.
- **What we do instead:** `EvalParams::from_current()` hardcodes the current values of all private constants directly in the tuner source. A `// NOTE: keep in sync with eval crate constants` comment marks this block. This is a one-time snapshot used to initialize the optimization — the tuner then adjusts these values through coordinate descent. When T02 applies tuned weights, the eval crate constants are updated, not the tuner's initialization values.