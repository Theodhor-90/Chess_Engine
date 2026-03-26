## Milestone M11 — Polish & Release

### Goal

Production readiness: packaging, documentation, and community integration.

### Phases

#### Phase P01 — Build & Distribution
- **T01**: Cross-compilation — CI builds for x86-64 (Linux, macOS, Windows) and ARM64 (macOS, Linux). Feature flags for SIMD level (SSE2, AVX2, NEON).
- **T02**: Binary packaging — GitHub Releases with pre-built binaries. Include embedded NNUE weights in the binary (or as a companion file).
- **T03**: Versioning — SemVer, changelog generation from git history, release automation via GitHub Actions.

#### Phase P02 — Documentation & Community
- **T01**: README — features, build instructions, UCI options, benchmark results, Elo estimates.
- **T02**: Architecture documentation — design decisions, data structures, search algorithm explanations. Aimed at other engine authors and contributors.
- **T03**: Lichess Bot integration — deploy as a Lichess bot using the `lichess-bot` bridge. Configure time controls and challenge acceptance. This gives the engine real opponents and a public rating.

### Exit Criteria
- CI produces cross-compiled binaries for x86-64 (Linux, macOS, Windows) and ARM64 (macOS, Linux) with appropriate SIMD feature flags
- GitHub Releases contain pre-built binaries with embedded or companion NNUE weights
- SemVer versioning and changelog generation are automated via GitHub Actions
- README documents features, build instructions, UCI options, benchmarks, and Elo estimates
- Architecture documentation explains design decisions, data structures, and algorithms for contributors
- Lichess Bot is deployed and accepting challenges with configured time controls
- All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`