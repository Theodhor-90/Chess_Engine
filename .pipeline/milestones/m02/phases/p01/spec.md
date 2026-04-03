# Phase P01 — Non-Sliding Pieces

## Goal

Generate moves for pawns, knights, and kings — the pieces whose attack patterns do not depend on board occupancy (except pawns, which need occupancy for pushes and captures). This phase produces the move generation functions for all non-sliding piece types, building on the M01 foundation of core types (`Bitboard`, `Square`, `Piece`, `Color`, `Move`) and position representation (`Position`, `make_move`/`unmake_move`, `is_square_attacked`).

## Deliverables

- **Pawn move generation** — single push, double push, captures, en passant, promotions (all 4 promotion piece types: knight, bishop, rook, queen). Separate logic for white and black pawn direction.
- **Knight move generation** — pre-computed attack tables indexed by square (64-entry lookup table of `Bitboard` attack masks).
- **King move generation** — pre-computed attack tables indexed by square, excluding squares attacked by the opponent (requires integration with `is_square_attacked`).

## Technical Constraints

- All move generation should produce pseudo-legal moves represented as the `Move` type (16-bit packed `u16` with from/to/flags encoding from M01).
- Attack tables for knights and kings must be pre-computed at initialization time (compile-time `const` or lazy-static), not computed per call.
- Pawn logic must correctly handle all special cases: double push only from starting rank, en passant capture only when en passant square is set in `Position`, promotions only on the 8th/1st rank, and all four promotion piece types must generate separate `Move` entries.
- Functions should operate on `Bitboard` representations from the `types` crate and read position state from the `board` crate's `Position` struct.
- Code belongs in the `movegen` crate per the project structure.
- No `unsafe` unless provably necessary. No `unwrap()` in library crate code. Use `Result`/`Option` propagation.

## Dependencies

- **Milestone M01 complete** — requires `types` crate (`Bitboard`, `Square`, `Piece`, `Color`, `Move`, `File`, `Rank`) and `board` crate (`Position`, FEN parsing, `make_move`/`unmake_move`, `is_square_attacked`).
- No dependency on other M02 phases — this is the first phase in the milestone.