Now I have complete context. Let me synthesize the definitive specification.

# Phase P03 — Castling, Legality & Perft

## Goal

Handle castling move generation for both sides (O-O and O-O-O), enforce full legality by filtering pseudo-legal moves that leave the king in check, and validate the entire move generator end-to-end with a `perft` function and test suite matching known reference counts for standard positions to depth 5+ — serving as the definitive correctness gate for all of move generation.

## Design Decisions

1. **Castling as a separate generator**: Castling move generation is implemented as its own function (`generate_castling_moves(pos: &Position, moves: &mut Vec<Move>)`) rather than extending `generate_king_moves`, because castling has unique preconditions (castling rights, path clearance, attack checks) distinct from normal king movement. This keeps the king generator focused and avoids mixing pre-computed table lookups with runtime castling validation. The function follows the same signature pattern as all existing piece generators.

2. **Legality via make/unmake post-filter**: The legal move filter uses the generate-then-filter approach — generate all pseudo-legal moves (from P01 and P02 generators plus castling), then for each move call `make_move`, check if the own king is attacked via `is_square_attacked`, and `unmake_move` if illegal. This is simple, correct, and leverages the existing `make_move`/`unmake_move` and `is_square_attacked` infrastructure from M01. Pin detection is an optional optimization that must not affect correctness.

3. **Top-level `generate_legal_moves` entry point**: A single public function `generate_legal_moves(pos: &mut Position) -> Vec<Move>` takes `&mut Position` (mutable because the legality filter calls `make_move`/`unmake_move` internally) and returns a complete list of legal moves. The position is left in its original state after the call — the mutation is purely internal. This becomes the canonical interface the rest of the engine uses for move generation.

4. **Perft as both library function and test suite**: The `perft` function is implemented as a public function in the `movegen` crate (usable programmatically) with integration tests in `tests/` validating against reference positions. A standalone perft runner binary in `tools/` provides CLI access per the project structure.

## Tasks

### T01 — Castling Move Generation

Implement `generate_castling_moves(pos: &Position, moves: &mut Vec<Move>)` to generate O-O (kingside) and O-O-O (queenside) castling moves for the side to move. For each potential castling move, verify three conditions:

1. The specific castling right is present — for white kingside: `pos.castling_rights().contains(CastlingRights::WHITE_KINGSIDE)`, for white queenside: `WHITE_QUEENSIDE`, for black kingside: `BLACK_KINGSIDE`, for black queenside: `BLACK_QUEENSIDE`.
2. No pieces occupy the squares between the king and rook (check occupancy bitboard against the intermediate squares).
3. No square the king passes through (including origin and destination) is attacked by the opponent, verified via `pos.is_square_attacked(square, opponent_color)`.

Emit moves using `Move::new(king_from, king_to, MoveFlag::KING_CASTLE)` for kingside and `Move::new(king_from, king_to, MoveFlag::QUEEN_CASTLE)` for queenside.

Include unit tests covering: castling available and correctly generated, castling blocked by intervening pieces, castling blocked by attacked squares (including the king's origin, transit, and destination squares), castling rights revoked (king or rook has moved), and both colors (white and black).

### T02 — Legal Move Filter

Implement a legality filter that takes the complete set of pseudo-legal moves (from all P01, P02, and T01 generators) and removes any move that leaves the own king in check. The algorithm for each pseudo-legal move:

1. Call `pos.make_move(mv)` to get an `UndoInfo`.
2. Find the own king's square and check if it is attacked by the opponent via `pos.is_square_attacked(king_sq, opponent_color)`.
3. Call `pos.unmake_move(mv, undo)` to restore the position.
4. Retain the move only if the king was not left in check.

Expose a top-level `generate_legal_moves(pos: &mut Position) -> Vec<Move>` function that:
- Collects all pseudo-legal moves by calling `generate_pawn_moves`, `generate_knight_moves`, `generate_king_moves`, `generate_sliding_moves`, and `generate_castling_moves` with a shared `Vec<Move>`.
- Applies the legality filter to remove illegal moves.
- Returns the filtered list. The position is restored to its original state before returning.

Include unit tests covering: discovered check removal (moving a pinned piece exposes the king), pinned piece restriction (pinned piece can only move along the pin ray), en passant legality (the rare case where capturing en passant removes two pawns from a rank and exposes the king to a rook/queen attack along that rank), and positions where all pseudo-legal moves are legal versus positions where some are filtered out.

### T03 — Perft Function and Test Suite

Implement `perft(pos: &mut Position, depth: u32) -> u64` that recursively counts leaf nodes: generate legal moves via `generate_legal_moves`, for each move call `make_move`, recurse at `depth - 1`, then call `unmake_move`. At depth 0, return 1.

Create integration tests in `tests/perft.rs` that validate perft results against known reference counts for six standard positions, all to depth 5+:

| Position | FEN | Depth | Expected Nodes |
|----------|-----|-------|----------------|
| Starting position | `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1` | 5 | 4,865,609 |
| Kiwipete | `r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -` | 5 | 193,690,690 |
| CPW Position 3 | `8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - -` | 5 | 674,624 |
| CPW Position 4 | `r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq -` | 5 | 15,833,292 |
| CPW Position 5 | `rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ -` | 5 | 89,941,194 |
| CPW Position 6 | `r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - -` | 5 | 164,075,551 |

Exact match is required — any discrepancy indicates a bug in move generation.

Additionally, create a standalone perft runner binary in `tools/` (e.g., `tools/perft/main.rs` or `tools/perft.rs`) that accepts a FEN string and depth as command-line arguments and prints the perft result, with optional `divide` mode that prints per-move subtree counts for debugging.

## Exit Criteria

1. Castling move generation produces correct O-O and O-O-O moves for both white and black, respecting castling rights, path clearance, and attacked-square constraints.
2. The legal move filter correctly removes all pseudo-legal moves that leave the own king in check, producing no illegal moves and omitting no legal moves.
3. `generate_legal_moves` produces the complete set of legal moves for any position, combining all piece type generators, castling, and the legality filter.
4. `perft` results match known reference counts exactly for all six standard test positions (starting position, Kiwipete, CPW Position 3, CPW Position 4, CPW Position 5, CPW Position 6) to depth 5+.
5. A standalone perft runner binary exists in `tools/`.

## Dependencies

- **Phase P01 (Non-Sliding Pieces)** — `generate_pawn_moves`, `generate_knight_moves`, and `generate_king_moves` pseudo-legal move generators.
- **Phase P02 (Sliding Pieces & Magic Bitboards)** — `generate_bishop_moves`, `generate_rook_moves`, `generate_queen_moves`, and `generate_sliding_moves` via magic bitboard lookup.
- **Milestone M01 complete** — `Position` struct with `castling_rights()`, `make_move(&mut self, Move) -> UndoInfo`, `unmake_move(&mut self, Move, UndoInfo)`, `is_square_attacked(&self, Square, Color) -> bool`, Zobrist hashing, and all core types (`Bitboard`, `Square`, `Piece`, `Color`, `Move`, `MoveFlag`, `CastlingRights`, `UndoInfo`).

## Artifacts

- `crates/movegen/src/castling.rs` — castling move generation module
- `crates/movegen/src/lib.rs` — updated with `castling` module export, `generate_legal_moves`, and `perft` public functions
- `tests/perft.rs` — integration tests validating perft against six standard reference positions
- `tools/perft.rs` (or `tools/perft/main.rs`) — standalone perft runner binary