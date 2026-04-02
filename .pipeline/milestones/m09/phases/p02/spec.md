## Phase P02 — Syzygy Endgame Tablebases

### Goal

Integrate Syzygy endgame tablebase probing into the engine so it achieves perfect play in low-piece-count endgames. This involves interfacing with a tablebase probing library (Fathom C library or Rust port), using WDL/DTZ values at the root to filter losing moves, probing during search for exact scores, and exposing configuration through UCI options with lazy-loaded tablebase files.

### Deliverables

- **T01 — Syzygy tablebase probing**: Integrate the `Fathom` C library (via FFI) or a Rust port to perform WDL (win/draw/loss) and DTZ (distance to zeroing move) probes for positions at supported piece counts (typically 3–7 pieces).
- **T02 — Search integration**: At the root, use tablebase WDL results to filter out losing moves so the engine never plays a theoretically losing move when tablebase data is available. During search, use WDL probes at low piece counts to return exact scores, short-circuiting the evaluation.
- **T03 — UCI options and lazy loading**: Add UCI options for the Syzygy tablebase directory path and a configurable probe depth (maximum number of pieces for which to probe during search). Implement lazy loading so tablebase files are only memory-mapped or loaded when first needed, avoiding startup delay.

### Technical Decisions & Constraints

- The primary integration path is the `Fathom` C library via Rust FFI (`cc` or `bindgen` crate), though a pure Rust port is an acceptable alternative.
- If using Fathom via FFI, `unsafe` blocks are required and must include `// SAFETY:` comments per project convention.
- WDL probes are cheaper than DTZ probes; use WDL during search and DTZ primarily at the root for selecting the fastest winning move.
- Tablebase probing should be gated by piece count to avoid unnecessary overhead in positions with too many pieces.
- Lazy loading avoids memory and startup costs when tablebases are configured but not yet needed.
- External Syzygy tablebase files (3–5 piece minimum) are required for testing.
- Mate scores from tablebase results must be correctly converted to the engine's score representation.

### Dependencies

- **Phase P01** (Polyglot Opening Book) — not a hard technical dependency, but follows in milestone ordering.
- **M08** (Testing, Tuning & Strength Validation) must be complete — functioning search and evaluation.
- **M04** (UCI Protocol) must be complete — UCI option support.
- **M05** (Transposition Table & Hash) must be complete — Zobrist hashing for search integration.
- External: Fathom C library source or Rust Syzygy probing crate.
- External: Syzygy `.rtbw` (WDL) and `.rtbz` (DTZ) tablebase files for testing.