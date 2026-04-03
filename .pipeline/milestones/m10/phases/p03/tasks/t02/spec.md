## T02 — SIMD Optimization

### Objective

Implement vectorized SIMD codepaths for the three hot operations in the NNUE inference engine: accumulator incremental updates (add/subtract feature vectors), clipped-ReLU activation, and dot products. Target x86-64 AVX2 and ARM NEON, with SSE2 as baseline fallback on x86-64.

### Deliverables

1. **SIMD implementations** behind Cargo feature flags (`avx2`, `sse2`, `neon`) in `crates/nnue` for:
   - Accumulator incremental updates (vectorized add/subtract of feature vectors)
   - Clipped-ReLU activation (vectorized clamp and propagation through hidden layers)
   - Dot products (vectorized output layer computation)
2. **`// SAFETY:` comments** on all `unsafe` blocks per project convention
3. **Scalar fallback preserved** — the non-SIMD codepath from Phase P01 must remain functional when no SIMD feature flag is enabled
4. **Benchmarks** demonstrating at least **1.5x NPS improvement** over the scalar/baseline codepath from Phase P01

### Files to Create or Modify

- `crates/nnue/src/simd/` — SIMD-specific modules (e.g., `avx2.rs`, `neon.rs`, `sse2.rs`)
- `crates/nnue/src/inference.rs` (or equivalent) — dispatch to SIMD codepaths based on feature flags
- `crates/nnue/Cargo.toml` — feature flag definitions (`avx2`, `sse2`, `neon`)
- `benches/` — NPS benchmarks comparing scalar vs SIMD codepaths

### Key Implementation Details

- **SIMD dispatch via Cargo feature flags** at compile time — `avx2`, `sse2`, `neon` — consistent with approach from Phase P01
- **AVX2** is the primary optimization target on x86-64; **SSE2** is baseline fallback; **NEON** is the ARM target
- Three operations to vectorize:
  1. Accumulator incremental updates: vectorized add/subtract of i16 feature vectors
  2. Clipped-ReLU: vectorized clamp (0 to quantization max) through hidden layers
  3. Dot products: vectorized multiply-accumulate for output layer
- Each `unsafe` block requires a `// SAFETY:` comment explaining why it is sound
- No `unsafe` without provable necessity — this is a project-wide convention
- The scalar fallback path must produce identical results to SIMD paths (bitwise identical for integer arithmetic)

### Dependencies

- **T01 (Accumulator Incremental Updates)** — the incremental update logic must exist before it can be SIMD-optimized
- **Phase P01 (NNUE Architecture)** — the inference engine, feature flag infrastructure, and scalar codepaths must be in place
- **Phase P02 (Training & Integration)** — a trained network must exist for benchmarking

### Verification Criteria

1. SIMD codepaths produce **identical** results to scalar fallback across all test positions
2. NPS improves by at least **1.5x** over the Phase P01 scalar/baseline codepath, measured via benchmarks
3. All three target architectures compile and pass tests: AVX2, SSE2 (x86-64), NEON (ARM)
4. Every `unsafe` block has a `// SAFETY:` comment
5. Scalar fallback works correctly when no SIMD feature flag is enabled
6. Elo does not regress compared to the non-optimized NNUE from P02 (validated via SPRT)
7. `cargo clippy -- -D warnings` and `cargo fmt --check` pass