Now I have all the information I need. Let me produce the implementation plan.

## 1. Deliverables

| # | Action | File | Purpose |
|---|--------|------|---------|
| 1 | **Create** | `crates/nnue/src/format.rs` | File format constants (magic bytes, version), header struct, endianness convention, and format specification documentation in module-level doc comment |
| 2 | **Create** | `crates/nnue/src/loader.rs` | `load` function that reads a `.nnue` binary file into a `Network` struct, with header validation, layer dimension verification, and error handling |
| 3 | **Create** | `crates/nnue/tests/testdata/valid.nnue` | Synthetic test fixture file with known weights for round-trip testing (generated programmatically by a test helper, not committed as a binary blob) |
| 4 | **Modify** | `crates/nnue/src/lib.rs` | Add `mod format;` and `mod loader;` declarations, add public re-exports for `load`, error type, and format constants |
| 5 | **Modify** | `crates/nnue/Cargo.toml` | Add `thiserror = "2"` dependency for error types |

## 2. Dependencies

- **`thiserror = "2"`** — added to `crates/nnue/Cargo.toml` for deriving error types (already used by `crates/types` and `crates/board` in this project)
- No other new external dependencies. File I/O uses `std::io::{Read, BufReader, Write, BufWriter}` and `std::fs::File`. Byte-order conversion uses `i16::from_le_bytes`, `i32::from_le_bytes`, etc. from `std`.

## 3. Implementation Details

### 3.1 `crates/nnue/src/format.rs`

**Purpose**: Define the custom `.nnue` binary file format constants and header structure.

**Design decision**: Use a custom format rather than Stockfish-compatible because:
- The Stockfish `.nnue` format is tightly coupled to their specific architecture (HalfKAv2, different layer sizes, feature transformer layout) and uses a complex nested section header scheme with architecture-specific hash codes.
- Our topology (HalfKP input, 40960→256→32→1) differs from Stockfish's, making direct compatibility impractical.
- A custom little-endian format is simpler, self-documenting, and matches x86 native byte order.

**Exports**:
- `MAGIC: [u8; 4]` — magic bytes `b"NNUE"` identifying the file format
- `FORMAT_VERSION: u32` — version 1
- `Header` struct — parsed file header containing version, architecture hash, and layer dimensions
- `architecture_hash() -> u32` — computes a hash from the compiled network constants (`HALFKP_FEATURES`, `L1_SIZE`, `L2_SIZE`, `OUTPUT_SIZE`) to detect mismatches between file and compiled code
- `write_header(writer: &mut impl std::io::Write, header: &Header) -> std::io::Result<()>` — serializes a header (used by test helper to create fixture files)

**Header layout** (all little-endian):
| Offset | Size | Field |
|--------|------|-------|
| 0 | 4 | Magic bytes `b"NNUE"` |
| 4 | 4 | `u32` format version |
| 8 | 4 | `u32` architecture hash |
| 12 | 4 | `u32` halfkp_features (input size) |
| 16 | 4 | `u32` l1_size |
| 20 | 4 | `u32` l2_size |
| 24 | 4 | `u32` output_size |

Total header: 28 bytes.

**Key logic**:
- `architecture_hash()`: simple XOR-rotate hash of the four dimension constants, providing a fast check that file and code expect the same topology.

```rust
pub const MAGIC: [u8; 4] = *b"NNUE";
pub const FORMAT_VERSION: u32 = 1;
pub const HEADER_SIZE: usize = 28;

pub struct Header {
    pub version: u32,
    pub arch_hash: u32,
    pub halfkp_features: u32,
    pub l1_size: u32,
    pub l2_size: u32,
    pub output_size: u32,
}
```

### 3.2 `crates/nnue/src/loader.rs`

**Purpose**: Load a `.nnue` file from disk into a `Network` struct with full validation.

**Exports**:
- `pub fn load(path: &std::path::Path) -> Result<Network, NnueLoadError>` — reads and validates the file
- `pub enum NnueLoadError` — error type with variants for all failure modes
- `pub fn write(path: &std::path::Path, network: &Network) -> Result<(), NnueLoadError>` — writes a `Network` to a `.nnue` file (needed for test fixture generation, and symmetrical with `load`)

**Error type** (using `thiserror`):

```rust
#[derive(Debug, thiserror::Error)]
pub enum NnueLoadError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid magic bytes: expected {expected:?}, got {got:?}")]
    InvalidMagic { expected: [u8; 4], got: [u8; 4] },
    #[error("unsupported format version: expected {expected}, got {got}")]
    UnsupportedVersion { expected: u32, got: u32 },
    #[error("architecture mismatch: file hash {file_hash:#010x}, expected {expected_hash:#010x}")]
    ArchitectureMismatch { file_hash: u32, expected_hash: u32 },
    #[error("dimension mismatch: file declares {field}={file_value}, compiled expects {expected_value}")]
    DimensionMismatch { field: &'static str, file_value: u32, expected_value: u32 },
    #[error("unexpected end of file while reading {context}")]
    UnexpectedEof { context: &'static str },
}
```

**Key logic**:
1. Open file with `BufReader`
2. Read and validate 28-byte header: check magic, version, architecture hash, and each dimension against compiled constants
3. Read input weights: `HALFKP_FEATURES * L1_SIZE` values as `i16` (little-endian)
4. Read input bias: `L1_SIZE` values as `i16`
5. Read hidden1 weights: `L2_SIZE * 2 * L1_SIZE` values as `i8`
6. Read hidden1 bias: `L2_SIZE` values as `i32` (little-endian)
7. Read hidden2 weights: `L2_SIZE` values as `i8`
8. Read hidden2 bias: 1 value as `i32`
9. Construct and return `Network`

**Weight ordering in the file**: matches the in-memory layout exactly — `input_weights` is column-major (`HALFKP_FEATURES * L1_SIZE` i16 values written sequentially), then `input_bias`, then `hidden1_weights`, etc.

**Byte reading helpers** (private): `read_i16_vec`, `read_i8_vec`, `read_i32` — read N values of the specified type from the reader in little-endian format. These are small private functions within `loader.rs`, not separate utilities.

**`write` function**: Inverse of `load` — writes header then each weight array in the same order. Used by tests to create synthetic fixture files programmatically.

### 3.3 Test fixture strategy

Instead of committing a binary `.nnue` file, tests will use the `write` function to create a temporary `.nnue` file with known weights, then `load` it back and verify. This avoids binary fixtures in version control and ensures the write/load round-trip is tested.

For the test that verifies loaded weights produce the expected inference output, the test will:
1. Create a `Network` with deterministic non-zero weights
2. Write it to a temp file via `write`
3. Load it back via `load`
4. Run `forward` with a known `Accumulator` state and verify the output matches the expected score

### 3.4 `crates/nnue/src/lib.rs` modifications

Add module declarations and re-exports:

```rust
pub mod format;
pub mod loader;

pub use loader::{load, write, NnueLoadError};
```

### 3.5 `crates/nnue/Cargo.toml` modifications

Add `thiserror` dependency:

```toml
[dependencies]
chess-types = { path = "../types" }
chess-board = { path = "../board" }
thiserror = "2"
```

## 4. API Contracts

### `load(path: &Path) -> Result<Network, NnueLoadError>`

**Input**: Filesystem path to a `.nnue` binary file.

**Output on success**: A fully populated `Network` struct with all weight arrays matching the compiled architecture constants.

**Output on failure**: `NnueLoadError` variant describing the issue.

**Example**:
```rust
use chess_nnue::load;
use std::path::Path;

let network = load(Path::new("weights.nnue"))?;
// network.input_weights.len() == HALFKP_FEATURES * L1_SIZE
// network.hidden1_weights.len() == L2_SIZE * 2 * L1_SIZE
```

### `write(path: &Path, network: &Network) -> Result<(), NnueLoadError>`

**Input**: Filesystem path and a reference to a `Network`.

**Output on success**: `Ok(())` — file written to disk.

**Output on failure**: `NnueLoadError::Io` wrapping the I/O error.

### `architecture_hash() -> u32`

**Input**: None (reads compile-time constants).

**Output**: A `u32` hash derived from `HALFKP_FEATURES`, `L1_SIZE`, `L2_SIZE`, `OUTPUT_SIZE`.

## 5. Test Plan

All tests reside in `crates/nnue/src/loader.rs` as `#[cfg(test)] mod tests` (colocated, per project convention).

### Test 1: `load_valid_roundtrip`
- Creates a `Network` with deterministic non-zero weights (e.g., `input_weights[i] = (i % 256) as i16 - 128`)
- Writes it to a temp file via `write`
- Loads it back via `load`
- Asserts every weight array and bias matches the original

**Covers verification criteria**: 1 (load returns populated Network), 5 (round-trip correctness).

### Test 2: `load_invalid_magic`
- Writes a valid file, then overwrites the first 4 bytes with `b"BAAD"`
- Calls `load`, asserts `NnueLoadError::InvalidMagic`

**Covers verification criterion**: 2 (incorrect magic bytes).

### Test 3: `load_dimension_mismatch`
- Writes a valid header but with a wrong `l1_size` value
- Calls `load`, asserts `NnueLoadError::DimensionMismatch`

**Covers verification criterion**: 3 (mismatched layer dimensions).

### Test 4: `load_truncated_file`
- Writes a valid header but truncates the file before all weights are written
- Calls `load`, asserts an error is returned (either `Io` with `UnexpectedEof` kind or `NnueLoadError::UnexpectedEof`)

**Covers verification criterion**: 4 (truncated file, no panic).

### Test 5: `load_and_forward_reference`
- Creates a `Network` with known small weights, writes and loads it
- Constructs an `Accumulator` with known values
- Calls `forward` with the loaded network
- Asserts the output score matches the hand-computed expected value

**Covers verification criterion**: 5 (loaded weights work with inference engine).

### Test 6: `load_unsupported_version`
- Writes a valid file but patches the version field to `99`
- Calls `load`, asserts `NnueLoadError::UnsupportedVersion`

**Covers verification criteria**: 2 (header validation — version variant).

## 6. Implementation Order

1. **`crates/nnue/Cargo.toml`** — add `thiserror = "2"` dependency
2. **`crates/nnue/src/format.rs`** — define magic, version, header struct, `architecture_hash()`, `write_header()`, and `read_header()` functions
3. **`crates/nnue/src/loader.rs`** — implement `NnueLoadError` enum, `load` function, `write` function, byte-reading helpers, and all tests
4. **`crates/nnue/src/lib.rs`** — add `mod format;`, `mod loader;`, and public re-exports

## 7. Verification Commands

```bash
# Format check
cargo fmt --check -p chess-nnue

# Lint check
cargo clippy -p chess-nnue -- -D warnings

# Run all tests for the nnue crate
cargo test -p chess-nnue

# Run loader tests specifically (to verify new tests pass)
cargo test -p chess-nnue loader

# Ensure full workspace still builds
cargo build --workspace
```

## 8. Design Deviations

**Deviation 1: Custom format instead of Stockfish-compatible**

- **Parent spec states**: "Support Stockfish-compatible format or define custom format"
- **Decision**: Custom format
- **Rationale**: The Stockfish `.nnue` format uses a nested section-based header scheme with architecture-specific hash codes tied to their HalfKAv2 feature transformer (768→512→32→32→1). Our architecture uses HalfKP (40960→256→32→1) with different quantization, making Stockfish compatibility impossible without architecture changes. The custom format is simpler, self-documenting, and the spec explicitly permits this choice.

**Deviation 2: Including a `write` function**

- **Parent spec states**: Deliverables focus on `load` and reading
- **Decision**: Include a `write` function in `loader.rs`
- **Rationale**: The spec requires a "test fixture `.nnue` file with known weights." Writing the fixture programmatically via a `write` function is more maintainable than committing a binary blob, and it tests the format specification from both directions. The `write` function is the minimal way to satisfy the test fixture requirement without binary files in version control. It also enables the Phase P02 training pipeline to export networks.