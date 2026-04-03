# T01 — README

## Objective

Write a comprehensive `README.md` at the repository root that serves as the primary user entry point for the chess engine.

## Deliverable

- **File**: `README.md` at the repository root

## Required Sections

1. **Engine Features** — bitboard-based move generation (magic bitboards), alpha-beta search with modern enhancements (PVS, null move pruning, LMR, futility pruning, killer/history/counter-move heuristics, check and singular extensions), NNUE evaluation, UCI protocol support, opening book (Polyglot) and endgame tablebase (Syzygy) support
2. **Build Instructions** — Cargo workspace setup, feature flags for SIMD levels (`simd-sse2`, `simd-avx2` for x86-64, `simd-neon` for ARM64), cross-compilation targets (x86-64 Linux/macOS/Windows, ARM64 Linux/macOS)
3. **UCI Option Reference** — all supported UCI options and their effects (hash size, threads, Syzygy path, book path, evaluation mode, etc.)
4. **Benchmark Results** — perft speeds (nodes/sec), search NPS, nodes per second at fixed depth, time-to-depth figures
5. **Current Elo Estimates** — estimated strength from self-play or tournament results (target: 2200+ HCE, 2500+ NNUE)

## Constraints

- Documentation covers the full engine (all milestones M01–M10 complete, Phase P01 Build & Distribution complete)
- References build instructions, release binaries, and versioning from Phase P01
- Must be accurate and consistent with the actual engine implementation

## Dependencies

- None within this phase (T01 is independent)
- Phase P01 (Build & Distribution) must be complete — README references cross-compilation, release binaries, and versioning

## Verification Criteria

1. `README.md` exists at the repository root
2. Contains all five required sections: features, build instructions, UCI options, benchmarks, Elo estimates
3. Build instructions are accurate and include SIMD feature flag documentation
4. UCI option reference is complete and matches the engine's actual options
5. Benchmark and Elo sections contain concrete numbers or placeholder format ready for real data