Now I have all the context. The crate names are `chess-types` and `chess-board`, which match what the specs reference. Let me now produce the definitive specification.

# Phase P01 — Non-Sliding Pieces

## Goal

Implement pseudo-legal move generation for pawns, knights, and kings within a new `movegen` crate, producing correct `Move` values for all non-sliding piece movement patterns — including pawn single/double pushes, pawn captures, en passant, all four promotion types, and pre-computed attack table lookups for knights and kings.

## Design Decisions

1. **New `movegen` crate**: Move generation code lives in `crates/movegen/` per the project structure defined in the master plan. This crate depends on `chess-types` and `chess-board`.

2. **Pseudo-legal generation, not legal**: This phase generates pseudo-legal moves — moves that are valid piece movements but may leave the king in check. Legal filtering is assigned to Phase P03 (T02) of this milestone. King move generation excludes squares attacked by the opponent (using `Position::is_square_attacked`), but does not account for discovered checks on the king from other pieces.

3. **Pre-computed attack tables for knights and kings**: Attack patterns for knights and kings are fixed per square regardless of board occupancy. These are stored as `[Bitboard; 64]` arrays computed at initialization time (compile-time `const` or `lazy_static`/`OnceLock`), not per-call.

4. **Pawn generation separated by color**: White and black pawns move in opposite directions. The implementation uses separate logic paths (or a direction parameter) for each color to handle push direction, starting rank, promotion rank, and en passant rank correctly.

5. **Move output via `Vec<Move>`**: Generated moves are collected into a `Vec<Move>` (or appended to a caller-provided `&mut Vec<Move>`). The exact collection API is a task-level decision.

## Tasks

### T01 — Pawn Move Generation

Implement pawn pseudo-legal move generation for both colors. Deliverables:

- Single pawn push (one square forward, blocked by any piece), encoded with `MoveFlag::QUIET`
- Double pawn push from starting rank (blocked by pieces on either intermediate or target square), encoded with `MoveFlag::DOUBLE_PAWN_PUSH`
- Pawn captures (diagonal, one square, only onto opponent pieces), encoded with `MoveFlag::CAPTURE`
- En passant captures when `Position::en_passant()` returns a target square, encoded with `MoveFlag::EN_PASSANT`
- Non-capture promotions on the 8th/1st rank generating four separate `Move` entries per promotion square, using `MoveFlag::KNIGHT_PROMOTION`, `MoveFlag::BISHOP_PROMOTION`, `MoveFlag::ROOK_PROMOTION`, and `MoveFlag::QUEEN_PROMOTION`
- Capture-promotions on the 8th/1st rank generating four separate `Move` entries per promotion square, using `MoveFlag::KNIGHT_PROMOTION_CAPTURE`, `MoveFlag::BISHOP_PROMOTION_CAPTURE`, `MoveFlag::ROOK_PROMOTION_CAPTURE`, and `MoveFlag::QUEEN_PROMOTION_CAPTURE`
- Unit tests verifying each pawn move type for both white and black, including edge cases (blocked pushes, en passant availability, promotion with and without capture)

### T02 — Knight Move Generation

Implement knight pseudo-legal move generation using pre-computed attack tables. Deliverables:

- A `[Bitboard; 64]` attack table where each entry contains the bitboard of squares a knight on that square can reach
- Attack table initialization (compile-time `const` or runtime-once initialization)
- Knight move generation function that iterates over friendly knights, looks up the attack table, removes friendly-occupied squares, and produces `Move` values — non-captures encoded with `MoveFlag::QUIET` and captures (landing on opponent-occupied squares) encoded with `MoveFlag::CAPTURE`
- Unit tests verifying knight moves from corner, edge, and center squares, and that friendly pieces are excluded from targets

### T03 — King Move Generation

Implement king pseudo-legal move generation using pre-computed attack tables, excluding squares attacked by the opponent. Deliverables:

- A `[Bitboard; 64]` attack table where each entry contains the bitboard of squares a king on that square can reach
- Attack table initialization (compile-time `const` or runtime-once initialization)
- King move generation function that looks up the attack table for the king's square, removes friendly-occupied squares, removes squares attacked by the opponent (using `Position::is_square_attacked`), and produces `Move` values — non-captures encoded with `MoveFlag::QUIET` and captures (landing on opponent-occupied squares) encoded with `MoveFlag::CAPTURE`
- Unit tests verifying king moves exclude friendly-occupied squares and opponent-attacked squares

## Exit Criteria

1. Pawn move generation correctly produces all pseudo-legal pawn moves — single push, double push, captures, en passant, and all four promotion types (both capture and non-capture promotions) — for both white and black.
2. Knight attack tables are pre-computed and knight move generation produces correct pseudo-legal moves from any square.
3. King attack tables are pre-computed and king move generation produces correct pseudo-legal moves, excluding squares attacked by the opponent.
4. All unit tests pass via `cargo test` in the `movegen` crate.

## Dependencies

- **Milestone M01 complete** — requires the `chess-types` crate (`Bitboard`, `Square`, `Piece`, `Color`, `Move`, `MoveFlag`, `File`, `Rank`, `PieceKind`) and the `chess-board` crate (`Position`, `Position::piece_bitboard`, `Position::occupied`, `Position::occupied_by`, `Position::side_to_move`, `Position::en_passant`, `Position::is_square_attacked`).
- No dependency on other M02 phases — this is the first phase in the milestone.

## Artifacts

- `crates/movegen/` — new crate directory with `Cargo.toml` and `src/lib.rs`
- Pawn move generation module within the `movegen` crate
- Knight attack table and move generation module within the `movegen` crate
- King attack table and move generation module within the `movegen` crate
- Unit tests colocated in source files via `#[cfg(test)] mod tests`
- Workspace `Cargo.toml` updated to include the `movegen` crate as a member