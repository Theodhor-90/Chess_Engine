## Task T02 — Inference Engine with SIMD Forward Pass

### Objective

Implement the forward-pass inference engine that evaluates a position given the network weights. This includes accumulator initialization from a full position, incremental accumulator update support (add/remove features), clipped-ReLU activation, and SIMD-accelerated integer arithmetic for SSE2, AVX2, and NEON.

### Deliverables

- Working forward pass that takes a position (via accumulator state) and network weights, producing an integer evaluation score
- Accumulator initialization from a full `Position` (compute all active features, sum corresponding weight columns)
- Incremental accumulator update API — `add_feature` and `remove_feature` operations that adjust the accumulator without full recomputation
- Clipped-ReLU activation function implementation
- SIMD codepaths for SSE2, AVX2, and NEON behind Cargo feature flags
- Scalar fallback implementation for platforms without SIMD support
- SIMD level selection: AVX2 > SSE2 > scalar on x86-64; NEON > scalar on ARM

### Files to Create or Modify

- **Create** `crates/nnue/src/accumulator.rs` (or similar) — accumulator struct, initialization, incremental update methods
- **Create** `crates/nnue/src/inference.rs` (or similar) — forward pass logic, clipped-ReLU, layer propagation
- **Create** `crates/nnue/src/simd.rs` (or similar) — SIMD intrinsic wrappers for SSE2/AVX2/NEON, feature-gated
- **Modify** `crates/nnue/src/lib.rs` — add new module declarations
- **Modify** `crates/nnue/Cargo.toml` — add dependency on `crates/board` for `Position`, add feature flags for SIMD levels

### Implementation Details & Constraints

- Accumulator stores the result of the input layer computation as a vector of quantized integers (e.g., `i16`)
- Clipped-ReLU: `clamp(x, 0, max_val)` where `max_val` is determined by the quantization scheme from T01
- SIMD intrinsics require `unsafe` blocks — each must have a `// SAFETY:` comment explaining why it is sound
- Feature detection: compile-time via `#[cfg(target_feature = "avx2")]` or Cargo feature flags, or runtime via `std::is_x86_feature_detected!`
- The forward pass must propagate through all layers: accumulator → clipped-ReLU → hidden1 weights → clipped-ReLU → hidden2 weights → output
- Both perspectives (white and black accumulator halves) must be handled for the HalfKP feature set
- No `unwrap()` in library code; use `Result`/`Option` propagation

### Dependencies

- **T01** — requires the network architecture types, layer dimensions, feature index computation, and weight/bias storage structures defined in T01

### Verification Criteria

1. Forward pass produces a deterministic integer evaluation score for a given position and set of weights
2. Accumulator initialized from a full position produces the same result as manually summing all active feature weight columns
3. Incremental updates (add then remove a feature) return the accumulator to its original state
4. SIMD codepaths produce identical results to the scalar fallback for all test cases
5. `cargo test -p nnue` passes, covering forward pass correctness against known reference values
6. `cargo fmt --check` and `cargo clippy -- -D warnings` pass
7. Each `unsafe` block has a `// SAFETY:` comment