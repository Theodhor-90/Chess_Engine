Now I have complete context. Let me produce the revised specification.

## Milestone M10 — NNUE Evaluation

### Goal

Replace or augment the handcrafted evaluation with an efficiently updatable neural network (NNUE) to achieve a major strength jump, targeting 2500+ Elo. This milestone covers defining the network architecture, building the inference engine with SIMD acceleration, loading network weights from binary files, generating training data via self-play, training the network using external tooling, integrating NNUE evaluation into the search with a UCI-selectable evaluation mode, and optimizing inference through incremental accumulator updates, vectorized operations, and network distillation.

### Scope

- NNUE network architecture definition with HalfKP or HalfKAv2 input features, accumulator structure, and quantized weights
- Network topology: input layer → hidden layer 1 → hidden layer 2 → output layer
- Inference engine implementing forward pass with incremental accumulator updates
- SIMD intrinsics (SSE2/AVX2/NEON) for vectorized integer arithmetic during inference
- Binary `.nnue` network file format — support Stockfish-compatible format or define custom format
- Training data generation via self-play using the handcrafted engine, exporting FEN + eval + game result
- Training pipeline using `bullet` or `nnue-pytorch` on generated data, with iterative generate → train → embed → test → repeat workflow
- Evaluation switchover — NNUE evaluation replaces handcrafted eval in search; handcrafted eval remains as fallback; UCI option to select evaluation mode
- Accumulator incremental updates — only recompute affected features on make/unmake, tracking dirty accumulators to minimize recomputation
- SIMD optimization — vectorized clipped-ReLU, dot products, and accumulator updates for x86-64 AVX2 and ARM NEON
- Network distillation — train a smaller, faster network approximating the larger one, measuring Elo vs NPS tradeoff, shipping the best balance

### Phases

#### Phase P01 — NNUE Architecture

Define the neural network structure, build the inference engine, and implement the weight-loading format.

- **T01**: NNUE network architecture — HalfKP (or HalfKAv2) input features, accumulator structure, quantized weights. Define the network topology (input → hidden1 → hidden2 → output).
- **T02**: Inference engine — forward pass with incremental accumulator updates. Use SIMD (SSE2/AVX2/NEON) intrinsics for vectorized integer arithmetic.
- **T03**: Network file format — load `.nnue` weights from binary file. Support Stockfish-compatible format or define custom format.

#### Phase P02 — Training & Integration

Generate training data, train the network with external tooling, and integrate NNUE evaluation into the engine's search loop.

- **T01**: Training data generation — use the handcrafted engine to generate self-play games with position evaluations. Export as training data (FEN + eval + game result).
- **T02**: Training pipeline — use `bullet` or `nnue-pytorch` to train the network on generated data. Iterate: generate data → train → embed → test → repeat.
- **T03**: Evaluation switchover — NNUE evaluation replaces handcrafted eval in search. Handcrafted eval remains as fallback. UCI option to select evaluation mode.

#### Phase P03 — NNUE Optimization

Maximize inference speed through incremental computation, SIMD vectorization, and network size optimization.

- **T01**: Accumulator incremental updates — only recompute affected features on make/unmake. Track dirty accumulators to minimize recomputation.
- **T02**: SIMD optimization — vectorized clipped-ReLU, dot products, and accumulator updates for target architectures (x86-64 AVX2, ARM NEON).
- **T03**: Network distillation — train a smaller, faster network that approximates the larger network. Measure Elo vs NPS tradeoff. Ship the best balance.

### Exit Criteria

1. NNUE network architecture is defined with HalfKP or HalfKAv2 input features, accumulator structure, and quantized weights
2. Inference engine performs forward passes with incremental accumulator updates using SIMD intrinsics
3. Network weights load correctly from `.nnue` binary files
4. Training data generation produces self-play data with position evaluations exported as FEN + eval + game result
5. Training pipeline using `bullet` or `nnue-pytorch` produces a trained network that evaluates positions more accurately than the handcrafted eval
6. NNUE evaluation is integrated into search with a UCI option to select evaluation mode; handcrafted eval remains as fallback
7. Accumulator incremental updates only recompute affected features on make/unmake, tracking dirty accumulators
8. SIMD optimizations (AVX2, NEON) are implemented for clipped-ReLU, dot products, and accumulator updates
9. Network distillation produces a network that achieves at least 95% of the full network's Elo at 1.5x or greater NPS improvement, validated via SPRT testing; the Elo vs NPS tradeoff curve is documented and the selected network is justified against measured data
10. Engine plays at ~2500+ Elo level

### Dependencies

- **M08 (Testing, Tuning & Strength Validation)** — a working, tuned engine with self-play framework, SPRT testing, and CuteChess-CLI integration is required for training data generation and strength validation
- **M09 (Opening Book & Endgame Tablebases)** — completed engine with opening book and tablebase support provides the baseline engine used for self-play data generation
- **External tooling** — `bullet` or `nnue-pytorch` for network training (these are consumed, not built)
- **crates/eval** — the existing handcrafted evaluation (from M03/M07) that NNUE replaces and that remains as fallback
- **crates/search** — the search framework (from M03/M05/M06) into which NNUE evaluation is integrated