# Task T01 — Cross-compilation

## Objective

Create a GitHub Actions workflow that builds the chess engine binary for all five target platforms with SIMD-level feature flags, producing seven release artifacts total.

## Deliverables

- **`.github/workflows/release.yml`** — a new workflow file containing a build matrix that produces binaries for all target triples and SIMD levels
- Cross-compilation toolchain setup (Rust target installation, cross-compilation dependencies for Linux ARM64)
- Build artifacts uploaded as workflow artifacts for use by downstream jobs (T02 packaging)

## Target Matrix

Five target triples, seven binaries total:

| Target Triple | SIMD Feature | Binary Count |
|---|---|---|
| `x86_64-unknown-linux-gnu` | `simd-sse2`, `simd-avx2` | 2 |
| `x86_64-apple-darwin` | `simd-sse2`, `simd-avx2` | 2 |
| `x86_64-pc-windows-msvc` | `simd-sse2`, `simd-avx2` | 2 |
| `aarch64-apple-darwin` | `simd-neon` | 1 |
| `aarch64-unknown-linux-gnu` | `simd-neon` | 1 |

## Key Implementation Details

- The `chess-nnue` crate already defines `simd-sse2`, `simd-avx2`, and `simd-neon` Cargo feature flags — the workflow passes these at build time per target triple
- Each build must run existing preflight checks before producing the release binary: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
- The project already uses GitHub Actions for regression testing (`.github/workflows/regression.yml`) — follow existing workflow conventions
- Binaries are compile-time SIMD-specialized (not runtime detection)
- The workflow should use `actions/upload-artifact` to make binaries available to downstream packaging jobs

## Dependencies

- None within this phase (this is the first task)
- All prior milestones (M01–M10) must be complete — this workflow builds the finished engine

## Verification Criteria

1. The workflow YAML is valid and defines a matrix covering all 5 target triples × appropriate SIMD flags (7 combinations)
2. Each matrix entry installs the correct Rust target and cross-compilation toolchain
3. Each matrix entry runs preflight checks (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`)
4. Each matrix entry produces a release binary and uploads it as a workflow artifact
5. The workflow file is located at `.github/workflows/release.yml`