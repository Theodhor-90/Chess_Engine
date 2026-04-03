## Phase P01 — Build & Distribution

### Goal

Establish the CI and release infrastructure so the chess engine can be compiled, packaged, and distributed across all target platforms automatically. This phase delivers cross-platform GitHub Actions workflows, binary packaging as GitHub Releases with NNUE weights, and automated SemVer versioning with changelog generation.

### Deliverables

- **T01 — Cross-compilation**: GitHub Actions CI configured to build the engine binary for x86-64 (Linux, macOS, Windows) and ARM64 (macOS, Linux). Cargo feature flags select SIMD level per target: `simd-sse2` and `simd-avx2` for x86-64 builds, `simd-neon` for ARM64 builds.
- **T02 — Binary packaging**: Pre-built binaries published to GitHub Releases. The NNUE weight file is included either embedded in the binary or as a companion file alongside each release artifact.
- **T03 — Versioning**: SemVer adopted for release numbers. Changelog generation automated from git history. Release automation wired into GitHub Actions so that tagging a version triggers the full build-and-publish pipeline.

### Technical Decisions & Constraints

- CI/CD platform is GitHub Actions (per project conventions in MASTER_PLAN.md)
- SIMD feature flags must be Cargo feature flags (`simd-sse2`, `simd-avx2`, `simd-neon`) — not runtime detection
- Five target triples required: x86-64 Linux, x86-64 macOS, x86-64 Windows, ARM64 macOS, ARM64 Linux
- NNUE weights from M10 must be bundled with or embedded in the release binaries
- The project uses Cargo workspaces with a top-level `engine` binary crate that wires everything together
- Preflight checks (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`) must pass in CI

### Dependencies

- All prior milestones (M01–M10) must be complete — this phase packages the finished engine
- M10 (NNUE Evaluation) specifically required for NNUE weight files to package
- No dependency on Phase P02 (this phase is independent and comes first)