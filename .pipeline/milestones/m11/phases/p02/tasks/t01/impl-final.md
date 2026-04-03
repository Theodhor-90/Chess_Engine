All tests passed (7/7, 0 failures). All verification commands confirm success.

**Summary:**

- **`README.md`** (created) — comprehensive user-facing documentation with five sections:
  - **Engine Features**: bitboard move gen, PVS, pruning, NNUE, tablebases, UCI support, etc.
  - **Build Instructions**: prerequisites, basic build, SIMD flags (`simd-sse2`, `simd-avx2`, `simd-neon`), cross-compilation targets, pre-built binaries, release trigger
  - **UCI Option Reference**: table of all 6 options (BookFile, BookMode, SyzygyPath, SyzygyProbeDepth, EvalFile, EvalMode) with types, defaults, ranges, descriptions matching engine source
  - **Benchmark Results**: perft and search performance tables with placeholder values
  - **Current Elo Estimates**: ~2200+ HCE, ~2500+ NNUE