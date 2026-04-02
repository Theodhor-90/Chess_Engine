# T01 — Polyglot `.bin` Book Format Parser

## Objective

Implement the Polyglot binary book format reader and Polyglot-specific Zobrist key hashing so the engine can load a `.bin` opening book and look up moves for any given position.

## Deliverables

- **Polyglot Zobrist key constants**: The 781 random values defined by the Polyglot specification, plus a function `polyglot_hash(position: &Position) -> u64` that computes a Polyglot-format hash from the engine's `Position`.
- **Book entry struct**: A `PolyglotEntry` representing one 16-byte record containing fields: `key: u64`, `move_raw: u16`, `weight: u16`, `learn: u32`.
- **Binary `.bin` file reader**: Loads all entries from a Polyglot `.bin` file into a sorted in-memory `Vec<PolyglotEntry>`. The file is read entirely into memory on initialization (books are typically a few MB).
- **Binary search lookup**: Given a Polyglot key, returns all matching `PolyglotEntry` records. Uses binary search to find the first matching key, then scans forward to collect all entries for that position.
- **Move translation function**: Converts a Polyglot move encoding (from-square, to-square, promotion piece) to the engine's internal `Move` type (packed `u16`). Must handle castling conventions — Polyglot uses king-to-rook-square encoding which may differ from the engine's convention.
- **Unit tests**: Validate Polyglot key computation against known reference hashes (e.g., the starting position Polyglot key `0x463b96181691fc9c`), correct entry parsing from binary data, and move translation correctness.

## Key Implementation Details

- Polyglot defines its own Zobrist key scheme that is **distinct from the engine's internal Zobrist hashing** (in `crates/board`). Do not reuse or modify the engine's existing Zobrist keys.
- The `.bin` format stores 16-byte records sorted by Zobrist key. Each record is big-endian.
- Code location: `crates/engine` crate (or a dedicated `book` module within it), since it bridges book files, position hashing, and UCI. Depends on `types` and `board` crates.

## Files to Create or Modify

- Create: `crates/engine/src/book/mod.rs` (or similar module path) — Polyglot Zobrist constants, hash function, entry struct, binary reader, lookup, move translation
- Modify: `crates/engine/src/lib.rs` — add `pub mod book;` declaration
- Modify: `crates/engine/Cargo.toml` — if any new dependencies are needed (likely none; only std library I/O)

## Dependencies

- No dependencies on other tasks within this phase (T01 is the foundation).
- Requires types from `crates/types` (`Square`, `Move`, `Piece`, `Color`, etc.).
- Requires `Position` from `crates/board` for hash computation.

## Verification Criteria

1. `cargo test` passes all new unit tests.
2. Polyglot hash of the starting position equals the known reference value `0x463b96181691fc9c`.
3. A test `.bin` file (or synthetic binary data) is correctly parsed into `PolyglotEntry` records.
4. Binary search lookup returns the correct set of entries for a given position key.
5. Polyglot move encoding is correctly translated to the engine's `Move` type, including castling move conventions.
6. `cargo clippy -- -D warnings` and `cargo fmt --check` pass.