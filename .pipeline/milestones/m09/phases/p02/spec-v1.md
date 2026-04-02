# Phase P02 — Syzygy Endgame Tablebases

## Goal

Integrate Syzygy endgame tablebase probing into the engine so it achieves perfect play in low-piece-count endgames, by interfacing with a tablebase probing library for WDL and DTZ lookups, using tablebase results at the root to filter losing moves and during search to return exact scores, and exposing Syzygy configuration through UCI options with lazy-loaded tablebase files.

## Design Decisions

1. **Probing library**: The primary integration path is the Fathom C library via Rust FFI (using the `cc` crate to compile Fathom from source and a thin `unsafe` wrapper). A pure Rust Syzygy probing crate is an acceptable alternative if it provides equivalent WDL and DTZ functionality. This decision will be finalized in T01.

2. **WDL vs DTZ probe usage**: WDL probes are cheaper than DTZ probes. WDL probes are used during search to return exact win/draw/loss scores. DTZ probes are used at the root to select the fastest winning move or longest-to-lose move among WDL-equivalent options.

3. **Piece count gating**: Tablebase probing is gated by the number of pieces on the board versus a configurable probe depth limit, avoiding unnecessary overhead in positions with too many pieces.

4. **Lazy loading**: Tablebase files are loaded or memory-mapped on first probe rather than at engine startup, avoiding memory and latency costs when tablebases are configured but not yet needed in the current game.

5. **Score conversion**: Tablebase WDL results are converted to the engine's internal score representation, with wins/losses mapped to mate-scale scores adjusted for distance to ensure correct search behavior.

6. **FFI safety**: If using Fathom via FFI, all `unsafe` blocks include `// SAFETY:` comments per project convention.

## Tasks

- **T01 — Syzygy tablebase probing**: Integrate the Fathom C library (via FFI) or a Rust port to perform WDL (win/draw/loss) and DTZ (distance to zeroing move) probes for positions at supported piece counts. Provide an initialization function accepting a directory path and a probe interface that takes a `Position` and returns WDL/DTZ results.

- **T02 — Search integration**: At the root, use tablebase WDL to filter losing moves so the engine never plays a theoretically lost move when tablebase data is available. Use DTZ at the root to prefer the fastest winning path. During search, use WDL probes at low piece counts to return exact scores, short-circuiting evaluation. Gate probing by piece count against the configured probe depth limit.

- **T03 — UCI options and lazy loading**: Add UCI options for the Syzygy tablebase directory path (`SyzygyPath`) and a configurable probe depth (`SyzygyProbeDepth`, maximum piece count for probing during search). Implement lazy loading so tablebase files are only loaded when first probed. Integrate option handling into the engine's `setoption` command processing.

## Exit Criteria

1. Syzygy tablebase probing returns correct WDL and DTZ values for known endgame positions.
2. Root search uses tablebase WDL to filter losing moves at low piece counts.
3. In-search tablebase probes return exact scores at supported piece counts.
4. UCI options control Syzygy tablebase path and probe depth.
5. Tablebase files are lazily loaded (not at startup).

## Dependencies

- **M09/P01** (Polyglot Opening Book) — precedes this phase in milestone ordering.
- **M08** (Testing, Tuning & Strength Validation) — complete; functioning search, evaluation, and testing infrastructure are available.
- **M04** (UCI Protocol) — complete; UCI option support and engine main loop are in place.
- **M05** (Transposition Table & Hash) — complete; Zobrist hashing is available for search integration.
- External: Fathom C library source or equivalent Rust Syzygy probing crate.
- External: Syzygy `.rtbw` (WDL) and `.rtbz` (DTZ) tablebase files (3–5 piece minimum) for testing.

## Artifacts

- New crate or module for Syzygy tablebase probing (FFI bindings or Rust port wrapper)
- Modified `crates/search/` — tablebase probes integrated into root search and in-search logic
- Modified `crates/engine/src/main.rs` — new UCI options (`SyzygyPath`, `SyzygyProbeDepth`), `setoption` handling, and lazy-loading initialization