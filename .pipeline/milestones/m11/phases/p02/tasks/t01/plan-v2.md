## 1. Deliverables

- **`README.md`** at the repository root — comprehensive user-facing documentation covering engine features, build instructions, UCI option reference, benchmark results, and Elo estimates.

## 2. Dependencies

- None. This task creates a single Markdown file with no code or package dependencies.
- **Prerequisite**: Phase P01 (Build & Distribution) is complete — the README references cross-compilation targets, SIMD feature flags, release binaries, and versioning from the CI workflows already in place.

## 3. Implementation Details

### README.md

A single Markdown file with five required sections, plus a brief header/introduction.

#### Header
- Engine name: `ChessEngine`
- Author: `Theodhor`
- One-line description: a competitive chess engine written in Rust, built from first principles, targeting UCI compatibility.

#### Section 1 — Engine Features
Summarize the engine's capabilities based on what milestones M01–M10 delivered:
- Bitboard-based board representation with magic bitboards for sliding piece move generation
- Alpha-beta search with Principal Variation Search (PVS)
- Iterative deepening with time management
- Quiescence search
- Transposition table with Zobrist hashing
- Pruning: null move pruning, late move reductions (LMR), futility pruning
- Move ordering: MVV-LVA, killer moves, history heuristic, counter-move heuristic, static exchange evaluation (SEE)
- Extensions: check extension, singular extension
- NNUE evaluation (HalfKP architecture with SIMD-accelerated inference)
- Classical evaluation fallback (material + piece-square tables, pawn structure, king safety, mobility)
- Polyglot opening book support
- Syzygy endgame tablebase probing
- Full UCI protocol support (including pondering)

#### Section 2 — Build Instructions
Document the Cargo workspace setup:
- Prerequisites: Rust stable toolchain (2021 edition)
- Basic build: `cargo build --release -p engine`
- SIMD feature flags:
  - `simd-sse2` — for x86-64 with SSE2 (broad compatibility)
  - `simd-avx2` — for x86-64 with AVX2 (modern CPUs, faster NNUE inference)
  - `simd-neon` — for ARM64 (Apple Silicon, ARM Linux)
  - Example: `cargo build --release -p engine --features simd-avx2`
- Cross-compilation targets from CI: `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`, `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin`
- Running tests: `cargo test`
- Pre-built binaries available from GitHub Releases (archive naming: `chess-engine-{target}-{simd_level}.tar.gz` or `.zip` for Windows, each containing the engine binary and `nn.nnue`)
- Release trigger: pushing a SemVer tag (e.g., `git tag v1.0.0 && git push --tags`)

#### Section 3 — UCI Option Reference
Table of all 6 UCI options with name, type, default, range/values, and description:

| Option | Type | Default | Range / Values | Description |
|--------|------|---------|----------------|-------------|
| `BookFile` | string | `""` | — | Path to a Polyglot opening book `.bin` file. Empty disables the book. |
| `BookMode` | combo | `bestmove` | `bestmove`, `weighted` | Book move selection: `bestmove` always plays the highest-weight move; `weighted` selects randomly proportional to weight. |
| `SyzygyPath` | string | `""` | — | Path to directory containing Syzygy tablebase files. Empty disables probing. |
| `SyzygyProbeDepth` | spin | `6` | 0–7 | Maximum piece count (including kings) for tablebase probing. |
| `EvalFile` | string | `""` | — | Path to NNUE network file (`.nnue`). Empty disables NNUE evaluation. |
| `EvalMode` | combo | `nnue` | `nnue`, `classical` | Evaluation mode: `nnue` uses the neural network (if loaded via `EvalFile`), `classical` uses hand-crafted evaluation. Falls back to `classical` if no network is loaded. |

#### Section 4 — Benchmark Results
Present benchmark figures in a structured format. Since actual numbers depend on hardware and the current engine state, use concrete placeholder values formatted for easy replacement with real data. Include:
- Perft results: starting position perft(6) nodes and nodes/sec
- Search NPS: nodes per second at fixed depth from a standard position (e.g., starting position depth 20)
- Time-to-depth: time to reach depth 20 from the starting position
- Hardware note: specify that figures are from a reference machine (to be filled with actual CPU model)

#### Section 5 — Current Elo Estimates
State the target ratings from the master plan:
- ~2200+ Elo with classical (hand-crafted) evaluation
- ~2500+ Elo with NNUE evaluation
- Note these are estimated from self-play and tournament results against reference engines, with a note to update once formal testing is complete.

## 4. API Contracts

N/A — this task produces a documentation file, not code.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| `README.md` exists at repository root | File was created in the correct location |
| Contains "Features" or "Engine Features" heading | Section 1 present |
| Contains "Build" heading | Section 2 present |
| Contains all three SIMD feature flags (`simd-sse2`, `simd-avx2`, `simd-neon`) | Build instructions include SIMD documentation |
| Contains "UCI" heading with all 6 option names (`BookFile`, `BookMode`, `SyzygyPath`, `SyzygyProbeDepth`, `EvalFile`, `EvalMode`) | Section 3 is complete and matches engine's actual options |
| Contains "Benchmark" heading | Section 4 present |
| Contains "Elo" heading | Section 5 present |

Verification is manual inspection (grep-based checks), since the deliverable is a Markdown file, not executable code.

## 6. Implementation Order

1. Read the engine source (`crates/engine/src/main.rs`) and CI workflow (`.github/workflows/release.yml`) to confirm all details are accurate against the actual implementation.
2. Write `README.md` — compose all five sections in a single file, referencing the actual UCI option names/types/defaults from the engine source and the actual build targets/feature flags from the CI workflow.
3. Verify the file is well-formed Markdown and all section content matches the codebase.

## 7. Verification Commands

```bash
# Verify README.md exists at the repository root
test -f README.md && echo "README.md exists" || echo "MISSING"

# Verify all five required sections are present
grep -c '## .*Features\|## .*Build\|## .*UCI\|## .*Benchmark\|## .*Elo' README.md

# Verify SIMD feature flags are documented
grep -c 'simd-sse2\|simd-avx2\|simd-neon' README.md

# Verify all 6 UCI options are documented
grep -c 'BookFile\|BookMode\|SyzygyPath\|SyzygyProbeDepth\|EvalFile\|EvalMode' README.md

# Preflight checks still pass (no code changed)
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

## 8. Design Deviations

None.