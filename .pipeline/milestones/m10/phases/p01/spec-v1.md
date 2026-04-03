## Phase P01 — NNUE Architecture

### Goal

Define the NNUE neural network structure using HalfKP (or HalfKAv2) king-relative input features with an accumulator and quantized weights, build an inference engine that performs forward passes accelerated by SIMD intrinsics (SSE2/AVX2/NEON) with incremental accumulator updates, and implement a binary `.nnue` file format for loading trained network weights.

### Design Decisions

1. **Crate placement**: NNUE code will reside in a new `crates/nnue` crate within the Cargo workspace, keeping the neural network logic separate from the handcrafted evaluation in `crates/eval`. This crate depends on `crates/types` for `Square`, `Piece`, `Color` and on `crates/board` for `Position`.

2. **Input feature set**: HalfKP (or HalfKAv2) king-relative piece features. The choice between the two will be finalized in T01 based on the tradeoff between feature dimensionality and representational power. Both map (king_square, piece_square, piece_type, piece_color) into a feature index.

3. **Network topology**: A four-layer architecture — input → hidden layer 1 → hidden layer 2 → output — where the input layer uses an accumulator structure that can be incrementally updated. Weights are quantized to integer types for fast inference.

4. **SIMD acceleration**: The inference engine uses `unsafe` SIMD intrinsics for SSE2/AVX2 on x86-64 and NEON on ARM. Each `unsafe` block includes a `// SAFETY:` comment per project convention. Feature detection at compile time (via Cargo feature flags) or runtime selects the appropriate codepath.

5. **Network file format**: Binary `.nnue` format. The decision to adopt Stockfish-compatible format or define a custom format will be finalized in T03, documented with a format specification including header, layer dimensions, and quantization parameters.

6. **Incremental accumulator design**: The accumulator structure supports incremental updates from the initial design, so that feature additions and removals from make/unmake can be applied without full recomputation. This is an architectural requirement of T02, with full optimization deferred to Phase P03.

### Tasks

**T01 — NNUE Network Architecture Definition**

Define the complete network architecture: HalfKP (or HalfKAv2) input feature mapping, accumulator structure, quantized weight representation, and the full network topology (input → hidden1 → hidden2 → output). Deliverables: Rust type definitions for the network layers, feature index computation, weight and bias storage structures, and quantization scheme documentation.

**T02 — Inference Engine with SIMD Forward Pass**

Implement the forward-pass inference engine that evaluates a position given the network weights. Includes accumulator initialization from a full position, incremental accumulator update support (add/remove features), clipped-ReLU activation, and SIMD-accelerated integer arithmetic for SSE2, AVX2, and NEON. Deliverables: working forward pass that produces an evaluation score, with SIMD codepaths behind feature flags and a scalar fallback.

**T03 — Network File Format and Weight Loading**

Implement the binary `.nnue` file loader that reads network weights from disk into the in-memory network structure. Define or adopt the file format (Stockfish-compatible or custom), including header validation, endianness handling, and layer dimension verification. Deliverables: `load` function that reads a `.nnue` file and returns a populated network structure, with error handling for malformed or incompatible files.

### Exit Criteria

1. NNUE network architecture is defined with HalfKP or HalfKAv2 input features, accumulator structure, and quantized weights, with the full topology (input → hidden1 → hidden2 → output) implemented as Rust types
2. Inference engine performs forward passes using SIMD intrinsics (SSE2/AVX2/NEON) with a scalar fallback, producing an integer evaluation score from a position
3. Incremental accumulator updates are supported — features can be added and removed without full recomputation
4. Network weights load correctly from `.nnue` binary files with validation of file integrity, layer dimensions, and format compatibility
5. `cargo test` passes for the `nnue` crate, covering feature index computation, forward pass correctness against a known reference, and weight loading from a test fixture

### Dependencies

- `crates/types` — `Square`, `Piece`, `Color`, and related core types used for feature index computation
- `crates/board` — `Position` representation used to initialize the accumulator from a full position

### Artifacts

- `crates/nnue/` — new crate containing all NNUE architecture, inference, and weight-loading code
- `crates/nnue/Cargo.toml` — crate manifest with dependencies on `types` and `board`, and feature flags for SIMD levels
- `crates/nnue/src/lib.rs` — crate root
- `crates/nnue/src/arch.rs` — network architecture types, feature mapping, and topology definitions (T01)
- `crates/nnue/src/inference.rs` — forward pass, accumulator, SIMD kernels (T02)
- `crates/nnue/src/format.rs` — `.nnue` file format parsing and weight loading (T03)
- Updated root `Cargo.toml` — workspace member addition for `crates/nnue`