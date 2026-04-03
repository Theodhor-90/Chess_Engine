Now I have all the context. Let me address both feedback issues:

1. **Scope issue**: Remove the stray agent commentary on line 1
2. **Ambiguity issue**: Replace "with appropriate SIMD feature flags" with a more specific phrasing

# Phase P01 — Build & Distribution

## Goal

Establish the CI and release infrastructure so the chess engine can be cross-compiled for x86-64 (Linux, macOS, Windows) and ARM64 (macOS, Linux) with SIMD feature flags, packaged as GitHub Releases with NNUE weights, and versioned with automated SemVer and changelog generation — enabling one-step distribution of production binaries to users.

## Design Decisions

1. **GitHub Actions as CI/CD platform**: The project already uses GitHub Actions for regression testing (`.github/workflows/regression.yml`). Cross-compilation and release workflows extend this existing infrastructure rather than introducing a new CI system.

2. **Cargo feature flags for SIMD selection**: The `chess-nnue` crate already defines `simd-sse2`, `simd-avx2`, and `simd-neon` feature flags. The CI matrix will pass these features at build time per target triple, producing distinct binaries with compile-time SIMD specialization rather than runtime detection.

3. **Five target triples**: `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`, `aarch64-apple-darwin`, `aarch64-unknown-linux-gnu`. Each x86-64 target produces two binaries (SSE2 and AVX2); each ARM64 target produces one binary (NEON). This yields seven release artifacts total.

4. **NNUE weights as companion file**: The NNUE weight file is included alongside each release artifact rather than compiled into the binary, keeping binary size manageable and allowing weight file updates without recompilation. This matches the M10 design where weights are loaded from a `.nnue` file.

5. **Tag-triggered release pipeline**: Pushing a SemVer tag (e.g., `v1.0.0`) triggers the full build-and-publish workflow. This separates development CI (run on PRs) from release CI (run on tags), keeping PR feedback fast.

## Tasks

### T01 — Cross-compilation

Create a GitHub Actions workflow that builds the engine binary for all five target platforms with the following SIMD feature flags: `simd-sse2` and `simd-avx2` for x86-64 targets, `simd-neon` for ARM64 targets. The workflow matrix covers x86-64 (Linux, macOS, Windows) with both SSE2 and AVX2 variants, and ARM64 (macOS, Linux) with NEON. Each build runs the existing preflight checks (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`) before producing the release binary.

**Deliverables:**
- `.github/workflows/release.yml` with a build matrix producing binaries for all target triples and SIMD levels
- Cross-compilation toolchain setup (Rust target installation, cross-compilation dependencies for Linux ARM64)
- Build artifacts uploaded as workflow artifacts for use by downstream jobs

### T02 — Binary Packaging

Extend the release workflow to package each built binary with the NNUE weight file and publish them as GitHub Release assets. Each asset is a compressed archive (`.tar.gz` for Linux/macOS, `.zip` for Windows) named with the target triple and SIMD level for clear identification.

**Deliverables:**
- Archive creation step in the release workflow producing named archives per target/SIMD combination
- NNUE weight file included in each archive as a companion file
- GitHub Release created with all archives attached as assets

### T03 — Versioning

Adopt SemVer for the project. Automate changelog generation from git history (commit messages since the previous tag). Wire the release workflow to trigger on version tags, so that `git tag v1.0.0 && git push --tags` executes the full build-package-publish pipeline end to end.

**Deliverables:**
- Changelog generation step producing a changelog from git history between tags
- Release workflow trigger configured on `v*` tag pushes
- GitHub Release created with the generated changelog as the release body
- `Cargo.toml` workspace version aligned with the tagged SemVer version

## Exit Criteria

1. CI produces cross-compiled binaries for x86-64 (Linux, macOS, Windows) and ARM64 (macOS, Linux) with `simd-sse2` and `simd-avx2` feature flags for x86-64 targets and `simd-neon` for ARM64 targets
2. GitHub Releases contain pre-built binaries with companion NNUE weight files, packaged as compressed archives named by target and SIMD level
3. SemVer versioning and changelog generation are automated via GitHub Actions — tagging a version triggers the build-and-release pipeline

## Dependencies

- All prior milestones (M01–M10) must be complete — this phase packages the finished engine
- M10 (NNUE Evaluation) specifically required for NNUE weight files to package
- Existing GitHub Actions CI infrastructure (`.github/workflows/regression.yml`) as a reference for workflow conventions
- No dependency on Phase P02 — this phase is independent and comes first

## Artifacts

- `.github/workflows/release.yml` — cross-compilation, packaging, and release automation workflow
- Compressed release archives (7 total) published to GitHub Releases — one per target/SIMD combination, each containing the engine binary and NNUE weight file
- Generated changelog included in each GitHub Release body