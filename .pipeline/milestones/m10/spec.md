## Milestone M10 — NNUE Evaluation

### Goal

Replace or augment the handcrafted evaluation with a neural network for a major strength jump. Target: ~2500+ Elo.

### Phases

#### Phase P01 — NNUE Architecture
- **T01**: NNUE network architecture — HalfKP (or HalfKAv2) input features, accumulator structure, quantized weights. Define the network topology (input → hidden1 → hidden2 → output).
- **T02**: Inference engine — forward pass with incremental accumulator updates. Use SIMD (SSE2/AVX2/NEON) intrinsics for vectorized integer arithmetic.
- **T03**: Network file format — load `.nnue` weights from binary file. Support Stockfish-compatible format or define custom format.

#### Phase P02 — Training & Integration
- **T01**: Training data generation — use the handcrafted engine to generate self-play games with position evaluations. Export as training data (FEN + eval + game result).
- **T02**: Training pipeline — use `bullet` or `nnue-pytorch` to train the network on generated data. Iterate: generate data → train → embed → test → repeat.
- **T03**: Evaluation switchover — NNUE evaluation replaces handcrafted eval in search. Handcrafted eval remains as fallback. UCI option to select evaluation mode.

#### Phase P03 — NNUE Optimization
- **T01**: Accumulator incremental updates — only recompute affected features on make/unmake. Track dirty accumulators to minimize recomputation.
- **T02**: SIMD optimization — vectorized clipped-ReLU, dot products, and accumulator updates for target architectures (x86-64 AVX2, ARM NEON).
- **T03**: Network distillation — train a smaller, faster network that approximates the larger network. Measure Elo vs NPS tradeoff. Ship the best balance.

### Exit Criteria
- NNUE network architecture is defined with HalfKP or HalfKAv2 input features and quantized weights
- Inference engine performs forward passes with incremental accumulator updates using SIMD intrinsics
- Network weights load correctly from `.nnue` binary files
- Training data generation produces sufficient self-play data with position evaluations
- Training pipeline produces a trained network that evaluates positions more accurately than handcrafted eval
- NNUE evaluation is integrated into search with UCI option to select evaluation mode
- Accumulator incremental updates minimize recomputation on make/unmake
- SIMD optimizations (AVX2, NEON) are implemented for critical inference operations
- Network distillation produces an optimally-sized network balancing Elo and NPS
- Engine plays at ~2500+ Elo level
- All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`