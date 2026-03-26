# Phase P03 — Castling, Legality & Perft

## Goal

Handle castling move generation, enforce full legality by filtering pseudo-legal moves that leave the king in check, and validate the entire move generator end-to-end with perft — the definitive correctness gate for all of move generation. After this phase, the engine has a verified-correct legal move generator for all chess positions.

## Deliverables

- **Castling move generation** — generate O-O (kingside) and O-O-O (queenside) castling moves for both sides. Must verify: king and rook have not moved (castling rights in `Position`), no pieces between king and rook, and no squares the king passes through (including origin and destination) are attacked by the opponent.
- **Legal move filter** — generate pseudo-legal moves (from P01 and P02), then filter out any move that leaves the own king in check. Efficient pin detection to avoid unnecessary `is_square_attacked` calls where possible.
- **`perft` function and test suite** — a recursive move counting function that validates the move generator by matching node counts against known reference values for standard positions: starting position, Kiwipete, CPW Position 3, CPW Position 4, CPW Position 5, and CPW Position 6, all to depth 5+. This is the correctness gate — perft must match exactly before the milestone is considered complete.

## Technical Constraints

- Castling generation must integrate with the `Position` struct's castling rights tracking and use `is_square_attacked` to verify no attacked squares in the king's path.
- The legal move filter should work as a post-filter on the complete pseudo-legal move list (combining output from all piece move generators in P01 and P02 plus castling). The approach: generate all pseudo-legal moves, then for each, call `make_move`, check if own king is in check, and `unmake_move` if illegal.
- Pin detection is an optimization — if implemented, it should reduce the number of full make/unmake cycles needed for legality checking, but correctness must not depend on it.
- The `perft` function should be usable both as a test and as a standalone binary/tool (per the project structure's `tools/` directory for the perft runner).
- All six standard perft positions must match reference counts exactly. Any mismatch indicates a bug in move generation.
- Code belongs in the `movegen` crate (move generation and filtering) and `tests/` or `tools/` (perft suite).

## Dependencies

- **Phase P01 (Non-Sliding Pieces)** — pawn, knight, and king move generators.
- **Phase P02 (Sliding Pieces & Magic Bitboards)** — bishop, rook, and queen move generators.
- **Milestone M01 complete** — `Position` struct with castling rights, `make_move`/`unmake_move`, `is_square_attacked`, Zobrist hashing.