Now let me examine what prior milestones M10 depends on to understand the dependency chain properly.

# Milestone M11 — Polish & Release

## Goal

Deliver a production-ready chess engine by establishing cross-platform CI builds with SIMD-level feature flags, packaging pre-built binaries with NNUE weights as GitHub Releases with automated SemVer versioning and changelogs, writing user-facing and contributor-facing documentation, and deploying the engine as a Lichess bot for live play and public rating.

## Scope

- Cross-compilation CI workflows targeting x86-64 (Linux, macOS, Windows) and ARM64 (macOS, Linux) with feature flags for SIMD levels (SSE2, AVX2, NEON)
- Binary packaging as GitHub Releases with pre-built binaries and embedded or companion NNUE weight files
- SemVer versioning scheme with changelog generation from git history and release automation via GitHub Actions
- README covering features, build instructions, UCI options, benchmark results, and Elo estimates
- Architecture documentation explaining design decisions, data structures, and search algorithm details, aimed at engine authors and contributors
- Lichess Bot deployment using the `lichess-bot` bridge with configured time controls and challenge acceptance

## Phases

### Phase P01 — Build & Distribution

Establish the CI and release infrastructure so the engine can be compiled, packaged, and distributed across all target platforms automatically.

- **T01**: Cross-compilation — configure CI (GitHub Actions) to build the engine binary for x86-64 (Linux, macOS, Windows) and ARM64 (macOS, Linux). Use Cargo feature flags to select SIMD level (SSE2, AVX2, NEON) per target.
- **T02**: Binary packaging — publish pre-built binaries to GitHub Releases. Include the NNUE weight file either embedded in the binary or as a companion file alongside each release artifact.
- **T03**: Versioning — adopt SemVer for release numbers. Automate changelog generation from git history. Wire release automation into GitHub Actions so tagging a version triggers the full build-and-publish pipeline.

### Phase P02 — Documentation & Community

Write the documentation needed for users and contributors, and connect the engine to Lichess for live play.

- **T01**: README — write a comprehensive README covering engine features, build instructions, UCI option reference, benchmark results, and current Elo estimates.
- **T02**: Architecture documentation — document design decisions, core data structures (bitboards, position representation, move encoding, TT layout, NNUE architecture), and search algorithm explanations. Target audience is other engine authors and potential contributors.
- **T03**: Lichess Bot integration — deploy the engine as a Lichess bot using the `lichess-bot` bridge. Configure accepted time controls and challenge acceptance rules. Verify the bot connects, plays games, and obtains a public rating.

## Exit Criteria

1. CI produces cross-compiled binaries for x86-64 (Linux, macOS, Windows) and ARM64 (macOS, Linux) with appropriate SIMD feature flags (SSE2, AVX2, NEON)
2. GitHub Releases contain pre-built binaries with embedded or companion NNUE weights
3. SemVer versioning and changelog generation are automated via GitHub Actions — tagging a version triggers the build-and-release pipeline
4. README documents features, build instructions, UCI options, benchmark results, and Elo estimates
5. Architecture documentation explains design decisions, data structures, and search algorithms for contributors
6. Lichess Bot is deployed, accepting challenges with configured time controls, and playing games on Lichess

## Dependencies

- **M10 (NNUE Evaluation)** must be complete — NNUE weight files are packaged with the release, and the engine binary includes the NNUE inference path
- **All prior milestones (M01–M10)** must be complete — this milestone packages and documents the finished engine
- **GitHub Actions** — CI/CD platform for cross-compilation and release automation
- **`lichess-bot` bridge** — external tool used to connect the UCI engine to the Lichess API