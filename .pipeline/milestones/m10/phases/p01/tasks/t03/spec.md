## Task T03 — Network File Format and Weight Loading

### Objective

Implement the binary `.nnue` file loader that reads network weights from disk into the in-memory network structure. Define or adopt the file format (Stockfish-compatible or custom), including header validation, endianness handling, and layer dimension verification.

### Deliverables

- `load` function (or equivalent) that reads a `.nnue` file from a path and returns a populated `Network` structure
- Binary file format specification — either Stockfish-compatible or custom format, documented with:
  - Header layout (magic bytes, version, architecture hash)
  - Layer ordering and dimension encoding
  - Endianness convention
  - Quantization parameters
- Header validation on load — reject files with wrong magic, incompatible version, or mismatched architecture
- Layer dimension verification — ensure loaded dimensions match the compiled network architecture from T01
- Error handling for malformed, truncated, or incompatible files (return descriptive errors, no panics)
- Test fixture `.nnue` file (can be a minimal synthetic file with known weights for testing)

### Files to Create or Modify

- **Create** `crates/nnue/src/format.rs` (or similar) — file format constants, header struct, serialization/deserialization
- **Create** `crates/nnue/src/loader.rs` (or similar) — `load` function, validation, file reading logic
- **Create** test fixture file(s) for weight loading tests (e.g., in `crates/nnue/tests/` or `crates/nnue/src/testdata/`)
- **Modify** `crates/nnue/src/lib.rs` — add module declarations, public re-exports for the load function
- **Modify** `crates/nnue/Cargo.toml` — if additional dependencies are needed (e.g., `byteorder` for endianness, though `std` may suffice)

### Implementation Details & Constraints

- The format decision (Stockfish-compatible vs custom) should be documented with rationale in T03
- Endianness: if custom format, use little-endian (matches x86 native byte order); if Stockfish-compatible, match their format
- The loader must validate that the file's layer dimensions match the `Network` architecture constants defined in T01
- Use `std::io::Read` / `std::io::BufReader` for file reading
- Error types should use `thiserror` per project convention
- No `unwrap()` in library code — all I/O and parsing errors must be propagated via `Result`
- Minimize external dependencies — prefer `std` for byte manipulation where feasible

### Dependencies

- **T01** — requires the network architecture types and weight storage structures to populate
- **T02** — the loaded weights must be compatible with the inference engine's forward pass; verification can use the inference engine to confirm loaded weights produce expected output

### Verification Criteria

1. `load` function successfully reads a valid `.nnue` file and returns a populated `Network` structure
2. Loading a file with incorrect magic bytes returns an appropriate error
3. Loading a file with mismatched layer dimensions returns an appropriate error
4. Loading a truncated file returns an appropriate error (does not panic)
5. Loaded weights, when used with the inference engine from T02, produce the expected evaluation score for a reference position
6. `cargo test -p nnue` passes, covering all loading success and error scenarios
7. `cargo fmt --check` and `cargo clippy -- -D warnings` pass
8. File format specification is documented in code comments or a module-level doc comment