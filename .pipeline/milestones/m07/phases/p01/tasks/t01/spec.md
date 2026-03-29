# T01 — Pawn Hash Table

## Objective

Implement the pawn hash table data structure and its probe/store interface within the `eval` crate. This is a fixed-size cache that stores pawn structure evaluation results keyed by a pawn-specific Zobrist hash, avoiding redundant pawn evaluation computation.

## Deliverables

- A `PawnHashEntry` struct storing: pawn Zobrist key (or verification bits), midgame score (`i32`), endgame score (`i32`)
- A `PawnHashTable` struct with a fixed-size `Vec<PawnHashEntry>` (default 16K entries, power-of-two sizing)
- A function to compute the pawn Zobrist hash from a `Position` by XOR-ing `PIECE_SQUARE_KEYS` entries (from `crates/board/src/zobrist.rs`) for all white and black pawns on the board. No new random key tables are introduced.
- `probe(&self, key: u64) -> Option<(i32, i32)>` — returns cached (mg, eg) scores on key match; returns `None` on miss
- `store(&mut self, key: u64, mg: i32, eg: i32)` — writes a new entry at the index determined by `key & (size - 1)`
- Unit tests verifying:
  - Probe miss returns `None`
  - Store followed by probe returns correct scores
  - Different pawn configurations produce different hash keys

## Files to Create or Modify

- **Create**: `crates/eval/src/pawn.rs` — new module containing `PawnHashTable`, `PawnHashEntry`, and pawn Zobrist hash computation
- **Modify**: `crates/eval/src/lib.rs` — add `mod pawn;` declaration (the full integration into `evaluate()` happens in T03)

## Key Implementation Details

- Use power-of-two table sizing with bitwise AND masking (`hash & (size - 1)`) for index computation, consistent with the transposition table pattern from M05
- The pawn Zobrist hash is derived from existing `PIECE_SQUARE_KEYS` in `crates/board/src/zobrist.rs` — iterate over both white and black pawn bitboards, XOR-ing the corresponding piece-square key for each pawn
- Each entry stores both midgame and endgame scores so cached results integrate directly into the tapered evaluation framework without recomputation
- The `PIECE_SQUARE_KEYS` array is in `crates/board/src/zobrist.rs`; ensure the pawn hash computation has access to it (it may need to be `pub`)

## Dependencies

- No dependencies on other tasks within this phase (this is the first task)
- Depends on existing `board` crate (`Position` struct, `PIECE_SQUARE_KEYS` in `crates/board/src/zobrist.rs`)
- Depends on existing `types` crate (`Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`)

## Verification Criteria

1. `PawnHashTable::new()` creates a table with 16K entries (or configurable power-of-two size)
2. `probe()` on an empty/unmatched table returns `None`
3. After `store(key, mg, eg)`, `probe(key)` returns `Some((mg, eg))` with the correct values
4. Computing the pawn Zobrist hash for two different pawn configurations yields different keys
5. `cargo test` passes for all new tests in the `eval` crate
6. `cargo clippy -- -D warnings` produces no warnings for the `eval` crate