## T03 — Network Distillation

### Objective

Train a smaller, faster NNUE network that approximates the full network using `bullet` or `nnue-pytorch`. Evaluate candidate architectures, measure Elo vs NPS tradeoffs, and select the network that achieves at least 95% of the full network's Elo at 1.5x or greater NPS improvement.

### Deliverables

1. **Distilled `.nnue` weights file** — a trained smaller network in the same binary format from Phase P01
2. **Elo vs NPS tradeoff documentation** with measured data for all four candidates (full network as control + three distilled variants)
3. **Justification** for the selected network size based on measured data

### Candidate Architectures

Evaluate four candidates (exact layer dimensions derived from the P01 architecture):
1. **Control**: the full network from Phase P02 (baseline)
2. **Half-width input layer variant**: e.g., if full network is 512→32→32→1, this is 256→32→32→1
3. **Reduced hidden layer variant**: e.g., 512→16→16→1 (or equivalent reduction)
4. **Combined reduction variant**: e.g., 256→16→16→1

### Files to Create or Modify

- `tools/` — distillation scripts/configuration for `bullet` or `nnue-pytorch`
- Distilled `.nnue` weights file (output artifact)
- Documentation file with Elo vs NPS tradeoff curve and selection justification
- The distilled network uses the **same inference codepath** from Phase P01 (differing only in layer dimensions), so `crates/nnue` must support parameterized layer sizes

### Key Implementation Details

- Uses the same external tooling (`bullet` or `nnue-pytorch`) from Phase P02 for training
- The distilled network uses the same `.nnue` file format and inference codepath — only layer dimensions differ
- `crates/nnue` must support loading networks with different layer dimensions (parameterized, not hardcoded)
- SPRT testing uses the M08 infrastructure (CuteChess-CLI) to measure Elo for each candidate
- NPS is measured via engine benchmarks under consistent conditions
- Selection criterion: **at least 95% of the full network's Elo** at **1.5x or greater NPS improvement**

### Dependencies

- **T01 (Accumulator Incremental Updates)** — incremental updates must be in place for accurate NPS measurement
- **T02 (SIMD Optimization)** — SIMD optimizations must be in place for accurate NPS measurement
- **Phase P01 (NNUE Architecture)** — inference engine and `.nnue` file format
- **Phase P02 (Training & Integration)** — the full trained network (control) and training pipeline
- **M08 (Testing, Tuning & Strength Validation)** — SPRT testing and CuteChess-CLI for Elo measurement
- **External tooling** — `bullet` or `nnue-pytorch` for training distilled networks

### Verification Criteria

1. At least three distilled candidate architectures are trained and evaluated
2. Each candidate's Elo is measured via SPRT testing with CuteChess-CLI
3. Each candidate's NPS is measured under consistent benchmark conditions
4. The selected network achieves **at least 95% of the full network's Elo** at **1.5x or greater NPS improvement**
5. Elo vs NPS tradeoff curve is documented with measured data points for all four candidates
6. The selected network loads and runs correctly through the existing inference codepath
7. Overall engine Elo does not regress compared to non-optimized NNUE from P02
8. `cargo test` passes, `cargo clippy -- -D warnings` and `cargo fmt --check` pass