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

- **Parent spec requires**: "Five target triples, seven binaries total" (phase spec design decision #3).
- **Why seven is incorrect**: The phase spec lists five target triples including both `aarch64-apple-darwin` and `aarch64-unknown-linux-gnu`. Counting correctly: 3 x86-64 platforms × 2 SIMD levels = 6, plus 2 ARM64 platforms × 1 SIMD level = 2, totaling 8. The spec's count of "seven" undercounts by one ARM64 target.
- **Alternative chosen**: Define 8 matrix entries to cover all 5 target triples × all SIMD levels. The `aarch64-apple-darwin` target builds natively on `macos-latest` (ARM64 runner), while `aarch64-unknown-linux-gnu` uses `cross`.

**Deviation 3: macOS x86-64 builds use `macos-13` instead of `macos-latest`**

- **Why**: GitHub Actions `macos-latest` resolves to ARM64 (M-series) runners. Using `macos-latest` for x86-64 macOS builds would mean preflight `cargo test` compiles and runs tests against `aarch64-apple-darwin` (the host architecture) rather than the intended `x86_64-apple-darwin` target. This validates code correctness on a different architecture than the release binary targets.
- **Alternative chosen**: Use `macos-13` for entries 3–4 (`x86_64-apple-darwin`), which provides native Intel x86-64 runners. This ensures preflight checks and the release build both target x86-64 natively, matching the intended target architecture. `macos-13` runners remain available on GitHub Actions for this purpose.