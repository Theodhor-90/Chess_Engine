# Task T01 — Pawn Move Generation

## Objective

Implement pseudo-legal pawn move generation for both white and black within a new `movegen` crate (`crates/movegen/`). This is the first task in M02/P01 and establishes the crate structure that T02 and T03 will build upon.

## Deliverables

1. **New `movegen` crate** at `crates/movegen/` with `Cargo.toml` and `src/lib.rs`
2. **Workspace registration** — update the root `Cargo.toml` to include `crates/movegen` as a workspace member
3. **Pawn move generation module** within the `movegen` crate that generates all pseudo-legal pawn moves for the side to move

## Move Types to Generate

For both white and black pawns (separate logic for each color's direction):

- **Single pawn push** — one square forward, blocked by any piece on the target square. Encoded with `MoveFlag::QUIET`.
- **Double pawn push** — two squares forward from starting rank (rank 2 for white, rank 7 for black), blocked if any piece occupies the intermediate or target square. Encoded with `MoveFlag::DOUBLE_PAWN_PUSH`.
- **Pawn captures** — diagonal one square onto opponent-occupied squares. Encoded with `MoveFlag::CAPTURE`.
- **En passant captures** — diagonal capture onto the en passant target square when `Position::en_passant()` returns `Some(square)`. Encoded with `MoveFlag::EN_PASSANT`.
- **Non-capture promotions** — single push to the 8th rank (white) or 1st rank (black), generating **four** separate `Move` entries per pawn using `MoveFlag::KNIGHT_PROMOTION`, `MoveFlag::BISHOP_PROMOTION`, `MoveFlag::ROOK_PROMOTION`, `MoveFlag::QUEEN_PROMOTION`.
- **Capture-promotions** — diagonal capture onto the promotion rank, generating **four** separate `Move` entries per capture using `MoveFlag::KNIGHT_PROMOTION_CAPTURE`, `MoveFlag::BISHOP_PROMOTION_CAPTURE`, `MoveFlag::ROOK_PROMOTION_CAPTURE`, `MoveFlag::QUEEN_PROMOTION_CAPTURE`.

## Implementation Constraints

- The `movegen` crate depends on `chess-types` and `chess-board` (the existing crate names from M01)
- Use bitboard operations for all move generation (shift entire pawn bitboard by 8/16 for pushes, by 7/9 for captures)
- Moves are collected via `&mut Vec<Move>` or returned as `Vec<Move>` — decide at implementation time
- This is **pseudo-legal** generation: moves may leave the king in check. Legal filtering comes in P03/T02.
- No `unsafe` unless provably necessary. No `unwrap()` in library code.
- Unit tests colocated in source files via `#[cfg(test)] mod tests`

## Key Dependencies (from M01)

- `Bitboard` — shift, AND, OR, NOT, iteration, `is_empty()`
- `Square` — `index()`, `file()`, `rank()`, `from_file_rank()`
- `Move` — `new(from, to, flags)`
- `MoveFlag` — all flag constants listed above
- `Color` — `White`, `Black`, `opposite()`
- `Piece` — `new(color, PieceKind::Pawn)`
- `PieceKind::Pawn`
- `File`, `Rank` — for rank/file checks
- `Position` — `piece_bitboard()`, `occupied()`, `occupied_by()`, `side_to_move()`, `en_passant()`

## Dependencies on Other Tasks

- None — this is the first task in the phase. It must create the `movegen` crate that T02 and T03 will add to.

## Verification Criteria

1. `cargo test -p movegen` (or equivalent crate name) passes all unit tests
2. Unit tests cover each pawn move type for both white and black:
   - Single push (including blocked push)
   - Double push (including blocked by intermediate/target piece)
   - Captures (left and right diagonals)
   - En passant capture (when available and when not)
   - Non-capture promotion (all four piece types)
   - Capture-promotion (all four piece types)
3. Generated moves use the correct `MoveFlag` for each move type
4. No moves are generated for empty squares or opponent pawns
5. `cargo clippy -- -D warnings` passes for the `movegen` crate
6. `cargo fmt --check` passes