Now I have a thorough understanding of the entire codebase. Let me now review the Polyglot specification details to ensure accuracy in the plan. The key things I need to understand:

1. **Polyglot square mapping**: Polyglot uses rank-file ordering where a1=0, but the bit encoding for moves is `to_file(3) | to_row(3) | from_file(3) | from_row(3) | promotion(3)`.
2. **Polyglot castling convention**: Polyglot encodes castling as king moves to the rook's square (e.g., e1h1 for O-O, e1a1 for O-O-O), which differs from the engine's convention (e1g1 for O-O, e1c1 for O-O-O).
3. **Polyglot Zobrist keys**: 781 random u64 values defined by the spec — different from the engine's internal Zobrist keys.

Now I have all the information needed to write the task plan.

## 1. Deliverables

- **Create: `crates/engine/src/book/mod.rs`** — Polyglot Zobrist key constants (781 `u64` values per the Polyglot specification), `polyglot_hash` function that computes a Polyglot-format hash from a `Position`, `PolyglotEntry` struct representing a 16-byte record, `PolyglotBook` struct that loads a `.bin` file into a sorted `Vec<PolyglotEntry>` and provides binary-search lookup, and `polyglot_move_to_engine_move` translation function. Unit tests colocated in the module.
- **Create: `crates/engine/src/lib.rs`** — Library root declaring `pub mod book;` to expose the book module.
- **Modify: `crates/engine/Cargo.toml`** — Add `[lib]` section so the crate exposes both a library and a binary.

## 2. Dependencies

- `chess-types` (already a dependency of `crates/engine`) — provides `Square`, `Move`, `MoveFlag`, `Piece`, `PieceKind`, `Color`, `File`, `Rank`, `Bitboard`.
- `chess-board` (already a dependency of `crates/engine`) — provides `Position`, `CastlingRights`.
- `chess-movegen` (already a dependency of `crates/engine`) — needed to validate Polyglot moves against legal moves during translation.
- No new external dependencies required. Only `std` I/O (`std::fs::File`, `std::io::Read`) is needed to load the `.bin` file.

## 3. Implementation Details

### `crates/engine/Cargo.toml`

Add a `[lib]` section so the engine crate has both a library (`src/lib.rs`) and a binary (`src/main.rs`):

```toml
[lib]
name = "chess_engine"
path = "src/lib.rs"
```

The existing `[[bin]]` section remains unchanged.

### `crates/engine/src/lib.rs`

```rust
pub mod book;
```

### `crates/engine/src/book/mod.rs`

#### Polyglot Zobrist Key Constants

The Polyglot specification defines exactly 781 random `u64` values. These are **fixed, well-known values** published in the Polyglot source code — they are NOT generated at compile time. They must be embedded as a literal array.

The 781 keys are organized as:
- `RANDOM_PIECE[768]` — indexed as `[piece_kind * 2 + color][square]` where Polyglot uses the ordering: Black Pawn, White Pawn, Black Knight, White Knight, Black Bishop, White Bishop, Black Rook, White Rook, Black Queen, White Queen, Black King, White King. Each piece has 64 square entries. Total: 12 * 64 = 768.
- `RANDOM_CASTLE[4]` — one per castling right (white kingside, white queenside, black kingside, black queenside).
- `RANDOM_EN_PASSANT[8]` — one per file (A through H).
- `RANDOM_TURN` — a single key XORed when it is white's turn to move.

All 781 values are published constants from the Polyglot source.

#### `polyglot_hash(pos: &Position) -> u64`

Computes the Polyglot Zobrist hash for a given engine `Position`:

1. For each piece on the board, map the engine's `(Color, PieceKind, Square)` to the Polyglot piece index and XOR in the corresponding `RANDOM_PIECE` key. The Polyglot piece indexing is: `kind_offset * 2 + color_offset` where the kind order is Pawn(0), Knight(1), Bishop(2), Rook(3), Queen(4), King(5), and within each kind Black=0, White=1. So: BlackPawn=0, WhitePawn=1, BlackKnight=2, WhiteKnight=3, ... BlackKing=10, WhiteKing=11. Each piece index multiplied by 64 + square index gives the key offset.
2. For castling rights, XOR in `RANDOM_CASTLE[0]` if white can castle kingside, `[1]` if white can castle queenside, `[2]` if black can castle kingside, `[3]` if black can castle queenside.
3. For en passant, XOR in `RANDOM_EN_PASSANT[file]` **only if** there is a legal en passant capture possible (i.e., there exists an enemy pawn adjacent to the en passant target that could actually capture). This matches the Polyglot specification which only hashes en passant if the capture is available.
4. XOR in `RANDOM_TURN` if it is white to move (Polyglot convention: key is present for white's turn).

Polyglot squares use the same little-endian rank-file mapping as the engine (a1=0, h8=63), so no square conversion is needed.

#### `PolyglotEntry`

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolyglotEntry {
    pub key: u64,
    pub raw_move: u16,
    pub weight: u16,
    pub learn: u32,
}
```

Each entry is 16 bytes, big-endian:
- Bytes 0–7: `key` (u64, big-endian)
- Bytes 8–9: `raw_move` (u16, big-endian)
- Bytes 10–11: `weight` (u16, big-endian)
- Bytes 12–15: `learn` (u32, big-endian)

Parsing: read the file into a `Vec<u8>`, iterate in 16-byte chunks, and decode each field using `u64::from_be_bytes`, `u16::from_be_bytes`, `u32::from_be_bytes`.

#### `PolyglotBook`

```rust
pub struct PolyglotBook {
    entries: Vec<PolyglotEntry>,
}
```

- `PolyglotBook::from_file(path: &str) -> std::io::Result<PolyglotBook>` — reads the entire `.bin` file into memory, parses it into a `Vec<PolyglotEntry>`. The entries in a conformant Polyglot book file are already sorted by key.
- `PolyglotBook::probe(&self, key: u64) -> &[PolyglotEntry]` — uses `partition_point` (binary search) to find the first entry with `entry.key >= key`, then scans forward while `entry.key == key`. Returns a slice of matching entries. Returns an empty slice if no match.

#### `polyglot_move_to_engine_move(pos: &mut Position, raw_move: u16) -> Option<Move>`

Decodes the Polyglot move encoding and maps it to the engine's `Move` type:

**Polyglot raw_move bit layout:**
- Bits 0–2: to file (0=A, 7=H)
- Bits 3–5: to row/rank (0=rank 1, 7=rank 8)
- Bits 6–8: from file
- Bits 9–11: from row/rank
- Bits 12–14: promotion piece (0=none, 1=knight, 2=bishop, 3=rook, 4=queen)

Decoding steps:
1. Extract `to_file = raw_move & 7`, `to_row = (raw_move >> 3) & 7`, `from_file = (raw_move >> 6) & 7`, `from_row = (raw_move >> 9) & 7`, `promo = (raw_move >> 12) & 7`.
2. Build `from_sq = Square::from_file_rank(from_file, from_row)` and `to_sq = Square::from_file_rank(to_file, to_row)`.
3. **Handle castling convention**: Polyglot encodes castling as king-to-rook-square. The engine uses king-to-destination-square. If the moving piece is a king and the move goes from e1 to h1, translate to e1→g1 (kingside). From e1 to a1, translate to e1→c1 (queenside). Same for e8→h8 becoming e8→g8, and e8→a8 becoming e8→c8.
4. Generate legal moves for the position via `chess_movegen::generate_legal_moves(pos)` and find the legal move matching `from_sq`, `to_sq`, and promotion piece. This is the safest approach and handles all flag determination (CAPTURE, DOUBLE_PAWN_PUSH, EN_PASSANT, etc.) automatically.
5. Return `Some(matched_move)` if found, `None` otherwise.

#### Unit Tests

All tests colocated in `#[cfg(test)] mod tests` within `book/mod.rs`:

| Test | What it verifies |
|------|-----------------|
| `polyglot_hash_startpos` | `polyglot_hash` of the starting position equals the known reference value `0x463b96181691fc9c` |
| `polyglot_hash_known_positions` | Hash matches reference values for additional known positions (e.g., after 1.e4: `0x823c9b50fd114196`, after 1.e4 d5: `0x0756b94461c50fb0`, after 1.e4 d5 2.e5: `0x662fafb965db29d4`) |
| `entry_parsing` | Constructs synthetic 16-byte big-endian binary data, parses it into a `PolyglotEntry`, and verifies all fields are correctly decoded |
| `binary_search_lookup` | Constructs a `PolyglotBook` with known entries, probes with a matching key and verifies all entries are returned; probes with a non-matching key and verifies empty result |
| `castling_move_translation` | Creates a position where castling is legal, encodes a Polyglot castling move (e.g., king e1→h1), and verifies `polyglot_move_to_engine_move` returns the engine's `Move::new(E1, G1, KING_CASTLE)` |
| `quiet_move_translation` | Verifies a normal quiet move (e.g., e2→e4) is correctly translated |
| `promotion_move_translation` | Verifies a promotion move is correctly translated with the right promotion flag |

## 4. API Contracts

```rust
use chess_engine::book::{PolyglotBook, polyglot_hash, polyglot_move_to_engine_move};

// Load a book
let book = PolyglotBook::from_file("book.bin")?;

// Compute the Polyglot hash for the current position
let key = polyglot_hash(&position);

// Look up book entries
let entries = book.probe(key);

// Convert the best entry's move to engine's Move type
if let Some(entry) = entries.first() {
    if let Some(engine_move) = polyglot_move_to_engine_move(&mut position, entry.raw_move) {
        // Use engine_move
    }
}
```

N/A for REST/HTTP.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| `polyglot_hash_startpos` | Polyglot key of starting position equals `0x463b96181691fc9c` |
| `polyglot_hash_known_positions` | Polyglot keys match published reference values for positions after 1.e4, 1.e4 d5, and 1.e4 d5 2.e5 |
| `entry_parsing` | Synthetic 16-byte big-endian binary data is correctly decoded into `PolyglotEntry` fields |
| `binary_search_lookup` | `probe` returns correct matching entries and empty slice for non-matching key |
| `castling_move_translation` | Polyglot king-to-rook castling encoding is correctly translated to engine's king-to-destination encoding |
| `quiet_move_translation` | Normal move is correctly translated from Polyglot encoding to engine `Move` |
| `promotion_move_translation` | Promotion move encoding is correctly translated with the right `MoveFlag` |

## 6. Implementation Order

1. **`crates/engine/Cargo.toml`** — add `[lib]` section
2. **`crates/engine/src/lib.rs`** — create with `pub mod book;`
3. **`crates/engine/src/book/mod.rs`** — implement in this order within the file:
   a. Polyglot Zobrist key constants (the 781 literal `u64` values)
   b. `polyglot_hash` function
   c. `PolyglotEntry` struct and parsing from bytes
   d. `PolyglotBook` struct with `from_file` and `probe`
   e. `polyglot_move_to_engine_move` function
   f. Unit tests

## 7. Verification Commands

```bash
cargo test -p engine
cargo clippy -p engine -- -D warnings
cargo fmt --check -p engine
```

## 8. Design Deviations

**Deviation: En passant hashing condition**

- The task spec says to compute the Polyglot hash from a `Position`, using the 781 specified random values.
- The Polyglot specification requires that the en passant file key is XORed into the hash **only if a pawn of the opponent is on an adjacent file on the 5th/4th rank** (i.e., only when the en passant capture is actually possible). The engine's `Position` stores the en passant square unconditionally whenever a double pawn push occurs, regardless of whether an opposing pawn can capture.
- The `polyglot_hash` function must therefore inspect the board to determine if the en passant capture is actually available before including the en passant key. This requires checking for enemy pawns adjacent to the pushed pawn, which goes beyond a simple mapping of `Position` fields to Polyglot keys. This is necessary for correct hash computation and matching against book entries.