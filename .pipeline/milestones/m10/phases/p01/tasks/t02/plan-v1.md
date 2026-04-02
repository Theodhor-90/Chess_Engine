No existing SIMD code in the source. Now I have all the context I need to produce the implementation plan.

## 1. Deliverables

| # | Action | File | Purpose |
|---|--------|------|---------|
| 1 | **Create** | `crates/nnue/src/accumulator.rs` | Accumulator struct with initialization from `Position`, incremental `add_feature`/`remove_feature` methods, and full refresh logic |
| 2 | **Create** | `crates/nnue/src/inference.rs` | Forward-pass logic: clipped-ReLU activation, layer propagation (accumulator → L1 → L2 → output), producing an `i32` evaluation score |
| 3 | **Create** | `crates/nnue/src/simd.rs` | SIMD intrinsic wrappers for vectorized clipped-ReLU, accumulator add/sub, and dot-product operations; codepaths for AVX2, SSE2, NEON, and scalar fallback behind `cfg(target_feature)` gates |
| 4 | **Modify** | `crates/nnue/src/lib.rs` | Add `mod accumulator;`, `mod inference;`, `mod simd;` declarations and public re-exports |
| 5 | **Modify** | `crates/nnue/Cargo.toml` | Add `chess-board` dependency (already present — verify it suffices), add Cargo feature flags for SIMD levels (`simd-avx2`, `simd-sse2`, `simd-neon`) |
| 6 | **Modify** | `crates/nnue/src/network.rs` | Move `Accumulator` out to new `accumulator.rs` module to separate concerns (the accumulator logic becomes more complex with position initialization) |

## 2. Dependencies

- **`chess-types`** (already declared in `crates/nnue/Cargo.toml`) — provides `Color`, `PieceKind`, `Piece`, `Square`, `Bitboard`
- **`chess-board`** (already declared in `crates/nnue/Cargo.toml`) — provides `Position` for accumulator initialization
- **No new external crates** — SIMD intrinsics come from `std::arch` (part of Rust std library)
- **Rust stable toolchain** — `std::arch::x86_64` (SSE2/AVX2) and `std::arch::aarch64` (NEON) are stable

## 3. Implementation Details

### 3.1 `crates/nnue/src/accumulator.rs`

**Purpose:** Encapsulates the incrementally updatable hidden layer 1 state with position-aware initialization.

**Moved from `network.rs`:** The existing `Accumulator` struct and its methods (`new`, `init_from_bias`, `add_feature`, `remove_feature`, `Default` impl) move here verbatim. The `network.rs` file retains only `Network`.

**New functionality:**

- **`Accumulator::refresh(pos: &Position, network: &Network)`** — Initializes the accumulator from a full `Position`:
  1. Call `init_from_bias(&network.input_bias)` to set both perspectives to the L1 bias vector.
  2. For each color perspective (`White`, `Black`):
     - Determine the king square for that perspective: `pos.piece_bitboard(Piece::new(perspective, PieceKind::King)).lsb()`, convert to `Square`.
     - Iterate all 10 non-king piece types (5 kinds × 2 colors). For each piece type, iterate the bitboard `pos.piece_bitboard(piece)`.
     - For each occupied square, compute `feature_index(king_sq, piece_sq, piece_kind, piece_color)` and call `add_feature(perspective, index, &network.input_weights)`.

**Exports:** `Accumulator` (struct), `Accumulator::new`, `Accumulator::init_from_bias`, `Accumulator::add_feature`, `Accumulator::remove_feature`, `Accumulator::refresh`.

### 3.2 `crates/nnue/src/inference.rs`

**Purpose:** Forward pass through the network layers, producing an evaluation score.

**Key function:**

```rust
pub fn forward(acc: &Accumulator, network: &Network, side_to_move: Color) -> i32
```

**Logic:**

1. **Concatenate perspectives:** Based on `side_to_move`, construct the L1 input of size `2 * L1_SIZE`:
   - If `White` to move: `[acc.white, acc.black]` (own perspective first, then opponent)
   - If `Black` to move: `[acc.black, acc.white]`

2. **Clipped-ReLU on L1 input:** Apply `clipped_relu` to each of the `2 * L1_SIZE` values from the accumulator, clamping to `[0, CLIPPED_RELU_MAX]` (i.e., `[0, 127]`). Output type: `i8` (fits in 0..127).

3. **L1 → L2 propagation:** Matrix-vector multiply `network.hidden1_weights` (shape `L2_SIZE × (2 * L1_SIZE)`, `i8`) by the clipped-ReLU output (`i8` vector of length `2 * L1_SIZE`). Accumulate into `i32` result, add `network.hidden1_bias`. This produces `L2_SIZE` `i32` values.

4. **Clipped-ReLU on L2:** Apply clipped-ReLU to the L2 values (after right-shifting by `QUANTIZATION_SHIFT_HIDDEN` to rescale), clamp to `[0, 127]`, cast to `i8`.

5. **L2 → Output propagation:** Dot product of `network.hidden2_weights` (`i8`, length `L2_SIZE`) with the L2 output (`i8`), accumulate into `i32`, add `network.hidden2_bias`.

6. **Return** the final `i32` score (right-shifted by `QUANTIZATION_SHIFT_HIDDEN` to convert from quantized domain back to centipawn-scale).

**Helper function:**

```rust
fn clipped_relu(input: &[i16], output: &mut [i8])
```

Clamps each `i16` value to `[0, CLIPPED_RELU_MAX]` and truncates to `i8`. Uses SIMD when available (delegates to `simd::` functions).

**Exports:** `forward` function.

### 3.3 `crates/nnue/src/simd.rs`

**Purpose:** SIMD-accelerated operations behind compile-time feature gates.

**Architecture:**

The module uses `#[cfg(target_arch = "x86_64")]` / `#[cfg(target_arch = "aarch64")]` gates and within each architecture block, uses `#[cfg(target_feature = "avx2")]` / `#[cfg(target_feature = "sse2")]` / `#[cfg(target_feature = "neon")]` for specific instruction sets. Each SIMD function has a scalar fallback.

**Functions to implement:**

1. **`clipped_relu_i16_to_i8(input: &[i16], output: &mut [i8])`** — Vectorized clipped-ReLU converting `i16` → `i8`:
   - AVX2: Load 16×i16 via `_mm256_loadu_si256`, clamp with `_mm256_max_epi16` (floor at 0) and `_mm256_min_epi16` (cap at 127), pack to `i8` with `_mm256_packs_epi16`.
   - SSE2: Same pattern with 128-bit `_mm_` intrinsics (8×i16 per iteration).
   - NEON: `vld1q_s16`, `vmaxq_s16`, `vminq_s16`, `vqmovn_s16`.
   - Scalar: Simple loop with `.clamp(0, CLIPPED_RELU_MAX) as i8`.

2. **`dot_i8_i32(a: &[i8], b: &[i8]) -> i32`** — Vectorized dot product of two `i8` slices, accumulated into `i32`:
   - AVX2: `_mm256_maddubs_epi16` + horizontal sum.
   - SSE2: `_mm_maddubs_epi16` + horizontal sum.
   - NEON: `vmull_s8` + `vpadalq_s16` + horizontal sum.
   - Scalar: `a.iter().zip(b).map(|(&x, &y)| (x as i32) * (y as i32)).sum()`.

3. **`vec_add_i16(acc: &mut [i16], weights: &[i16])`** — Vectorized element-wise add of weight column into accumulator:
   - AVX2: `_mm256_add_epi16` on 16 elements at a time.
   - SSE2: `_mm_add_epi16` on 8 elements at a time.
   - NEON: `vaddq_s16`.
   - Scalar: `acc[i] += weights[i]`.

4. **`vec_sub_i16(acc: &mut [i16], weights: &[i16])`** — Same as above but subtraction.

**Safety:** Every `unsafe` block gets a `// SAFETY:` comment explaining:
- The alignment/length preconditions are met (L1_SIZE = 256 is divisible by 16 and 32).
- The target feature is guaranteed by the `cfg` gate.

### 3.4 `crates/nnue/src/lib.rs` (Modify)

Add module declarations:

```rust
pub mod accumulator;
pub mod inference;
pub mod simd;
```

Update re-exports:
- Remove `Accumulator` from `network` re-export.
- Add `pub use accumulator::Accumulator;`
- Add `pub use inference::forward;`

### 3.5 `crates/nnue/Cargo.toml` (Modify)

Add Cargo feature flags:

```toml
[features]
default = []
simd-avx2 = []
simd-sse2 = []
simd-neon = []
```

These features are used with `#[cfg(feature = "simd-avx2")]` gates in `simd.rs`. The actual SIMD codepaths also require `#[cfg(target_feature = "avx2")]` (or compile with `-C target-feature=+avx2`), so the feature flags and target-feature checks work together — the feature flag enables the code, and the `target_feature` gate ensures the intrinsics are valid.

**Note:** The `chess-board` dependency is already declared. No changes needed for dependencies.

### 3.6 `crates/nnue/src/network.rs` (Modify)

Remove the `Accumulator` struct, its `impl` block, its `Default` impl, and its tests (`accumulator_init_from_bias`, `accumulator_add_remove_roundtrip`) from this file. These move to `accumulator.rs`. The `use chess_types::Color;` import is also removed (it was only used by `Accumulator`). Only `Network` and `Network::new_zeroed` remain.

## 4. API Contracts

### `Accumulator::refresh`

```rust
pub fn refresh(&mut self, pos: &Position, network: &Network)
```

- **Input:** A `&Position` (any valid chess position) and a `&Network` with loaded weights.
- **Output:** `self` is mutated — `self.white` and `self.black` are set to the accumulator state representing all active HalfKP features in the given position.
- **Example:** Given `Position::startpos()`, the white perspective accumulator sums weight columns for all 30 non-king pieces (16 pawns + 4 knights + 4 bishops + 4 rooks + 2 queens) relative to the white king on E1; the black perspective does the same relative to the black king on E8.

### `forward`

```rust
pub fn forward(acc: &Accumulator, network: &Network, side_to_move: Color) -> i32
```

- **Input:** A populated `Accumulator`, a `Network` with loaded weights, and the `Color` side to move.
- **Output:** An `i32` evaluation score in centipawn-scale (positive = good for side to move).
- **Example:** Given a zeroed network, returns 0 for any position. Given non-trivial weights, returns a deterministic integer for a given position.

### SIMD functions (module-internal)

```rust
pub(crate) fn clipped_relu_i16_to_i8(input: &[i16], output: &mut [i8])
pub(crate) fn dot_i8_i32(a: &[i8], b: &[i8]) -> i32
pub(crate) fn vec_add_i16(acc: &mut [i16], weights: &[i16])
pub(crate) fn vec_sub_i16(acc: &mut [i16], weights: &[i16])
```

- **Preconditions:** Slice lengths must match the corresponding architecture constants (`L1_SIZE`, `2 * L1_SIZE`, `L2_SIZE`). These are enforced by `debug_assert!`.
- **Invariant:** SIMD and scalar codepaths produce identical results for all valid inputs.

## 5. Test Plan

All tests are colocated in their source files via `#[cfg(test)] mod tests`.

### 5.1 `accumulator.rs` tests

**Test: `accumulator_init_from_bias`** (moved from `network.rs`)
Verifies `init_from_bias` sets both perspectives to the given bias values.

**Test: `accumulator_add_remove_roundtrip`** (moved from `network.rs`)
Verifies adding then removing the same feature returns accumulator to its original state. Satisfies verification criterion 3.

**Test: `accumulator_refresh_startpos`**
- Create a `Network` with deterministic non-zero input weights (e.g., `input_weights[i] = (i % 256) as i16 - 128`).
- Create a `Position::startpos()`.
- Call `Accumulator::refresh` to populate the accumulator.
- Manually compute the expected accumulator by iterating all pieces in startpos, computing `feature_index` for each, and summing the corresponding weight columns plus bias.
- Assert `acc.white == expected_white` and `acc.black == expected_black`.
- Satisfies verification criterion 2.

### 5.2 `inference.rs` tests

**Test: `forward_zeroed_network`**
- Create `Network::new_zeroed()` and `Accumulator::new()`.
- Call `forward(&acc, &network, Color::White)`.
- Assert result is `0` (all weights/biases zero → output is zero).
- Satisfies verification criterion 1 (deterministic score).

**Test: `forward_known_reference`**
- Create a `Network` with small, hand-chosen weights and biases for all layers (e.g., all weights = 1, biases = 0, or a specific pattern).
- Create an `Accumulator` with known L1 values (set directly, e.g., `acc.white[i] = i as i16` for `i in 0..L1_SIZE`).
- Compute the expected output by hand:
  1. Apply clipped-ReLU to `[acc.white, acc.black]` → get `i8` vector.
  2. Multiply by hidden1_weights, add hidden1_bias.
  3. Apply clipped-ReLU → get `i8` vector.
  4. Dot product with hidden2_weights, add hidden2_bias.
  5. Final shift.
- Assert `forward()` matches the hand-computed value.
- Satisfies verification criteria 1 and 5.

**Test: `forward_perspective_swap`**
- Verify that swapping `side_to_move` from White to Black with swapped accumulator perspectives produces a consistent (negated or equivalent) result, confirming the perspective concatenation logic is correct.

### 5.3 `simd.rs` tests

**Test: `clipped_relu_scalar_reference`**
- Generate a test vector of `i16` values spanning `[-500, 500]`.
- Run through scalar `clipped_relu_i16_to_i8`.
- Verify each output is `clamp(input, 0, 127) as i8`.

**Test: `dot_product_scalar_reference`**
- Create two `i8` slices with known values.
- Verify `dot_i8_i32` returns the correct sum of element-wise products.

**Test: `simd_matches_scalar`** (conditional on target features)
- For each SIMD function, run both the SIMD and scalar codepaths on the same input.
- Assert identical results.
- Satisfies verification criterion 4.

## 6. Implementation Order

1. **Modify `crates/nnue/Cargo.toml`** — Add the `[features]` section with SIMD feature flags.

2. **Create `crates/nnue/src/simd.rs`** — Implement the four SIMD wrapper functions with scalar fallbacks. These are leaf functions with no dependencies on other new code. Include unit tests.

3. **Create `crates/nnue/src/accumulator.rs`** — Move `Accumulator` from `network.rs`, add `refresh` method. Import `feature_index` from `crate::feature`, `Position` from `chess_board`, `Network` from `crate::network`. Include unit tests.

4. **Modify `crates/nnue/src/network.rs`** — Remove `Accumulator` struct, its `impl` blocks, `Default` impl, the `use chess_types::Color;` import, and the accumulator-related tests. Keep only `Network`.

5. **Create `crates/nnue/src/inference.rs`** — Implement `forward` function using `simd::clipped_relu_i16_to_i8` and `simd::dot_i8_i32`. Include unit tests.

6. **Modify `crates/nnue/src/lib.rs`** — Add `mod accumulator;`, `mod inference;`, `mod simd;`. Update re-exports: `pub use accumulator::Accumulator;`, `pub use inference::forward;`. Remove `Accumulator` from `network` re-export line.

## 7. Verification Commands

```bash
# Format check
cargo fmt --check -p chess-nnue

# Lint check
cargo clippy -p chess-nnue -- -D warnings

# Run all nnue crate tests
cargo test -p chess-nnue

# Run tests with verbose output to confirm all pass
cargo test -p chess-nnue -- --nocapture

# Build with AVX2 SIMD codepath (x86-64 only)
RUSTFLAGS="-C target-feature=+avx2" cargo build -p chess-nnue --features simd-avx2

# Build with SSE2 SIMD codepath (x86-64 only)
RUSTFLAGS="-C target-feature=+sse2" cargo build -p chess-nnue --features simd-sse2

# Full workspace check (ensure no breakage)
cargo check --workspace
```

## 8. Design Deviations

**Deviation 1: SIMD level selection mechanism**

- **Parent spec requires:** "Feature detection at compile time (via Cargo feature flags) or runtime via `std::is_x86_feature_detected!`" and "SIMD level selection: AVX2 > SSE2 > scalar on x86-64; NEON > scalar on ARM."
- **Why compile-time only is preferred here:** Runtime detection via `std::is_x86_feature_detected!` adds a branch on every SIMD call in a hot path. The standard approach in chess engine NNUE implementations (Stockfish, etc.) is compile-time selection via `target_feature` — the binary is compiled targeting the specific architecture it will run on. This is consistent with M11's plan to produce separate builds per SIMD level.
- **Chosen approach:** Use `#[cfg(target_arch)]` + `#[cfg(target_feature)]` for automatic compile-time dispatch. The Cargo feature flags (`simd-avx2`, `simd-sse2`, `simd-neon`) serve as additional gating so users can explicitly opt in. When no SIMD feature is matched, the scalar fallback is used. Runtime detection can be added in Phase P03 (NNUE Optimization) if needed.

**Deviation 2: Accumulator moved to its own module**

- **Parent spec (T01):** Defined `Accumulator` inside `network.rs` alongside `Network`.
- **Why this changes:** T02's task spec explicitly says to create `crates/nnue/src/accumulator.rs` (or similar) for the accumulator struct, initialization, and incremental update methods. With the addition of `refresh` (which depends on `Position`, `feature_index`, and `Network`) the accumulator logic becomes substantial enough to warrant its own module.
- **Chosen approach:** Move `Accumulator` from `network.rs` to `accumulator.rs` as the task spec directs. Existing tests move with it. The `network.rs` module retains only `Network`.