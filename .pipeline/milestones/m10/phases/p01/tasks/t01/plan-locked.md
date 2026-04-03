Now I have a thorough understanding of the codebase. Let me produce the task plan.

## 1. Deliverables

- **Create** `crates/nnue/Cargo.toml` — crate manifest for the new NNUE crate, declaring dependencies on `chess-types` and `chess-board`
- **Create** `crates/nnue/src/lib.rs` — crate root with module declarations and public re-exports
- **Create** `crates/nnue/src/arch.rs` — network topology constants: layer dimensions, feature set size, quantization parameters
- **Create** `crates/nnue/src/feature.rs` — HalfKP feature index computation mapping `(king_square, piece_square, piece_kind, piece_color)` to a unique feature index
- **Create** `crates/nnue/src/network.rs` — `Network` struct (weight/bias storage for all layers), `Accumulator` struct (incrementally updatable hidden layer 1 state)
- **Modify** root `Cargo.toml` — add `"crates/nnue"` to the `[workspace] members` list

## 2. Dependencies

- `chess-types = { path = "../types" }` — provides `Square`, `Piece`, `PieceKind`, `Color`
- `chess-board = { path = "../board" }` — provides `Position` (needed for accumulator initialization from a full position)
- No new external crates required. All types use standard library primitives (`i16`, `i8`, `i32`).

## 3. Implementation Details

### `crates/nnue/Cargo.toml`

```toml
[package]
name = "chess-nnue"
version = "0.1.0"
edition.workspace = true

[dependencies]
chess-types = { path = "../types" }
chess-board = { path = "../board" }
```

### `crates/nnue/src/arch.rs`

Network topology constants and quantization scheme documentation.

**Constants:**

- `NUM_SQUARES: usize = 64` — squares on the board
- `NUM_PIECE_KINDS: usize = 5` — non-king piece kinds (Pawn, Knight, Bishop, Rook, Queen) in HalfKP
- `NUM_COLORS: usize = 2` — piece colors
- `HALFKP_FEATURES: usize = NUM_SQUARES * NUM_PIECE_KINDS * NUM_COLORS * NUM_SQUARES` — total HalfKP features per perspective: 64 king squares × 10 colored non-king piece types × 64 piece squares = 40,960
- `L1_SIZE: usize = 256` — hidden layer 1 (accumulator) output dimension
- `L2_SIZE: usize = 32` — hidden layer 2 output dimension
- `OUTPUT_SIZE: usize = 1` — single scalar output
- `INPUT_SIZE: usize = HALFKP_FEATURES` — input feature dimensionality per perspective
- `QUANTIZATION_SHIFT_ACCUMULATOR: i32 = 6` — Q6 shift for accumulator weights (i16 scaled by 64)
- `QUANTIZATION_SHIFT_HIDDEN: i32 = 6` — Q6 shift for hidden-layer weights (i8 scaled by 64)
- `CLIPPED_RELU_MAX: i16 = 127` — clamp range for clipped-ReLU activation output

**Quantization scheme (documented in code comments):**
- Accumulator (input → L1): weights are `i16`, biases are `i16`. Real-valued weights are quantized by multiplying by `2^QUANTIZATION_SHIFT_ACCUMULATOR` (64) and rounding to the nearest integer. This preserves precision while enabling fast integer arithmetic.
- Hidden layers (L1 → L2, L2 → output): weights are `i8`, biases are `i32`. The smaller `i8` type reduces memory footprint and enables SIMD-friendly dot products. Biases are `i32` to hold accumulated products without overflow.
- Clipped-ReLU activation clamps values to `[0, CLIPPED_RELU_MAX]` (i.e., `[0, 127]`), fitting the result into an `i8` range for the next layer's input.

### `crates/nnue/src/feature.rs`

HalfKP feature index computation.

**Exports:**

- `feature_index(king_sq: Square, piece_sq: Square, piece_kind: PieceKind, piece_color: Color) -> usize` — computes a unique feature index within `0..HALFKP_FEATURES`. The mapping is: `king_sq.index() as usize * (NUM_PIECE_KINDS * NUM_COLORS * NUM_SQUARES) + colored_piece_index * NUM_SQUARES + piece_sq.index() as usize`, where `colored_piece_index = piece_color as usize * NUM_PIECE_KINDS + (piece_kind as usize)`. The function requires `piece_kind != PieceKind::King` (kings are not features in HalfKP); this is enforced by a debug assertion.
- `HalfKpFeature` — struct holding `king_sq`, `piece_sq`, `piece_kind`, `piece_color`; provides `index(&self) -> usize` as a convenience wrapper around `feature_index`.

**Design decision (documented in code):** HalfKP is chosen over HalfKAv2 because it is the most widely adopted NNUE feature set with well-understood training behavior, a smaller feature space (40,960 vs. larger for HalfKAv2), and broad compatibility with existing training tooling (`bullet`, `nnue-pytorch`). The reduced dimensionality means faster accumulator refresh and lower memory usage, which is appropriate for an initial NNUE implementation.

### `crates/nnue/src/network.rs`

Network and accumulator type definitions.

**Structs:**

- `Accumulator` — stores the incrementally updatable state for hidden layer 1.
  ```rust
  pub struct Accumulator {
      /// White perspective accumulator values (L1_SIZE elements).
      pub white: [i16; L1_SIZE],
      /// Black perspective accumulator values (L1_SIZE elements).
      pub black: [i16; L1_SIZE],
  }
  ```
  Methods:
  - `new() -> Accumulator` — creates a zeroed accumulator
  - `init_from_bias(bias: &[i16; L1_SIZE])` — sets both perspectives to the bias values (used when refreshing from scratch)
  - `add_feature(&mut self, perspective: Color, index: usize, weights: &[i16])` — adds the weight column at `index` to the given perspective's accumulator: `self.vals[i] += weights[index * L1_SIZE + i]` for `i in 0..L1_SIZE`
  - `remove_feature(&mut self, perspective: Color, index: usize, weights: &[i16])` — subtracts the weight column at `index` from the given perspective's accumulator

- `Network` — stores all network weights and biases.
  ```rust
  pub struct Network {
      /// Input → L1 weights. Stored as a flat array: HALFKP_FEATURES × L1_SIZE elements.
      /// Column-major: weights for feature `f` start at index `f * L1_SIZE`.
      pub input_weights: Box<[i16]>,
      /// L1 biases (L1_SIZE elements).
      pub input_bias: Box<[i16; L1_SIZE]>,
      /// L1 → L2 weights. L2_SIZE × (2 * L1_SIZE) elements.
      /// The input to this layer is the concatenation of white and black perspectives.
      pub hidden1_weights: Box<[i8]>,
      /// L2 biases (L2_SIZE elements).
      pub hidden1_bias: Box<[i32; L2_SIZE]>,
      /// L2 → output weights (OUTPUT_SIZE × L2_SIZE elements, i.e., L2_SIZE).
      pub hidden2_weights: Box<[i8; L2_SIZE]>,
      /// Output bias (OUTPUT_SIZE elements, i.e., 1).
      pub hidden2_bias: i32,
  }
  ```
  Methods:
  - `new_zeroed() -> Network` — allocates a network with all weights/biases zeroed (for testing)

### `crates/nnue/src/lib.rs`

Module declarations and re-exports.

```rust
pub mod arch;
pub mod feature;
pub mod network;

pub use arch::*;
pub use feature::{feature_index, HalfKpFeature};
pub use network::{Accumulator, Network};
```

### Root `Cargo.toml` modification

Add `"crates/nnue"` to the workspace `members` list, maintaining the existing ordering (library crates before tools, alphabetical within groups). Insert after `"crates/movegen"` (alphabetically `nnue` comes after `movegen` and before `search`).

## 4. API Contracts

```rust
use chess_nnue::{feature_index, Accumulator, Network, L1_SIZE, HALFKP_FEATURES};
use chess_types::{Square, PieceKind, Color};

// Compute a feature index
let idx = feature_index(Square::E1, Square::D4, PieceKind::Knight, Color::White);
assert!(idx < HALFKP_FEATURES);

// Initialize an accumulator
let net = Network::new_zeroed();
let mut acc = Accumulator::new();
acc.init_from_bias(&net.input_bias);

// Incremental update: add a feature
acc.add_feature(Color::White, idx, &net.input_weights);

// Incremental update: remove a feature
acc.remove_feature(Color::White, idx, &net.input_weights);
```

N/A for REST/HTTP.

## 5. Test Plan

All tests are colocated as `#[cfg(test)] mod tests` within the respective source files, following project conventions.

| Test | File | What it verifies |
|------|------|-----------------|
| `feature_index_in_range` | `feature.rs` | `feature_index` returns values within `0..HALFKP_FEATURES` for a sampling of valid inputs |
| `feature_index_unique` | `feature.rs` | Distinct `(king_sq, piece_sq, kind, color)` tuples produce distinct indices — test all 10 colored piece types on a fixed king square and distinct piece squares |
| `feature_index_varies_with_king_square` | `feature.rs` | Same piece on same square produces different feature indices when the king square differs |
| `accumulator_init_from_bias` | `network.rs` | After `init_from_bias`, both white and black arrays equal the bias vector |
| `accumulator_add_remove_roundtrip` | `network.rs` | Adding then removing the same feature returns the accumulator to its initial state |
| `network_new_zeroed` | `network.rs` | `Network::new_zeroed()` allocates with correct array lengths and all-zero values |

## 6. Implementation Order

1. `crates/nnue/Cargo.toml` — define crate manifest with dependencies
2. Root `Cargo.toml` — add `"crates/nnue"` to workspace members
3. `crates/nnue/src/arch.rs` — define all topology constants and quantization parameters
4. `crates/nnue/src/feature.rs` — implement HalfKP feature index computation with tests
5. `crates/nnue/src/network.rs` — implement `Network` and `Accumulator` structs with tests
6. `crates/nnue/src/lib.rs` — module declarations and re-exports

## 7. Verification Commands

```bash
cargo build -p chess-nnue
cargo test -p chess-nnue
cargo fmt --check -p chess-nnue
cargo clippy -p chess-nnue -- -D warnings
```

## 8. Design Deviations

**Deviation: `Network` uses `Box<[i16]>` for input weights instead of a fixed-size array.**

- **Spec requires**: Weight storage structures using quantized integer types. The natural choice is `[i16; HALFKP_FEATURES * L1_SIZE]` (a fixed-size array of 10,485,760 elements = ~20 MB).
- **Why problematic**: A 20 MB array on the stack would overflow. Even as a struct field, `[i16; 10_485_760]` in a `Box<Network>` requires the compiler to generate a stack temporary before boxing, which also overflows the stack in debug mode. Rust does not yet support guaranteed placement-new for large arrays.
- **Alternative chosen**: `Box<[i16]>` (heap-allocated slice) for `input_weights`, allocated via `vec![0i16; HALFKP_FEATURES * L1_SIZE].into_boxed_slice()`. This guarantees heap allocation without any stack intermediary. The length is validated at construction time. All other layers are small enough for fixed-size boxed arrays (e.g., `Box<[i16; L1_SIZE]>`).