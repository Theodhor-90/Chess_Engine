# ChessEngine

**Author:** Theodhor

A competitive chess engine written in Rust, built from first principles, targeting UCI compatibility.

## Engine Features

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

## Build Instructions

### Prerequisites

- Rust stable toolchain (2021 edition)

### Basic Build

```bash
cargo build --release -p engine
```

### SIMD Feature Flags

The engine supports SIMD-accelerated NNUE inference via feature flags:

- `simd-sse2` — for x86-64 with SSE2 (broad compatibility)
- `simd-avx2` — for x86-64 with AVX2 (modern CPUs, faster NNUE inference)
- `simd-neon` — for ARM64 (Apple Silicon, ARM Linux)

Example:

```bash
cargo build --release -p engine --features simd-avx2
```

### Cross-Compilation Targets

The CI pipeline builds release binaries for the following targets:

- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `x86_64-pc-windows-msvc`
- `aarch64-unknown-linux-gnu`
- `aarch64-apple-darwin`

### Running Tests

```bash
cargo test
```

### Pre-Built Binaries

Pre-built binaries are available from [GitHub Releases](../../releases). Archives follow the naming convention:

- `chess-engine-{target}-{simd_level}.tar.gz` (Linux, macOS)
- `chess-engine-{target}-{simd_level}.zip` (Windows)

Each archive contains the engine binary and `nn.nnue`.

### Creating a Release

Push a SemVer tag to trigger the release workflow:

```bash
git tag v1.0.0 && git push --tags
```

## UCI Option Reference

| Option | Type | Default | Range / Values | Description |
|--------|------|---------|----------------|-------------|
| `BookFile` | string | `""` | — | Path to a Polyglot opening book `.bin` file. Empty disables the book. |
| `BookMode` | combo | `bestmove` | `bestmove`, `weighted` | Book move selection: `bestmove` always plays the highest-weight move; `weighted` selects randomly proportional to weight. |
| `SyzygyPath` | string | `""` | — | Path to directory containing Syzygy tablebase files. Empty disables probing. |
| `SyzygyProbeDepth` | spin | `6` | 0–7 | Maximum piece count (including kings) for tablebase probing. |
| `EvalFile` | string | `""` | — | Path to NNUE network file (`.nnue`). Empty disables NNUE evaluation. |
| `EvalMode` | combo | `nnue` | `nnue`, `classical` | Evaluation mode: `nnue` uses the neural network (if loaded via `EvalFile`), `classical` uses hand-crafted evaluation. Falls back to `classical` if no network is loaded. |

## Benchmark Results

All figures measured on a reference machine (CPU: _to be filled_).

### Perft

| Position | Depth | Nodes | Nodes/sec |
|----------|-------|-------|-----------|
| Starting position | 6 | 119,060,324 | _TBD_ |

### Search Performance

| Position | Depth | Nodes/sec | Time |
|----------|-------|-----------|------|
| Starting position | 20 | _TBD_ | _TBD_ |

## Current Elo Estimates

- ~2200+ Elo with classical (hand-crafted) evaluation
- ~2500+ Elo with NNUE evaluation

These are estimated from self-play and tournament results against reference engines. Formal testing results will be updated once complete.
