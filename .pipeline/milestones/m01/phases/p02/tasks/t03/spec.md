# T03 — Zobrist Hashing

## Objective

Implement Zobrist key generation, full hash computation from position state, and incremental hash update support — providing efficient position identification that will be used by the transposition table, repetition detection, and pawn hash table in later milestones.

## Deliverables

- **Pre-generated random `u64` Zobrist keys** for:
  - Piece-square combinations: 12 pieces × 64 squares = 768 keys
  - Side-to-move: 1 key
  - Castling rights: 16 keys (one per combination of the 4 castling rights flags)
  - En passant file: 8 keys (one per file)
- Keys must be **fixed values** (compile-time constants or lazily initialized) to ensure deterministic hashing across runs
- **`Position` gains a `hash` field** (`u64`) representing the current Zobrist hash
- **Full hash computation function** — compute the complete Zobrist hash from scratch given a position's state (piece placements, side to move, castling rights, en passant)
- **Incremental update methods** — update the hash by XOR-ing in/out changed keys for:
  - Piece placement changes (add/remove a piece on a square)
  - Side-to-move toggle
  - Castling rights changes
  - En passant file changes
- **Integration with existing constructors** — `Position::from_fen` and `Position::startpos` must compute and store the initial Zobrist hash
- **Unit tests** verifying:
  - Identical positions produce identical hashes (construct same position via different paths)
  - Incremental updates match full recomputation (modify position, compare incremental hash vs recomputed hash)
  - Different positions produce different hashes (spot-checked across starting position, middlegame, and endgame positions — not a universal guarantee, but verified for known distinct positions)

## Files to Create or Modify

- `crates/board/src/zobrist.rs` — Zobrist key tables (constants or lazy static), full hash computation function, incremental update methods
- `crates/board/src/lib.rs` — add `mod zobrist;` and re-export public items
- `crates/board/src/position.rs` — add `hash: u64` field to `Position`, initialize it in constructors
- `crates/board/src/fen.rs` — update `from_fen` to compute and store the initial Zobrist hash after parsing

## Constraints

- Zobrist keys must be deterministic — same keys every run (no runtime random generation with varying seeds)
- Use XOR for all hash updates (standard Zobrist technique)
- No `unwrap()` in library code
- Minimize external dependencies — Zobrist key generation can use a fixed-seed PRNG or hardcoded constants
- Follow project coding conventions

## Dependencies on Prior Tasks

- **T01 (Position Struct)** must be complete — needs the `Position` struct and its fields
- **T02 (FEN Parser and Serializer)** must be complete — the Zobrist hash must be integrated into `from_fen` and `startpos`, so FEN parsing must already work

## Verification Criteria

1. `Position::startpos().hash` is a non-zero `u64` value and is identical across multiple calls
2. Two positions constructed from the same FEN produce identical hashes
3. Positions constructed from different FENs produce different hashes (verified for at least 5 distinct positions)
4. After making incremental updates to the hash (e.g., toggling side-to-move, adding/removing pieces), the resulting hash matches a full recomputation from the modified position state
5. `Position::from_fen` and `Position::startpos` both correctly compute and store the initial hash
6. All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`