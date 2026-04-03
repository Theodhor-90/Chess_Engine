# T01 — Syzygy Tablebase Probing

## Objective

Integrate the Fathom C library (via FFI) or a Rust port to perform WDL (win/draw/loss) and DTZ (distance to zeroing move) probes for positions at supported piece counts. Provide an initialization function accepting a directory path and a probe interface that takes a `Position` and returns WDL/DTZ results.

## Deliverables

- A new crate or module for Syzygy tablebase probing (e.g., `crates/syzygy/` or a module within an existing crate)
- FFI bindings to the Fathom C library compiled via the `cc` crate, OR integration of a pure Rust Syzygy probing crate with equivalent WDL and DTZ functionality
- An initialization function that accepts a directory path to Syzygy tablebase files (`.rtbw` for WDL, `.rtbz` for DTZ)
- A probe interface that takes a `Position` reference and returns WDL and DTZ results
- Score conversion: WDL results mapped to the engine's internal score representation, with wins/losses mapped to mate-scale scores adjusted for distance to ensure correct search behavior
- Piece count gating: probing gated by the number of pieces on the board versus a maximum supported piece count

## Key Implementation Details

- **Probing library choice**: The primary path is Fathom C library via Rust FFI. Use the `cc` crate to compile Fathom from source and provide a thin `unsafe` wrapper. A pure Rust Syzygy crate is acceptable if it provides equivalent WDL and DTZ functionality. Finalize this decision during implementation.
- **WDL vs DTZ usage**: WDL probes are cheaper than DTZ probes. WDL is used during search for exact win/draw/loss scores. DTZ is used at the root for selecting the fastest winning move.
- **FFI safety**: All `unsafe` blocks must include `// SAFETY:` comments per project coding conventions.
- **No `unwrap()` in library crates** — use `Result` or `Option` propagation.
- **Dependencies**: Minimize external crates. Core probing logic should be well-encapsulated.

## Files to Create or Modify

- Create: New crate or module for Syzygy probing (e.g., `crates/syzygy/src/lib.rs`)
- Create: FFI bindings module (if using Fathom)
- Create: `build.rs` for compiling Fathom C source (if using Fathom via `cc` crate)
- Modify: `Cargo.toml` (workspace root) — add the new crate to workspace members

## Dependencies

- None within this phase (this is the first task)
- External: Fathom C library source or equivalent Rust Syzygy probing crate
- External: Syzygy `.rtbw` and `.rtbz` tablebase files (3–5 piece minimum) for testing
- Prior milestones complete: M04 (UCI), M05 (TT & Zobrist hashing), M08 (testing infrastructure)

## Verification Criteria

1. The probing module compiles without errors or warnings (`cargo clippy -- -D warnings`)
2. WDL probes return correct win/draw/loss values for known endgame positions (e.g., KRK = win for side with rook, KK = draw)
3. DTZ probes return correct distance-to-zeroing values for known positions
4. Score conversion correctly maps WDL results to engine-internal mate-scale scores
5. Piece count gating correctly prevents probing when piece count exceeds the supported limit
6. All `unsafe` blocks have `// SAFETY:` comments
7. Unit tests pass: `cargo test` for the new crate/module
8. `cargo fmt --check` passes