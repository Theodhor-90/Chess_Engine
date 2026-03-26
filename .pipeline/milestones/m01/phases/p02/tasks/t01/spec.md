# T01 — Position Struct

## Objective

Define the `Position` struct and its construction logic within a new `crates/board/` crate, providing the foundational data structure that holds the complete state of a chess game.

## Deliverables

- **New crate**: `crates/board/` with `Cargo.toml` depending on `crates/types/`
- **Updated workspace**: Add `crates/board` to the workspace members in the root `Cargo.toml`
- **`Position` struct** with the following fields:
  - 12 piece bitboards — one `Bitboard` per piece-color combination (white pawns, white knights, white bishops, white rooks, white queens, white king, black pawns, black knights, black bishops, black rooks, black queens, black king), indexed by `Piece`
  - Occupancy bitboards — one per `Color` (white pieces, black pieces) and one total (all pieces)
  - Side to move (`Color`)
  - Castling rights — compact representation with four boolean flags (white kingside, white queenside, black kingside, black queenside)
  - En passant square (`Option<Square>`)
  - Halfmove clock (`u8`)
  - Fullmove counter (`u16`)
- **`Position::startpos()`** — returns the standard chess starting position with all pieces on their initial squares, white to move, all castling rights set, no en passant, halfmove clock 0, fullmove counter 1
- **Accessor methods** for querying piece placement:
  - Get the bitboard for a given piece (piece type + color)
  - Get the piece on a given square (returns `Option<Piece>` or similar)
  - Get occupancy bitboard by color or total

## Files to Create or Modify

- `crates/board/Cargo.toml` — new crate manifest with dependency on `crates/types/`
- `crates/board/src/lib.rs` — crate root and public API exports
- `crates/board/src/position.rs` — `Position` struct, field definitions, accessors, `startpos()`
- `Cargo.toml` (workspace root) — add `crates/board` to workspace members

## Constraints

- Depends on `Bitboard`, `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank` types from the `types` crate (Phase P01, already complete)
- No `unwrap()` in library code — use `Result` or `Option` propagation
- Follow project naming conventions: `snake_case` functions/variables, `PascalCase` types, `SCREAMING_SNAKE` constants
- `///` doc comments on all public items
- No external dependencies beyond `crates/types/` and standard library for core logic; `thiserror` allowed for error types

## Dependencies on Prior Tasks

- None within this phase (this is the first task)
- Depends on Phase P01 (Primitive Types) being complete

## Verification Criteria

1. `crates/board/` crate compiles successfully as part of the workspace
2. `Position::startpos()` returns a position with correct piece placement for all 32 pieces on their standard initial squares
3. `Position::startpos()` has white to move, all four castling rights set, no en passant square, halfmove clock 0, fullmove counter 1
4. Accessor methods correctly return piece bitboards and identify pieces on specific squares
5. Occupancy bitboards are consistent with the union of individual piece bitboards
6. All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`