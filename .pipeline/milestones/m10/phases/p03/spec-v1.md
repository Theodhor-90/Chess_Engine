Now I have all the context needed. Let me produce the phase specification.

## Phase P03 — NNUE Optimization

### Goal

Maximize NNUE inference speed through three optimization strategies: incremental accumulator computation that only recomputes affected features on make/unmake moves, SIMD vectorization of clipped-ReLU activations, dot products, and accumulator updates for x86-64 AVX2 and ARM NEON architectures, and network distillation to produce a smaller, faster network that approximates the full network with a favorable Elo-vs-NPS tradeoff.

### Design Decisions

1. **Dirty accumulator tracking**: Maintain per-accumulator dirty flags that record which side's accumulator has been invalidated by a make/unmake operation. On `make_move`, mark the affected side's accumulator dirty and record the feature additions and removals (king-relative piece changes). On `unmake_move`, restore the previous accumulator state. Only recompute an accumulator when it is accessed and its dirty flag is set, applying the minimal delta (feature add/subtract) rather than a full recomputation from the position.

2. **SIMD dispatch via feature flags**: Cargo feature flags (`avx2`, `sse2`, `neon`) select SIMD codepaths at compile time, consistent with the approach established in Phase P01. AVX2 is the primary optimization target on x86-64, with SSE2 as baseline fallback; NEON is the ARM target. Each `unsafe` SIMD block includes a `// SAFETY:` comment per project convention.

3. **Vectorized operations scope**: SIMD optimization covers three operations — accumulator incremental updates (add/subtract feature vectors), clipped-ReLU activation (clamp and propagate through hidden layers), and dot products (output layer computation). These are the hot paths identified in the inference forward pass from Phase P01.

4. **Network distillation via external tooling**: Distillation uses `bullet` or `nnue-pytorch` (the same external tooling from Phase P02) to train a smaller network that approximates the full network's output. The distilled network uses the same file format (`.nnue`) and inference codepath from Phase P01, differing only in layer dimensions.

5. **Elo vs NPS validation**: The distilled network is validated via SPRT testing from the M08 infrastructure. The selection criterion is at least 95% of the full network's Elo at 1.5x or greater NPS improvement. The Elo vs NPS tradeoff curve is documented with measured data points justifying the selected network.

### Tasks

**T01 — Accumulator Incremental Updates**

Optimize the accumulator update path so that make/unmake operations only recompute affected features rather than performing a full accumulator recomputation. Implement dirty accumulator tracking that records which accumulators need recomputation and what feature deltas (additions and removals) to apply. Deliverables: dirty flag tracking mechanism in `crates/nnue`, incremental add/subtract feature updates on make/unmake, and tests verifying that incrementally updated accumulators produce identical results to full recomputation across a suite of positions.

**T02 — SIMD Optimization**

Implement vectorized SIMD codepaths for the three hot operations in the inference engine: accumulator incremental updates (add/subtract feature vectors), clipped-ReLU activation, and dot products. Target x86-64 AVX2 and ARM NEON, with SSE2 as baseline fallback on x86-64. Deliverables: SIMD implementations behind Cargo feature flags in `crates/nnue`, `// SAFETY:` comments on all `unsafe` blocks, scalar fallback preserved, and benchmarks demonstrating NPS improvement over the scalar/baseline codepath from Phase P01.

**T03 — Network Distillation**

Train a smaller, faster NNUE network that approximates the full network using `bullet` or `nnue-pytorch`. Evaluate multiple candidate network sizes, measure Elo (via SPRT testing with CuteChess-CLI from M08) and NPS for each, and select the network that achieves at least 95% of the full network's Elo at 1.5x or greater NPS improvement. Deliverables: a distilled `.nnue` weights file, documented Elo vs NPS tradeoff curve with measured data, and justification for the selected network size.

### Exit Criteria

1. Accumulator incremental updates only recompute affected features on make/unmake, tracking dirty accumulators to minimize recomputation
2. Incrementally updated accumulators produce identical evaluation results to full recomputation
3. SIMD optimizations (AVX2, NEON) are implemented for clipped-ReLU, dot products, and accumulator updates, with SSE2 baseline fallback on x86-64
4. NPS improves measurably over the Phase P01 scalar/baseline inference codepath
5. Network distillation produces a network that achieves at least 95% of the full network's Elo at 1.5x or greater NPS improvement, validated via SPRT testing
6. Elo vs NPS tradeoff curve is documented with measured data and the selected network is justified
7. Engine plays at ~2500+ Elo level after all optimizations

### Dependencies

- **Phase P01 (NNUE Architecture)** — the inference engine architecture, SIMD feature flag infrastructure, accumulator structure, and `.nnue` weight-loading format must be in place
- **Phase P02 (Training & Integration)** — a trained, integrated NNUE network must exist before optimization and distillation work can begin
- **M08 (Testing, Tuning & Strength Validation)** — SPRT testing and CuteChess-CLI integration required for measuring Elo impact of optimizations and validating the distilled network
- **External tooling** — `bullet` or `nnue-pytorch` for training the distilled network

### Artifacts

- Modified `crates/nnue` — dirty accumulator tracking, incremental update logic, SIMD-optimized codepaths for AVX2, NEON, and SSE2
- Distilled `.nnue` weights file
- Elo vs NPS tradeoff documentation