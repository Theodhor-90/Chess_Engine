# Chess Engine — Master Plan

## Vision

A competitive chess engine written in Rust, built from first principles. The engine targets UCI compatibility, bitboard-based move generation, alpha-beta search with modern enhancements, and a hand-crafted evaluation function — with a path toward NNUE integration. Target: 2200+ Elo with classical evaluation, 2500+ with NNUE.

## Non-Goals

- GUI (the engine communicates via UCI — use Arena, CuteChess, or Lichess for display)
- Online play server (that is the Chess platform project)
- Opening book authoring tools
- Endgame tablebase generation (we consume Syzygy tablebases, not generate them)
- Training infrastructure for NNUE (we use existing tooling like bullet or nnue-pytorch)

---

## Tech Stack

- **Language**: Rust (2021 edition, stable toolchain)
- **Build**: Cargo workspaces
- **Testing**: Built-in `#[cfg(test)]` + `cargo test`, plus a `perft` binary for move generation validation
- **Benchmarking**: `criterion` for microbenchmarks, perft nodes/sec for move generation throughput
- **CI**: GitHub Actions (clippy, fmt, test, perft regression)
- **Protocol**: UCI (Universal Chess Interface)

## Project Structure

```
chess-engine/
├── crates/
│   ├── types/          # Core types: Square, Piece, Color, Move, Bitboard
│   ├── board/          # Position representation, make/unmake move, Zobrist hashing
│   ├── movegen/        # Legal move generation (bitboards, magic bitboards)
│   ├── eval/           # Evaluation function (material, PST, pawn structure, king safety)
│   ├── search/         # Alpha-beta, iterative deepening, transposition table, move ordering
│   ├── uci/            # UCI protocol parser and engine loop
│   └── engine/         # Top-level binary — wires everything together
├── tests/              # Integration tests (perft suites, known-position checks)
├── benches/            # Criterion benchmarks
├── tools/              # Utility binaries (perft runner, FEN debugger, tuner)
├── MASTER_PLAN.md
├── AGENTS.md
├── Cargo.toml          # Workspace root
└── .github/workflows/  # CI
```

## Coding Conventions

- **No `unsafe`** unless provably necessary for performance (magic bitboards, SIMD). Each `unsafe` block requires a `// SAFETY:` comment.
- **No `unwrap()` in library crates.** Use `Result` or `Option` propagation. `unwrap()` allowed only in binaries and tests.
- **Naming**: `snake_case` functions/variables, `PascalCase` types, `SCREAMING_SNAKE` constants.
- **Bitboard type**: Newtype wrapper `struct Bitboard(u64)` with operator overloads (`BitAnd`, `BitOr`, `BitXor`, `Not`, `Shl`, `Shr`).
- **Move encoding**: 16-bit packed integer — `from(6) | to(6) | flags(4)`. Flags encode promotion piece, castling, en passant, double pawn push.
- **Square indexing**: Little-endian rank-file mapping (a1 = 0, h8 = 63).
- **Tests**: Unit tests colocated in source files via `#[cfg(test)] mod tests`. Integration tests in `tests/`.
- **Documentation**: `///` doc comments on all public items. No comments restating what the code does — only explain *why*.
- **Error handling**: `thiserror` for library errors, `anyhow` for binary entry points.
- **Dependencies**: Minimize external crates. Core engine logic should depend only on the standard library.
- **Formatting**: `rustfmt` defaults. `clippy` with `-D warnings` in CI.

---

## Milestones

### Milestone M01 — Core Types & Board Representation

Establish the foundational data types and bitboard-based position representation. This is the substrate everything else builds on — correctness here is non-negotiable.

#### Phase P01 — Primitive Types

- **T01**: `Bitboard` newtype with arithmetic/bitwise operator overloads, bit iteration, population count, LSB/MSB extraction
- **T02**: `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank` enums with conversion traits and safe indexing
- **T03**: `Move` as a packed `u16` with encode/decode for from, to, flags (promotion, castling, en passant, double push)

#### Phase P02 — Position State

- **T01**: `Position` struct — 12 piece bitboards (one per piece-color), occupancy bitboards, side to move, castling rights, en passant square, halfmove clock, fullmove counter
- **T02**: FEN parser and serializer (`Position::from_fen`, `Position::to_fen`) with full validation
- **T03**: Zobrist hashing — random keys for piece-square, side-to-move, castling, en passant; incremental hash update

#### Phase P03 — Position Manipulation

- **T01**: `make_move` — apply a move to the position, updating all bitboards, castling rights, en passant, clocks, and Zobrist hash. Return captured piece info for unmake.
- **T02**: `unmake_move` — restore previous position state using captured piece info. Must be exact inverse of `make_move`.
- **T03**: `is_square_attacked` — determine if a given square is attacked by a given side. Foundation for legality checks and king safety.

---

### Milestone M02 — Move Generation

Complete legal move generation for all piece types, including special moves. Validated by perft to match known reference counts.

#### Phase P01 — Non-Sliding Pieces

- **T01**: Pawn move generation — single push, double push, captures, en passant, promotions (4 piece types). Separate logic for white/black direction.
- **T02**: Knight move generation — pre-computed attack tables indexed by square.
- **T03**: King move generation — pre-computed attack tables, excluding squares attacked by opponent.

#### Phase P02 — Sliding Pieces & Magic Bitboards

- **T01**: Magic bitboard infrastructure — blocker mask generation, magic number discovery (or use pre-computed magics), attack table initialization.
- **T02**: Bishop and rook move generation using magic bitboard lookup.
- **T03**: Queen move generation (bishop | rook attacks). Combined sliding piece interface.

#### Phase P03 — Castling, Legality & Perft

- **T01**: Castling move generation — check king/rook haven't moved, no pieces between, no squares attacked, generate O-O and O-O-O.
- **T02**: Legal move filter — generate pseudo-legal moves, filter those that leave king in check. Efficient pin detection.
- **T03**: `perft` function and test suite — validate against standard positions (startpos, Kiwipete, etc.) to depth 5+. This is the correctness gate for all of movegen.

---

### Milestone M03 — Basic Search & Evaluation

A playable engine: it can search positions, evaluate them, and pick reasonable moves. Not strong yet, but functional end-to-end.

#### Phase P01 — Material Evaluation & Piece-Square Tables

- **T01**: Material counting evaluation (standard piece values: P=100, N=320, B=330, R=500, Q=900).
- **T02**: Piece-square tables for all piece types — separate middlegame and endgame tables. Tapered evaluation interpolating between game phases based on remaining material.
- **T03**: Game phase calculation — compute a phase score from remaining material to weight middlegame vs endgame PST values.

#### Phase P02 — Negamax with Alpha-Beta

- **T01**: Negamax search framework with alpha-beta pruning. Returns `(score, best_move)`.
- **T02**: Quiescence search — extend search with captures and promotions beyond the main search depth to avoid horizon effect. Stand-pat evaluation.
- **T03**: Iterative deepening — search depth 1, 2, 3, ... collecting best move at each depth. Basic time management (fixed time per move initially).

#### Phase P03 — Move Ordering (Basic)

- **T01**: MVV-LVA (Most Valuable Victim, Least Valuable Attacker) capture ordering.
- **T02**: Killer move heuristic — store 2 killer moves per ply that caused beta cutoffs.
- **T03**: PV move ordering — store principal variation from previous iteration, search PV move first.

---

### Milestone M04 — UCI Protocol

Make the engine usable with standard chess GUIs and testing tools.

#### Phase P01 — UCI Core

- **T01**: UCI command parser — parse `uci`, `isready`, `ucinewgame`, `position`, `go`, `stop`, `quit` commands.
- **T02**: UCI output — `id`, `uciok`, `readyok`, `bestmove`, `info` (depth, score, nodes, nps, pv, time).
- **T03**: Engine main loop — stdin/stdout communication, search thread management, `stop` signal handling via atomic flag.

#### Phase P02 — Time Management

- **T01**: Time allocation — given `wtime`, `btime`, `winc`, `binc`, `movestogo`, calculate time budget for this move. Simple formula: `time_left / moves_to_go + increment`.
- **T02**: Search integration — check elapsed time periodically during search (every N nodes), abort when time budget exceeded. Return best move found so far.
- **T03**: UCI `go` variants — support `depth`, `nodes`, `movetime`, `infinite`, `ponder` modes. Ensure `stop` always responds with `bestmove`.

---

### Milestone M05 — Transposition Table & Hash

Massive search efficiency gain. The transposition table avoids re-searching positions reached via different move orders.

#### Phase P01 — Transposition Table

- **T01**: TT entry structure — Zobrist key (or verification bits), depth, score, bound type (exact/lower/upper), best move, age.
- **T02**: TT implementation — fixed-size hash table with replacement policy (prefer deeper entries, age-based eviction). Power-of-two sizing for fast modulo.
- **T03**: TT integration into search — probe before searching, store after searching. Use TT move for move ordering (highest priority). Handle score adjustment for mate scores stored at different plies.

#### Phase P02 — Search Enhancements (TT-Dependent)

- **T01**: TT-based move ordering — TT hit provides best move from prior search; place it first in move list.
- **T02**: Internal iterative deepening (IID) — if no TT move available at high depth, do a shallower search first to find a move to order first.
- **T03**: Repetition detection — track position history via Zobrist hashes. Draw by threefold repetition and fifty-move rule. Integrate into search scoring.

---

### Milestone M06 — Advanced Search Techniques

Push search depth and efficiency significantly. Each technique prunes large portions of the search tree.

#### Phase P01 — Pruning Techniques

- **T01**: Null move pruning — skip a move (give opponent two moves in a row); if the resulting score still exceeds beta, prune. Disable in endgame/zugzwang-prone positions.
- **T02**: Late move reductions (LMR) — reduce search depth for moves ordered late in the move list (likely bad moves). Re-search at full depth if reduced search returns above alpha.
- **T03**: Futility pruning — near leaf nodes, if static eval + margin is below alpha, skip quiet moves. Reverse futility pruning at pre-frontier nodes.

#### Phase P02 — Move Ordering Improvements

- **T01**: History heuristic — track which quiet moves cause beta cutoffs; use accumulated scores to order quiet moves.
- **T02**: Counter-move heuristic — for each (piece, to-square) of the previous move, store the move that refuted it. Use as a move ordering signal.
- **T03**: Static exchange evaluation (SEE) — evaluate a capture sequence to determine if a capture is winning, losing, or equal. Use to prune bad captures and order good ones.

#### Phase P03 — Search Extensions

- **T01**: Check extension — extend search by one ply when the side to move is in check (avoid missing forced tactics).
- **T02**: Singular extension — if the TT move is significantly better than all alternatives (verified by a reduced-depth search excluding it), extend it by one ply.
- **T03**: Principal Variation Search (PVS) — search first move with full window, remaining moves with zero-window. Re-search with full window on fail-high. Replaces plain alpha-beta.

---

### Milestone M07 — Advanced Evaluation

Transform the evaluation function from basic material + PST to positional understanding.

#### Phase P01 — Pawn Structure

- **T01**: Pawn hash table — cache pawn structure evaluation keyed by pawn Zobrist hash. Pawn structure changes infrequently, so cache hit rate is very high.
- **T02**: Pawn structure terms — doubled pawns (penalty), isolated pawns (penalty), backward pawns (penalty), passed pawns (bonus scaled by advancement), connected pawns (bonus).
- **T03**: Passed pawn evaluation — bonus for advancement rank, bonus for king proximity (own king close, opponent king far), penalty if blocked.

#### Phase P02 — King Safety & Piece Activity

- **T01**: King safety — pawn shield evaluation (bonus for pawns in front of king), open file penalty near king, virtual mobility of attacking pieces toward king zone.
- **T02**: Mobility evaluation — count legal/pseudo-legal moves per piece (excluding squares attacked by opponent pawns). Scale bonus by piece type.
- **T03**: Bishop pair bonus, rook on open/semi-open file bonus, rook on seventh rank bonus. Knight outpost detection (knight on opponent's side, supported by pawn, can't be attacked by opponent pawn).

#### Phase P03 — Endgame Knowledge

- **T01**: Specialized endgame evaluation — KBN vs K, KR vs K, KQ vs K mating patterns. Detect drawn endgames (insufficient material).
- **T02**: King centralization in endgame — bonus for king advancing toward center as material decreases. Encourage king activity in pawn endgames.
- **T03**: Tapered evaluation tuning — adjust middlegame/endgame weights and phase calculation. Ensure smooth interpolation across all evaluation terms.

---

### Milestone M08 — Testing, Tuning & Strength Validation

Systematic verification and parameter optimization.

#### Phase P01 — Test Infrastructure

- **T01**: Extended perft suite — validate movegen against 100+ positions from established test suites. Regression-proof the move generator.
- **T02**: Tactical test suite — WAC (Win At Chess), STS (Strategic Test Suite), ECM (Encyclopedia of Chess Middlegames). Measure solve rate at fixed depth/time.
- **T03**: Self-play framework — engine plays itself with randomized openings. Detect crashes, illegal moves, assertion failures, and time losses over thousands of games.

#### Phase P02 — Parameter Tuning

- **T01**: Texel tuning infrastructure — given a set of positions with game outcomes (win/draw/loss), optimize evaluation weights by minimizing MSE between predicted win probability and actual outcome.
- **T02**: Tune material values, PST values, and all evaluation weights using Texel method on a large dataset (e.g., CCRL games or Lichess database positions).
- **T03**: Search parameter tuning — LMR reduction table, null move reduction depth, futility margins, aspiration window sizes. Validate via self-play SPRT testing.

#### Phase P03 — Strength Measurement

- **T01**: CuteChess-CLI integration — automated tournament runner against reference engines (Stockfish at reduced depth, Ethereal, Rustic, etc.).
- **T02**: SPRT testing framework — Statistical Sequential Probability Ratio Test for A/B testing engine changes. Determine if a change gains or loses Elo with statistical confidence.
- **T03**: Regression test suite — automated CI job that runs a short tournament after each PR to catch strength regressions.

---

### Milestone M09 — Opening Book & Endgame Tablebases

External knowledge integration for stronger play in known positions.

#### Phase P01 — Polyglot Opening Book

- **T01**: Polyglot `.bin` book format parser — read entries, hash positions with Polyglot Zobrist keys, look up book moves.
- **T02**: Book move selection — weighted random selection by frequency/weight. Support for `bestmove` (always pick highest weight) and `weighted` modes.
- **T03**: UCI option for book path and mode. Integrate into engine loop — consult book before searching.

#### Phase P02 — Syzygy Endgame Tablebases

- **T01**: Syzygy tablebase probing — integrate `Fathom` (C library) or a Rust port for WDL (win/draw/loss) and DTZ (distance to zeroing) probes.
- **T02**: Search integration — at root, use tablebase WDL to filter losing moves. During search, use WDL probes at low piece counts to return exact scores.
- **T03**: UCI options for Syzygy path and probe depth. Lazy loading of tablebase files.

---

### Milestone M10 — NNUE Evaluation

Replace or augment the handcrafted evaluation with a neural network for a major strength jump.

#### Phase P01 — NNUE Architecture

- **T01**: NNUE network architecture — HalfKP (or HalfKAv2) input features, accumulator structure, quantized weights. Define the network topology (input → hidden1 → hidden2 → output).
- **T02**: Inference engine — forward pass with incremental accumulator updates. Use SIMD (SSE2/AVX2/NEON) intrinsics for vectorized integer arithmetic.
- **T03**: Network file format — load `.nnue` weights from binary file. Support Stockfish-compatible format or define custom format.

#### Phase P02 — Training & Integration

- **T01**: Training data generation — use the handcrafted engine to generate self-play games with position evaluations. Export as training data (FEN + eval + game result).
- **T02**: Training pipeline — use `bullet` or `nnue-pytorch` to train the network on generated data. Iterate: generate data → train → embed → test → repeat.
- **T03**: Evaluation switchover — NNUE evaluation replaces handcrafted eval in search. Handcrafted eval remains as fallback. UCI option to select evaluation mode.

#### Phase P03 — NNUE Optimization

- **T01**: Accumulator incremental updates — only recompute affected features on make/unmake. Track dirty accumulators to minimize recomputation.
- **T02**: SIMD optimization — vectorized clipped-ReLU, dot products, and accumulator updates for target architectures (x86-64 AVX2, ARM NEON).
- **T03**: Network distillation — train a smaller, faster network that approximates the larger network. Measure Elo vs NPS tradeoff. Ship the best balance.

---

### Milestone M11 — Polish & Release

Production readiness: packaging, documentation, and community integration.

#### Phase P01 — Build & Distribution

- **T01**: Cross-compilation — CI builds for x86-64 (Linux, macOS, Windows) and ARM64 (macOS, Linux). Feature flags for SIMD level (SSE2, AVX2, NEON).
- **T02**: Binary packaging — GitHub Releases with pre-built binaries. Include embedded NNUE weights in the binary (or as a companion file).
- **T03**: Versioning — SemVer, changelog generation from git history, release automation via GitHub Actions.

#### Phase P02 — Documentation & Community

- **T01**: README — features, build instructions, UCI options, benchmark results, Elo estimates.
- **T02**: Architecture documentation — design decisions, data structures, search algorithm explanations. Aimed at other engine authors and contributors.
- **T03**: Lichess Bot integration — deploy as a Lichess bot using the `lichess-bot` bridge. Configure time controls and challenge acceptance. This gives the engine real opponents and a public rating.

---

## Milestone Summary

| Milestone | Focus | Elo Target |
|-----------|-------|------------|
| M01 | Core types & board representation | — |
| M02 | Move generation (bitboards, magics, perft) | — |
| M03 | Basic search & evaluation | ~1200 |
| M04 | UCI protocol | ~1200 (playable) |
| M05 | Transposition table & hashing | ~1500 |
| M06 | Advanced search (pruning, extensions, PVS) | ~1800 |
| M07 | Advanced evaluation | ~2000 |
| M08 | Testing, tuning & validation | ~2200 |
| M09 | Opening book & endgame tablebases | ~2300 |
| M10 | NNUE evaluation | ~2500+ |
| M11 | Polish, packaging & release | — |

## Dependencies Between Milestones

```
M01 → M02 → M03 → M04
                 ↘      ↘
                  M05 → M06 → M07 → M08 → M09
                                          ↘
                                           M10 → M11
```

M01–M04 are strictly sequential (each builds on the previous). M05–M09 build incrementally on the search/eval foundation. M10 (NNUE) requires a working engine from M08. M11 (release) comes last.

---

## AI Delivery Model

This project uses the **Iteration Engine** (`/Users/theodhor/Desktop/Portfolio/Iteration_Engine`). The pipeline orchestrates Opus (planning/review) and implementation agents through the milestone → phase → task hierarchy defined above.

- **Planning**: Opus drafts specs, a challenger reviews, iterate until locked
- **Implementation**: Agent implements against the locked plan, Opus reviews code
- **Git**: One branch per phase, auto-commit per task, PR per phase for human review
- **Quality gate**: Perft validation after M02, tactical test suites after M03+, SPRT testing after M08+

### Pipeline Configuration

```json
{
  "project": "chess-engine",
  "masterPlanFile": "MASTER_PLAN.md",
  "pipelineDir": ".pipeline",
  "git": {
    "enabled": true,
    "branchPrefix": "phase/",
    "autoCommit": true,
    "autoPR": true
  },
  "preflight": {
    "enabled": true,
    "commands": ["cargo fmt --check", "cargo clippy -- -D warnings", "cargo test"]
  }
}
```
