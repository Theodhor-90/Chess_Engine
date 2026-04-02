## Phase P01 — NNUE Architecture

### Goal

Define the neural network structure, build the inference engine, and implement the weight-loading format. This phase establishes the foundational NNUE components: the network topology using HalfKP (or HalfKAv2) input features with an accumulator structure and quantized weights, a forward-pass inference engine accelerated with SIMD intrinsics (SSE2/AVX2/NEON), and a binary `.nnue` file format for loading trained network weights.

### Deliverables

- **T01**: NNUE network architecture — HalfKP (or HalfKAv2) input features, accumulator structure, quantized weights. Define the network topology (input → hidden1 → hidden2 → output).
- **T02**: Inference engine — forward pass with incremental accumulator updates. Use SIMD (SSE2/AVX2/NEON) intrinsics for vectorized integer arithmetic.
- **T03**: Network file format — load `.nnue` weights from binary file. Support Stockfish-compatible format or define custom format.

### Technical Decisions & Constraints

- Input feature set: HalfKP or HalfKAv2 (king-relative piece features)
- Network topology: input layer → hidden layer 1 → hidden layer 2 → output layer
- Weights are quantized for integer arithmetic during inference
- SIMD acceleration required: SSE2, AVX2 (x86-64), and NEON (ARM) intrinsics
- Network file format must be binary `.nnue` — either Stockfish-compatible or a documented custom format
- Incremental accumulator updates must be supported from the start in the inference engine design
- No `unsafe` blocks without `// SAFETY:` comments; SIMD intrinsics are an expected use of unsafe
- Core engine logic should minimize external dependencies per project conventions
- The new NNUE code will live under `crates/eval` or a new `crates/nnue` crate within the Cargo workspace

### Dependencies

- None within this milestone (this is the first phase)
- Depends on M08/M09 completed engine for eventual integration and testing
- Depends on existing `crates/types` for `Square`, `Piece`, `Color`, and other core types
- Depends on existing `crates/board` for `Position` representation used during inference