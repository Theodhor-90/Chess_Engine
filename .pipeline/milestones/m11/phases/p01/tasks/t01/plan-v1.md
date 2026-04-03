Let me check the Cargo documentation approach instead. In Cargo workspaces with resolver = "2", you can use `--features package/feature` syntax from the workspace root when building a specific package. This is documented behavior. Given the workspace root `Cargo.toml` uses `resolver = "2"`, the build command `cargo build -p engine --features chess-nnue/simd-avx2` should work.

Now I have all the context I need. Let me produce the plan.

## 1. Deliverables

- **`.github/workflows/release.yml`** — New GitHub Actions workflow file containing a build matrix that cross-compiles the engine binary for all 5 target triples with appropriate SIMD feature flags, producing 7 release artifacts total. Uploads each binary as a workflow artifact.

- **`crates/engine/Cargo.toml`** (modification) — Add feature forwarding so the `engine` crate exposes `simd-sse2`, `simd-avx2`, and `simd-neon` features that propagate to its `chess-nnue` dependency. This enables `cargo build -p engine --features simd-avx2` to activate the correct SIMD codepath in the NNUE crate.

## 2. Dependencies

- **`dtolnay/rust-toolchain@stable`** — GitHub Action already used by the existing `regression.yml` workflow
- **`actions/checkout@v4`** — Already used in `regression.yml`
- **`actions/cache@v4`** — Already used in `regression.yml`
- **`actions/upload-artifact@v4`** — Standard GitHub Action for uploading build artifacts between jobs
- **`cross-rs/cross`** — Required for `aarch64-unknown-linux-gnu` cross-compilation from an `ubuntu-latest` runner (the ARM64 Linux target cannot be built natively on GitHub Actions x86-64 runners without a cross-compilation toolchain)
- No new Cargo dependencies required

## 3. Implementation Details

### `crates/engine/Cargo.toml` — Feature Forwarding

Currently, the `engine` crate depends on `chess-nnue` but does not forward its SIMD feature flags. Without forwarding, `cargo build -p engine --features simd-avx2` would fail because `engine` has no such feature defined. Add a `[features]` section:

```toml
[features]
default = []
simd-sse2 = ["chess-nnue/simd-sse2"]
simd-avx2 = ["chess-nnue/simd-avx2"]
simd-neon = ["chess-nnue/simd-neon"]
```

This enables the CI matrix to use `cargo build --release -p engine --features simd-avx2` and have the feature propagate through the dependency chain to `chess-nnue`, where the SIMD code is implemented.

### `.github/workflows/release.yml` — Cross-compilation Workflow

**Trigger**: `workflow_dispatch` (manual trigger for now — T03 will add the `push: tags: ['v*']` trigger). This keeps the workflow testable during development without requiring a version tag.

**Matrix Strategy**: A single `build` job with an `include`-based matrix defining 7 entries:

| Matrix Entry | `target` | `simd` | `os` | `use_cross` |
|---|---|---|---|---|
| 1 | `x86_64-unknown-linux-gnu` | `simd-sse2` | `ubuntu-latest` | `false` |
| 2 | `x86_64-unknown-linux-gnu` | `simd-avx2` | `ubuntu-latest` | `false` |
| 3 | `x86_64-apple-darwin` | `simd-sse2` | `macos-latest` | `false` |
| 4 | `x86_64-apple-darwin` | `simd-avx2` | `macos-latest` | `false` |
| 5 | `x86_64-pc-windows-msvc` | `simd-sse2` | `windows-latest` | `false` |
| 6 | `x86_64-pc-windows-msvc` | `simd-avx2` | `windows-latest` | `false` |
| 7 | `aarch64-unknown-linux-gnu` | `simd-neon` | `ubuntu-latest` | `true` |
| 8 | `aarch64-apple-darwin` | `simd-neon` | `macos-latest` | `false` |

Note: Entry 8 uses `macos-latest` which on GitHub Actions resolves to ARM64 (M-series) runners, so `aarch64-apple-darwin` can be built natively.

**Steps per matrix entry**:

1. **Checkout**: `actions/checkout@v4`
2. **Setup Rust toolchain**: `dtolnay/rust-toolchain@stable` with `targets: ${{ matrix.target }}`
3. **Cache Cargo artifacts**: `actions/cache@v4` with the same key pattern as `regression.yml` (`${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}`)
4. **Install cross** (conditional on `matrix.use_cross == true`): `cargo install cross --locked`
5. **Preflight checks** (run on native toolchain, not cross-compiled — fmt and clippy are host-only tools, tests require native execution):
   - `cargo fmt --check`
   - `cargo clippy -- -D warnings`
   - `cargo test`
   - These run only on non-cross entries (`matrix.use_cross == false`). For `aarch64-unknown-linux-gnu` (cross-compiled), preflight is skipped because the same code is already validated by the native x86-64 Linux entry.
6. **Build release binary**:
   - If `matrix.use_cross == false`: `cargo build --release -p engine --target ${{ matrix.target }} --features ${{ matrix.simd }}`
   - If `matrix.use_cross == true`: `cross build --release -p engine --target ${{ matrix.target }} --features ${{ matrix.simd }}`
7. **Upload artifact**: `actions/upload-artifact@v4` with:
   - `name: engine-${{ matrix.target }}-${{ matrix.simd }}`
   - `path`: `target/${{ matrix.target }}/release/engine` (or `engine.exe` for Windows)

**Binary path resolution**: On Linux/macOS the binary is `target/<target>/release/engine`. On Windows it is `target/<target>/release/engine.exe`. The workflow uses a conditional to set the correct path.

**Artifact naming convention**: `engine-<target>-<simd>`, e.g., `engine-x86_64-unknown-linux-gnu-simd-avx2`. This produces 7 distinct artifacts for downstream consumption by T02.

## 4. API Contracts

N/A — this task produces a CI workflow file, not a programmatic API.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| YAML validity | Workflow YAML parses without syntax errors (`actionlint` or manual inspection) |
| Matrix coverage | The `include` matrix defines exactly 7 entries covering all 5 targets × appropriate SIMD flags (2 per x86-64 target, 1 per ARM64 target) |
| Rust target installation | Each matrix entry specifies the correct `targets` parameter for `dtolnay/rust-toolchain` |
| Preflight execution | Non-cross entries run `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` |
| Release binary build | Each entry builds with `--release -p engine --target <target> --features <simd>` |
| Artifact upload | Each entry uploads the built binary with a unique artifact name matching `engine-<target>-<simd>` |
| Feature forwarding | `engine` crate's `Cargo.toml` defines `simd-sse2`, `simd-avx2`, `simd-neon` features forwarding to `chess-nnue` |
| Workflow location | File exists at `.github/workflows/release.yml` |

## 6. Implementation Order

1. **`crates/engine/Cargo.toml`** — Add `[features]` section with `simd-sse2`, `simd-avx2`, `simd-neon` forwarding to `chess-nnue`
2. **`.github/workflows/release.yml`** — Create the workflow file with the full build matrix, toolchain setup, preflight checks, build commands, and artifact uploads

## 7. Verification Commands

```bash
# Verify engine Cargo.toml feature forwarding is syntactically correct
cargo check -p engine --features simd-sse2
cargo check -p engine --features simd-avx2

# Verify the workflow YAML is valid (if actionlint is available)
actionlint .github/workflows/release.yml

# Verify existing tests still pass after Cargo.toml modification
cargo test -p engine

# Verify the workflow file exists at the correct path
ls -la .github/workflows/release.yml
```

## 8. Design Deviations

**Deviation 1: Feature forwarding in `crates/engine/Cargo.toml`**

- **Parent spec requires**: The task spec states "The `chess-nnue` crate already defines `simd-sse2`, `simd-avx2`, and `simd-neon` Cargo feature flags — the workflow passes these at build time per target triple."
- **Why direct passthrough is problematic**: The `engine` crate is the build target (`-p engine`), and Cargo does not automatically forward `--features simd-avx2` to transitive dependencies unless the top-level crate defines and forwards those features. Without feature forwarding, the workflow would need to use `--features chess-nnue/simd-avx2` syntax, which only works from the workspace root and is fragile when the build target (`-p engine`) differs from the feature-owning package.
- **Alternative chosen**: Add `[features]` to `crates/engine/Cargo.toml` that forwards each SIMD feature to `chess-nnue`. This is the standard Cargo pattern for propagating features through the dependency chain and enables the clean build command `cargo build --release -p engine --features simd-avx2`.

**Deviation 2: Eight matrix entries instead of seven**

- **Parent spec requires**: "Five target triples, seven binaries total" (2 per x86-64 × 3 platforms = 6, plus 1 ARM64 Linux NEON = 7).
- **Why seven entries is insufficient**: The spec lists both `aarch64-apple-darwin` and `aarch64-unknown-linux-gnu` as ARM64 targets, each producing 1 binary. That is 6 (x86-64) + 2 (ARM64) = 8 total entries. The spec table showing 7 appears to count only one ARM64 target, but the design decision #3 in the phase spec explicitly lists five triples including both ARM64 targets, yielding "seven release artifacts." Counting more carefully: 3 x86-64 platforms × 2 SIMD levels = 6, plus 2 ARM64 platforms × 1 SIMD level = 2, totaling 8. The matrix must have 8 entries to cover all targets.
- **Alternative chosen**: Define 8 matrix entries to cover all 5 target triples × all SIMD levels. The `aarch64-apple-darwin` target builds natively on `macos-latest` (ARM64 runner), while `aarch64-unknown-linux-gnu` uses `cross`.