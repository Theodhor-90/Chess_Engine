# Task T03 — Move Encoding

## Objective

Implement the `Move` type as a packed `u16` in the `crates/types/` crate, supporting encode/decode for all chess move types via a 4-bit flags field.

## Deliverables

- `Move` struct wrapping a `u16`
- Bit layout: `from(6) | to(6) | flags(4)` — 6 bits for source square, 6 bits for destination square, 4 bits for flags
- Encode methods: construct a `Move` from source `Square`, destination `Square`, and flags
- Decode methods: extract source `Square`, destination `Square`, and flags from a `Move`
- Flag definitions covering all move types:
  - Quiet move (no special flags)
  - Double pawn push
  - King-side castling
  - Queen-side castling
  - Capture
  - En passant capture
  - Four promotion variants: knight, bishop, rook, queen promotion
  - Four promotion-capture variants: knight, bishop, rook, queen promotion with capture
- A `MoveFlags` enum or constants for all 16 flag combinations
- `Display` implementation producing UCI-style notation (e.g., `e2e4`, `e7e8q`)

## Files to Create or Modify

- Create `crates/types/src/moves.rs` (or `chess_move.rs` to avoid keyword conflicts)
- Modify `crates/types/src/lib.rs` to export the `Move` type and flag definitions

## Key Implementation Details

- The 16-bit packed layout makes `Move` extremely cache-friendly for move lists during search — this is a critical performance consideration
- Flag encoding follows a standard convention (matching Stockfish/CPW conventions):
  - Bits 0-3 encode the flag, bits 4-9 encode the destination square, bits 10-15 encode the source square (or equivalent layout — the exact bit positions should be chosen for efficient extraction)
- `from`/`to` fields must interoperate with `Square` indices (0..63) from T02
- Promotion flags must encode which piece to promote to (Knight, Bishop, Rook, Queen)
- Distinguish between capture promotions and non-capture promotions
- No `unwrap()` in library code
- No external dependencies
- `///` doc comments on all public items

## Dependencies

- **T01 (Bitboard)** — establishes the crate structure
- **T02 (Chess Enums)** — `Move` encode/decode uses `Square` for source and destination, and `PieceKind` for promotion piece

## Verification Criteria

1. Unit tests verify encode/decode round-trips for every flag type (quiet, double push, king castle, queen castle, capture, en passant, 4 promotions, 4 promotion captures)
2. Unit tests verify correct field extraction: source square, destination square, and flags match what was encoded
3. Unit tests cover boundary squares: moves from a1 (index 0) and to h8 (index 63)
4. Unit tests verify that all 16 possible flag values are distinct and correctly identified
5. Unit tests verify `Display` produces correct UCI notation including promotion suffix
6. `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` all pass