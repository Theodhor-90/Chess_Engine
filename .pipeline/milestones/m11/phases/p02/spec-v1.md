# Phase P02 — Documentation & Community

## Goal

Write user-facing and contributor-facing documentation for the chess engine and deploy it as a Lichess bot for live play, ensuring the engine is approachable for end users via a comprehensive README, understandable for contributors via architecture documentation, and actively playing on Lichess with a public rating.

## Design Decisions

1. **README as the primary user entry point**: The README serves as the single document covering features, build instructions, UCI options, benchmark results, and Elo estimates. It lives at the repository root as `README.md`.

2. **Architecture docs as a separate document**: Architecture documentation is a standalone file (e.g., `ARCHITECTURE.md` at the repository root) rather than scattered inline comments, since the target audience is engine authors and contributors who need a cohesive overview of design decisions, data structures, and search algorithms.

3. **Lichess Bot via `lichess-bot` bridge**: The engine connects to Lichess using the external `lichess-bot` bridge tool, which wraps any UCI engine into a Lichess-compatible bot. This avoids implementing the Lichess API directly and leverages the existing UCI protocol the engine already supports.

4. **Documentation covers the full engine**: Since all milestones M01–M10 and Phase P01 (Build & Distribution) are complete, documentation describes the final engine including NNUE evaluation, all search techniques, cross-platform builds, and release binaries.

## Tasks

### T01 — README

Write a comprehensive `README.md` at the repository root covering:
- Engine features (bitboard move generation, alpha-beta with modern enhancements, NNUE evaluation, UCI protocol)
- Build instructions (Cargo workspace, feature flags for SIMD levels, cross-compilation targets)
- UCI option reference (all supported UCI options and their effects)
- Benchmark results (perft speeds, search NPS, relevant performance figures)
- Current Elo estimates

**Deliverable**: `README.md` at the repository root containing all listed sections.

### T02 — Architecture Documentation

Write an architecture document explaining:
- Design decisions and their rationale (bitboard representation, magic bitboards, packed move encoding, Zobrist hashing, TT layout, NNUE architecture)
- Core data structures (bitboards, position representation, move encoding, transposition table, NNUE accumulator)
- Search algorithm explanations (alpha-beta, PVS, iterative deepening, pruning techniques, extensions, move ordering)

Target audience is other engine authors and potential contributors.

**Deliverable**: Architecture documentation file explaining design decisions, data structures, and search algorithms.

### T03 — Lichess Bot Integration

Deploy the engine as a Lichess bot using the `lichess-bot` bridge:
- Configure the `lichess-bot` bridge to run the engine binary via UCI
- Configure accepted time controls and challenge acceptance rules
- Verify the bot connects to Lichess, plays games, and obtains a public rating

**Deliverable**: Lichess bot deployment configuration and verification that the bot connects and plays games on Lichess.

## Exit Criteria

1. README documents features, build instructions, UCI options, benchmark results, and Elo estimates
2. Architecture documentation explains design decisions, data structures, and search algorithms for contributors
3. Lichess Bot is deployed, accepting challenges with configured time controls, and playing games on Lichess

## Dependencies

- **Phase P01 (Build & Distribution)** must be complete — the README references build instructions, release binaries, and versioning; architecture docs reference the final engine structure
- **All prior milestones (M01–M10)** must be complete — documentation covers the full engine
- **`lichess-bot` bridge** — external tool required for T03

## Artifacts

- `README.md` — comprehensive user-facing documentation at the repository root
- Architecture documentation file — contributor-facing design and algorithm documentation
- Lichess bot configuration — `lichess-bot` bridge configuration with time controls and challenge acceptance rules