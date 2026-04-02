## Task T01 — NNUE Network Architecture Definition

### Objective

Define the complete NNUE network architecture: HalfKP (or HalfKAv2) input feature mapping, accumulator structure, quantized weight representation, and the full network topology (input → hidden1 → hidden2 → output).

### Deliverables

- Rust type definitions for all network layers (input/accumulator, hidden1, hidden2, output)
- Feature index computation function mapping (king_square, piece_square, piece_type, piece_color) to a feature index
- Weight and bias storage structures using quantized integer types
- Quantization scheme documentation (which integer types, scaling factors, etc.)
- Decision between HalfKP and HalfKAv2 feature set, documented with rationale based on dimensionality vs representational power tradeoff

### Files to Create or Modify

- **Create** `crates/nnue/` — new crate within the Cargo workspace
- **Create** `crates/nnue/Cargo.toml` — crate manifest with dependencies on `crates/types` (for `Square`, `Piece`, `Color`)
- **Create** `crates/nnue/src/lib.rs` — crate root, module declarations
- **Create** `crates/nnue/src/arch.rs` (or similar) — network topology constants, layer dimension definitions
- **Create** `crates/nnue/src/feature.rs` (or similar) — HalfKP/HalfKAv2 feature index computation
- **Create** `crates/nnue/src/network.rs` (or similar) — `Network`, `Accumulator`, weight/bias storage structs
- **Modify** root `Cargo.toml` — add `crates/nnue` as a workspace member

### Implementation Details & Constraints

- The crate depends on `crates/types` for `Square`, `Piece`, `Color` and on `crates/board` for `Position`
- Weights must be quantized to integer types for fast inference (e.g., `i16` for accumulator, `i8` for hidden layer weights)
- The accumulator structure must be designed from the start to support incremental updates (feature add/remove without full recomputation), even though full optimization is deferred to Phase P03
- Network topology is four layers: input → hidden1 → hidden2 → output
- Follow project conventions: no `unwrap()` in library code, `// SAFETY:` comments on any `unsafe`, `///` doc comments on public items
- Naming: `snake_case` functions/variables, `PascalCase` types, `SCREAMING_SNAKE` constants

### Dependencies

- None within this phase (this is the first task)
- External: `crates/types` and `crates/board` from prior milestones

### Verification Criteria

1. `crates/nnue` compiles as a workspace member (`cargo build -p nnue` succeeds)
2. Network architecture types are defined with correct layer dimensions and quantized weight types
3. Feature index computation correctly maps (king_square, piece_square, piece_type, piece_color) tuples to unique feature indices within the expected range
4. Unit tests in `crates/nnue` pass (`cargo test -p nnue`), covering feature index computation for representative inputs
5. `cargo fmt --check` and `cargo clippy -- -D warnings` pass for the new crate
6. Quantization scheme is documented in code comments