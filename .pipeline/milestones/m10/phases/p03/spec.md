## Phase P03 — NNUE Optimization

### Goal

Maximize NNUE inference speed through three optimization strategies: incremental accumulator computation that only recomputes affected features on make/unmake moves, SIMD vectorization of clipped-ReLU activations, dot products, and accumulator updates for x86-64 AVX2 and ARM NEON architectures, and network distillation to produce a smaller, faster network that approximates the full network with a favorable Elo-vs-NPS tradeoff.

### Deliverables

- **T01**: Accumulator incremental updates — only recompute affected features on make/unmake. Track dirty accumulators to minimize recomputation.
- **T02**: SIMD optimization — vectorized clipped-ReLU, dot products, and accumulator updates for target architectures (x86-64 AVX2, ARM NEON).
- **T03**: Network distillation — train a smaller, faster network that approximates the larger network. Measure Elo vs NPS tradeoff. Ship the best balance.

### Technical Decisions & Constraints

- Dirty accumulator tracking: maintain flags/state to identify which accumulators need recomputation after make/unmake
- Only affected features are recomputed — no full recomputation on every move
- Target SIMD architectures: x86-64 AVX2 and ARM NEON (SSE2 as baseline fallback)
- Vectorized operations: clipped-ReLU activation, dot products, accumulator add/subtract updates
- `unsafe` blocks required for SIMD intrinsics — each must have a `// SAFETY:` comment per project conventions
- Feature flags for SIMD level selection (SSE2, AVX2, NEON) to match the CI cross-compilation targets from M11
- Network distillation: the distilled network must achieve at least 95% of the full network's Elo at 1.5x or greater NPS improvement (exit criterion)
- Elo vs NPS tradeoff curve must be documented; the selected network must be justified against measured data
- Validation via SPRT testing from M08 infrastructure
- Target overall engine strength: ~2500+ Elo after all optimizations

### Dependencies

- **Phase P01 (NNUE Architecture)** — the inference engine architecture and weight format must be in place
- **Phase P02 (Training & Integration)** — a trained, integrated NNUE network must exist before optimization work can begin
- **M08 (Testing, Tuning & Strength Validation)** — SPRT testing and CuteChess-CLI integration required for measuring Elo impact of optimizations
- **External tooling** — `bullet` or `nnue-pytorch` for training the distilled network