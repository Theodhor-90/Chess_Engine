Now I have a complete understanding of the codebase. Key observations:

1. **`vec_add_i16` and `vec_sub_i16`** exist in `simd.rs` with full AVX2/SSE2/NEON implementations but are **not used** by `accumulator.rs` — the `add_feature`/`remove_feature` methods do manual scalar loops.
2. **`clipped_relu_i16_to_i8`** and **`dot_i8_i32`** in `simd.rs` already have full SIMD implementations and are used by `inference.rs`.
3. The existing SIMD functions are `pub(crate)` and marked `#[allow(dead_code)]` on the vec add/sub functions.
4. There is no benchmark infrastructure yet.

The task requires:
- SIMD-optimizing all three hot operations (accumulator updates, clipped-ReLU, dot products)
- The clipped-ReLU and dot product SIMD paths **already exist**
- The accumulator incremental updates need to be wired to use the SIMD `vec_add_i16`/`vec_sub_i16` functions
- Benchmarks need to be created demonstrating 1.5x NPS improvement

## 1. Deliverables

| # | File | Action | Purpose |
|---|------|--------|---------|
| 1 | `crates/nnue/src/accumulator.rs` | Modify | Replace scalar loops in `add_feature` and `remove_feature` with calls to `simd::vec_add_i16` and `simd::vec_sub_i16`, enabling SIMD-accelerated accumulator incremental updates. |
| 2 | `crates/nnue/src/simd.rs` | Modify | Remove `#[allow(dead_code)]` from `vec_add_i16`, `vec_sub_i16`, `vec_add_scalar`, and `vec_sub_scalar` now that they are actively used. |
| 3 | `benches/nnue_bench.rs` | Create | Criterion benchmarks measuring NPS for the forward pass, accumulator add/remove operations, and full evaluation via `AccumulatorStack::evaluate`, comparing scalar fallback vs SIMD-enabled codepaths. |
| 4 | `Cargo.toml` (workspace root) | Modify | Add `benches/nnue_bench.rs` as a `[[bench]]` target and add the `criterion` dev-dependency needed for benchmarks. |

## 2. Dependencies

- **`criterion`** — required as a dev-dependency at the workspace root for the benchmark harness. Version `0.5` (current stable).
- **T01 (Accumulator Incremental Updates)** — completed; provides `AccumulatorStack` with dirty flag tracking, `push_move`/`pop`, and `refresh_if_needed`.
- **Phase P01 (NNUE Architecture)** — completed; provides the inference engine, SIMD feature flags, scalar codepaths, and the `Network`/`Accumulator` types.
- **Phase P02 (Training & Integration)** — completed; provides a trained network for meaningful NPS benchmarks.
- **Existing SIMD infrastructure** — `simd.rs` already contains complete AVX2, SSE2, NEON, and scalar implementations for all four vector operations (`clipped_relu_i16_to_i8`, `dot_i8_i32`, `vec_add_i16`, `vec_sub_i16`). The clipped-ReLU and dot product paths are already wired into `inference.rs`. The accumulator add/subtract paths exist but are not wired into `accumulator.rs`.

## 3. Implementation Details

### 3.1 `crates/nnue/src/accumulator.rs` (Modify)

**Purpose**: Wire the accumulator's feature add/remove operations to use SIMD-accelerated vector addition and subtraction instead of scalar loops.

**Changes**:

1. Add `use crate::simd;` to imports.

2. **`Accumulator::add_feature`** — Replace the scalar loop:
   ```rust
   // Current (scalar):
   for i in 0..L1_SIZE {
       vals[i] += weights[offset + i];
   }
   ```
   With a call to the SIMD-dispatched function:
   ```rust
   // SIMD-dispatched:
   simd::vec_add_i16(vals, &weights[offset..offset + L1_SIZE]);
   ```

3. **`Accumulator::remove_feature`** — Replace the scalar loop:
   ```rust
   // Current (scalar):
   for i in 0..L1_SIZE {
       vals[i] -= weights[offset + i];
   }
   ```
   With:
   ```rust
   // SIMD-dispatched:
   simd::vec_sub_i16(vals, &weights[offset..offset + L1_SIZE]);
   ```

**Exports**: No change to public API.

**Contracts**: `add_feature` and `remove_feature` must produce identical results to the scalar versions (integer arithmetic is exact). The `debug_assert_eq!` in `vec_add_i16`/`vec_sub_i16` verifies length matching. L1_SIZE=256 is divisible by 16 (AVX2 chunk size) and 8 (SSE2/NEON chunk size), so no remainder elements are processed by scalar fallback.

### 3.2 `crates/nnue/src/simd.rs` (Modify)

**Purpose**: Clean up dead-code attributes now that `vec_add_i16`, `vec_sub_i16`, `vec_add_scalar`, and `vec_sub_scalar` are actively used.

**Changes**:

1. Remove `#[allow(dead_code)]` from `pub(crate) fn vec_add_i16` (line 92).
2. Remove `#[allow(dead_code)]` from `pub(crate) fn vec_sub_i16` (line 137).
3. Remove `#[allow(dead_code)]` from `fn vec_add_scalar` (line 198).
4. Remove `#[allow(dead_code)]` from `fn vec_sub_scalar` (line 205).

**Exports**: No change — these functions remain `pub(crate)`.

### 3.3 `benches/nnue_bench.rs` (Create)

**Purpose**: Criterion benchmarks demonstrating SIMD NPS improvement over the scalar/baseline codepath.

**Key logic**:

The benchmark exercises the three hot operations targeted by this task:

1. **`bench_forward_pass`** — Measure the `forward()` function throughput. Creates a `Network` with deterministic non-zero weights and a pre-populated `Accumulator`, then benchmarks repeated `forward()` calls. This exercises clipped-ReLU and dot product SIMD paths.

2. **`bench_accumulator_add_remove`** — Measure `add_feature` and `remove_feature` throughput. Creates a `Network`, initializes an `Accumulator` from bias, then benchmarks repeated add/remove cycles on a set of feature indices. This exercises the vec_add/vec_sub SIMD paths.

3. **`bench_evaluate_position`** — Measure end-to-end NPS via `AccumulatorStack::evaluate`. Sets up startpos, creates an `AccumulatorStack`, refreshes it, then benchmarks the `evaluate` call. This exercises all three SIMD operations together (accumulator refresh uses add, forward pass uses clipped-ReLU and dot product).

4. **`bench_push_pop_cycle`** — Measure `push_move`/`pop` throughput. From startpos with a refreshed accumulator, benchmarks pushing a move (which triggers incremental delta updates via SIMD vec_add/vec_sub) and then popping.

The benchmark group uses Criterion's default configuration. Results can be compared between scalar (no feature flags) and SIMD (with `--features simd-avx2` or `simd-neon`) by running the benchmarks twice and using `critcmp` or Criterion's built-in comparison.

**Dependencies**: `criterion`, `chess-nnue`, `chess-board`, `chess-types`, `chess-movegen`.

### 3.4 `Cargo.toml` (Workspace root — Modify)

**Purpose**: Register the benchmark target and add criterion dev-dependency.

**Changes**: Add at the end of the file:

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
chess-nnue = { path = "crates/nnue" }
chess-board = { path = "crates/board" }
chess-types = { path = "crates/types" }
chess-movegen = { path = "crates/movegen" }

[[bench]]
name = "nnue_bench"
harness = false
```

## 4. API Contracts

N/A — No public API surface changes. The `add_feature`/`remove_feature` methods retain identical signatures and produce identical results. The SIMD dispatch is an internal implementation detail. The benchmark is a standalone binary with no exported API.

## 5. Test Plan

### 5.1 Existing tests validate correctness

The existing test suite in `accumulator.rs` (13 tests) and `simd.rs` (4 tests including `simd_matches_scalar`) already validate that:
- `add_feature`/`remove_feature` produce correct results (roundtrip test, refresh comparison tests)
- SIMD dispatch produces identical results to scalar fallback (`simd_matches_scalar` test)
- Incremental accumulator updates match full refresh across quiet moves, captures, en passant, castling, promotions, and long chains

Running these tests with SIMD feature flags enabled (`--features simd-avx2` on x86-64 or `--features simd-neon` on ARM) validates that the SIMD-wired accumulator produces identical results.

### 5.2 New tests: SIMD correctness through accumulator operations

Add one test to `accumulator.rs` `#[cfg(test)] mod tests`:

**`test: accumulator_add_remove_matches_scalar_reference`** — Manually computes the expected result of `add_feature` using a scalar loop, then compares with the actual `add_feature` result (which now uses SIMD dispatch). This directly validates that the SIMD wiring in the accumulator produces identical output to the scalar computation, complementing the existing `simd_matches_scalar` test which operates at the `vec_add_i16`/`vec_sub_i16` level.

```rust
#[test]
fn accumulator_add_remove_matches_scalar_reference() {
    let mut net = Network::new_zeroed();
    for (i, w) in net.input_weights.iter_mut().enumerate() {
        *w = (i % 256) as i16 - 128;
    }
    *net.input_bias = [0i16; L1_SIZE];

    let feature_idx = 100;
    let mut acc = Accumulator::new();
    acc.init_from_bias(&net.input_bias);

    // Compute expected result with scalar arithmetic
    let offset = feature_idx * L1_SIZE;
    let mut expected_white = acc.white;
    for i in 0..L1_SIZE {
        expected_white[i] += net.input_weights[offset + i];
    }

    // Use the actual add_feature (which now dispatches to SIMD)
    acc.add_feature(Color::White, feature_idx, &net.input_weights);
    assert_eq!(acc.white, expected_white, "SIMD add_feature must match scalar");

    // Now remove and verify we return to original
    acc.remove_feature(Color::White, feature_idx, &net.input_weights);
    assert_eq!(acc.white, *net.input_bias, "SIMD remove_feature must round-trip");
}
```

### 5.3 Benchmark-based NPS validation

The benchmark in `benches/nnue_bench.rs` provides the NPS comparison data. The verification procedure is:

1. Run benchmarks without SIMD: `cargo bench --bench nnue_bench` (scalar fallback)
2. Run benchmarks with SIMD: `cargo bench --bench nnue_bench --features chess-nnue/simd-avx2` (or `simd-neon` on ARM)
3. Compare throughput numbers — the SIMD-enabled run should show at least 1.5x improvement on the forward pass and accumulator operations.

## 6. Implementation Order

1. **`crates/nnue/src/simd.rs`** — Remove `#[allow(dead_code)]` from `vec_add_i16`, `vec_sub_i16`, `vec_add_scalar`, `vec_sub_scalar`.

2. **`crates/nnue/src/accumulator.rs`** — Add `use crate::simd;` import. Replace scalar loops in `add_feature` and `remove_feature` with `simd::vec_add_i16` and `simd::vec_sub_i16` calls. Add the `accumulator_add_remove_matches_scalar_reference` test.

3. **`Cargo.toml`** (workspace root) — Add `criterion` dev-dependency and `[[bench]]` configuration.

4. **`benches/nnue_bench.rs`** — Create the benchmark file with the four benchmark functions.

## 7. Verification Commands

```bash
# 1. Run all NNUE tests with default (scalar) features to verify no regression
cargo test -p chess-nnue

# 2. Run all NNUE tests with AVX2 SIMD enabled (x86-64)
RUSTFLAGS="-C target-feature=+avx2" cargo test -p chess-nnue --features simd-avx2

# 3. Run all NNUE tests with SSE2 SIMD enabled (x86-64)
RUSTFLAGS="-C target-feature=+sse2" cargo test -p chess-nnue --features simd-sse2

# 4. Run all NNUE tests with NEON SIMD enabled (ARM64 — only valid on aarch64)
# RUSTFLAGS="-C target-feature=+neon" cargo test -p chess-nnue --features simd-neon

# 5. Run clippy and fmt checks
cargo clippy -p chess-nnue -- -D warnings
cargo clippy -p chess-nnue --features simd-avx2 -- -D warnings
cargo fmt --check

# 6. Run the full workspace test suite to verify no integration regressions
cargo test --workspace

# 7. Run benchmarks (scalar baseline)
cargo bench --bench nnue_bench

# 8. Run benchmarks (SIMD-enabled, e.g. AVX2 on x86-64)
RUSTFLAGS="-C target-feature=+avx2" cargo bench --bench nnue_bench --features chess-nnue/simd-avx2

# 9. Compare benchmark results for 1.5x NPS improvement
# (Criterion outputs comparison automatically when baselines exist)
```

Note: On macOS ARM64 (aarch64-apple-darwin), the NEON feature is enabled by default. Use `--features simd-neon` for NEON benchmarks. Commands 2-3 are for x86-64 targets; command 4 is for ARM64 targets.

## 8. Design Deviations

**Deviation 1: No `simd/` subdirectory with separate files per architecture**

- **Parent spec requires**: The task spec's "Files to Create or Modify" section lists `crates/nnue/src/simd/` with separate `avx2.rs`, `neon.rs`, `sse2.rs` files.
- **Why that approach is problematic**: Phase P01 already established the SIMD organization as a single `simd.rs` file with `#[cfg]`-gated functions inline. All AVX2, SSE2, NEON, and scalar implementations already exist in `simd.rs` (578 lines). Splitting into a subdirectory would require restructuring working code from a completed prior phase, introducing module re-export changes in `lib.rs`, and breaking the established pattern — all without functional benefit.
- **Alternative chosen**: Maintain the existing `simd.rs` single-file structure established by Phase P01. The SIMD implementations already exist and are complete. The only modification needed is removing dead-code attributes and wiring the existing functions into `accumulator.rs`.

**Deviation 2: Benchmarks placed in workspace `benches/` rather than `crates/nnue/benches/`**

- **Parent spec requires**: The task spec lists `benches/` generically without specifying crate-level vs workspace-level.
- **Why workspace-level is correct**: The end-to-end NPS benchmark requires `AccumulatorStack::evaluate`, which calls into `chess_board::Position`, `chess_movegen`, and `chess_types` — cross-crate dependencies that are easier to express as a workspace-level benchmark. The workspace root `Cargo.toml` is the natural place for integration benchmarks that span multiple crates.
- **Alternative chosen**: Place the benchmark at workspace level (`benches/nnue_bench.rs`) with dev-dependencies declared in the workspace root `Cargo.toml`. This avoids adding multiple path dependencies to the `chess-nnue` crate's `Cargo.toml` that would only serve benchmarks.