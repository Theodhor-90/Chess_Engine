# Phase P01 — Polyglot Opening Book

## Goal

Implement a complete Polyglot `.bin` opening book integration so the engine can look up and play known book lines before falling back to search, by parsing the binary book format, hashing positions with Polyglot-specific Zobrist keys, selecting moves via configurable strategies (`bestmove` and `weighted`), and wiring book consultation into the UCI engine loop.

## Design Decisions

1. **Separate Polyglot Zobrist keys**: Polyglot defines its own set of random Zobrist values distinct from the engine's internal Zobrist scheme (in `crates/board`). A dedicated Polyglot hashing function will compute position keys using the Polyglot-specified random values. This avoids contaminating the engine's internal hashing.

2. **Binary search over sorted entries**: The Polyglot `.bin` format stores 16-byte records sorted by Zobrist key. Lookup uses binary search to find the first matching key, then scans forward to collect all entries for that position. This requires no in-memory index beyond the file contents.

3. **Move encoding translation**: Polyglot encodes moves differently from the engine's internal `Move` (packed `u16`). A translation layer converts Polyglot move encoding (from-square, to-square, promotion piece) to the engine's `Move` representation, handling castling conventions (Polyglot uses king-to-rook-square encoding that may differ from the engine's convention).

4. **Book code location**: The Polyglot book functionality will reside in the `crates/engine` crate (or a dedicated module within it), since it is a top-level feature that bridges the book file, position hashing, and UCI engine loop. It depends on `types`, `board`, and `uci`.

5. **File I/O strategy**: The book file is read into memory on initialization (at UCI option set time). Polyglot books are typically small (a few MB), making full in-memory loading practical and keeping lookup fast.

## Tasks

### T01 — Polyglot `.bin` Book Format Parser

Implement the Polyglot binary book format reader and Polyglot-specific Zobrist key hashing.

**Deliverables:**
- Polyglot Zobrist key constants (the 781 random values defined by the specification) and a function to compute a Polyglot hash from a `Position`
- Polyglot book entry struct representing one 16-byte record (key, move, weight, learn)
- Binary `.bin` file reader that loads entries into a sorted in-memory vector
- Binary search lookup that, given a Polyglot key, returns all matching book entries
- Translation function that converts a Polyglot move encoding to the engine's internal `Move` type
- Unit tests validating Polyglot key computation against known reference hashes (e.g., the starting position Polyglot key) and correct entry parsing

### T02 — Book Move Selection

Implement configurable move selection strategies over the book entries returned by the parser.

**Deliverables:**
- `bestmove` selection mode: given a list of book entries for a position, return the move with the highest weight
- `weighted` selection mode: given a list of book entries for a position, select a move randomly with probability proportional to each entry's weight
- A selection interface that accepts the mode as a parameter and delegates to the `bestmove` or `weighted` strategy based on the provided mode parameter
- Unit tests verifying that `bestmove` returns the highest-weight move and that `weighted` produces a distribution consistent with entry weights

### T03 — UCI Integration

Wire book consultation into the engine's UCI loop and expose configuration via UCI options.

**Deliverables:**
- UCI `option` for book file path (type `string`, default empty — no book loaded)
- UCI `option` for book selection mode (type `combo`, values `bestmove` and `weighted`, default `bestmove`)
- Engine loop modification: when a `go` command is received, consult the book first; if a book move is found for the current position, emit `bestmove` immediately without searching
- Handling of edge cases: no book loaded, position not in book (fall through to search), book file not found at configured path
- Integration test confirming that a known book position returns a book move and that a non-book position falls through to search

## Exit Criteria

1. Polyglot book parser correctly reads `.bin` format and looks up moves by position hash using Polyglot Zobrist keys.
2. Book move selection works in both `bestmove` (highest weight) and `weighted` (random by frequency) modes.
3. UCI option controls book path and selection mode; the engine consults the book before initiating search.
4. A known starting-position book entry is correctly looked up and returned as `bestmove` without triggering search.
5. When no book is loaded or the position is not in the book, the engine falls through to normal search.

## Dependencies

- **M08** (Testing, Tuning & Strength Validation) must be complete — tuned evaluation and search are prerequisite.
- **M04** (UCI Protocol) must be complete — UCI option support and the engine main loop are required for integration.
- **M05** (Transposition Table & Hash) must be complete — Zobrist hashing infrastructure is required for position matching.
- External: Polyglot `.bin` opening book file(s) for testing.
- No dependency on Phase P02 (Syzygy Endgame Tablebases); this phase is independent.