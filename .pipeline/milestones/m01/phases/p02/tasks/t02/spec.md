# T02 — FEN Parser and Serializer

## Objective

Implement `Position::from_fen` and `Position::to_fen` with full validation of all six FEN fields, enabling interoperability with standard chess tooling and providing the primary way to construct arbitrary positions.

## Deliverables

- **`Position::from_fen(fen: &str) -> Result<Position, FenError>`** — parses and validates all six FEN fields:
  1. Piece placement (8 ranks separated by `/`, using standard piece letters `KQRBNPkqrbnp` and digit counts `1-8`)
  2. Active color (`w` or `b`)
  3. Castling availability (`KQkq` subset, or `-` for none)
  4. En passant target square (algebraic notation e.g. `e3`, or `-` for none)
  5. Halfmove clock (non-negative integer)
  6. Fullmove number (positive integer)
- **`Position::to_fen(&self) -> String`** — produces a canonical FEN string from the current position state
- **`FenError` type** — error enum with `thiserror` derive, with variants covering each kind of validation failure (wrong field count, invalid piece placement, invalid active color, invalid castling, invalid en passant, out-of-range clocks)
- **Comprehensive unit tests** verifying:
  - Starting position FEN: `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1`
  - At least 3 middlegame positions: Alekhine's Defense (`r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2`), Italian-like (`r1bqkb1r/pppppppp/2n2n2/8/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3`), Nimzo-Indian (`rnbqk2r/pppp1ppp/4pn2/8/1bPP4/2N5/PP2PPPP/R1BQKBNR w KQkq - 2 4`)
  - At least 2 endgame positions: king+pawn vs king (`8/8/4k3/8/8/4K3/4P3/8 w - - 0 1`), rook endgame (`8/5k2/8/8/8/2K5/8/4R3 w - - 0 1`)
  - Round-trip property: `Position::from_fen(fen).unwrap().to_fen() == fen` for all canonical FEN strings above
  - Rejection of malformed FEN strings with appropriate `FenError` variants

## Files to Create or Modify

- `crates/board/src/fen.rs` — `from_fen`, `to_fen` implementations, and `FenError` type
- `crates/board/src/lib.rs` — add `mod fen;` and re-export public items
- `crates/board/src/position.rs` — may need to add the `from_fen` and `to_fen` methods to `Position` (or implement as associated functions/methods delegating to `fen.rs`)
- `crates/board/Cargo.toml` — add `thiserror` dependency if not already present

## Constraints

- No `unwrap()` in library code — all FEN parsing errors must produce `Result::Err(FenError)`
- `to_fen` must produce canonical FEN (standard piece letter case, ranks separated by `/`, correct digit compression)
- The parser must validate each FEN field and provide descriptive error messages
- Follow project coding conventions

## Dependencies on Prior Tasks

- **T01 (Position Struct)** must be complete — this task needs the `Position` struct, its fields, and accessor methods to exist

## Verification Criteria

1. `Position::from_fen` correctly parses all standard FEN strings listed above into valid `Position` instances
2. `Position::to_fen` produces the exact canonical FEN string for each position
3. Round-trip property holds: `from_fen(fen).unwrap().to_fen() == fen` for all test FEN strings
4. Malformed FEN strings are rejected with appropriate `FenError` variants (wrong field count, invalid piece placement, invalid active color, invalid castling string, invalid en passant, out-of-range clocks)
5. All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`