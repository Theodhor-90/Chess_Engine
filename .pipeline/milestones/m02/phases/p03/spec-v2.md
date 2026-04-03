Good — I can see that `make_move` and `unmake_move` take `&mut self`, while the existing generators take `&Position`. Now I have all the context to address the three feedback issues. Let me produce the revised specification.

# Phase P03 — Castling, Legality & Perft

## Goal

Handle castling move generation for both sides (O-O and O-O-O), enforce full legality by filtering pseudo-legal moves that leave the king in check, and validate the entire move generator end-to-end with a `perft` function and test suite matching known reference counts for standard positions to depth 5+ — serving as the definitive correctness gate for all of move generation.

## Design Decisions

1. **Castling as a separate generator**: Castling move generation is implemented as its own function (`generate_castling_moves`) rather than extending `generate_king_moves`, because castling has unique preconditions (castling rights, path clearance, attack checks) distinct from normal king movement. This keeps the king generator focused and avoids mixing pre-computed table lookups with runtime castling validation.

2. **Legality via make/unmake post-filter**: The legal move filter uses the generate-then-filter approach — generate all pseudo-legal moves (from P01 and P02 generators plus castling), then for each move call `make_move`, check if the own king is attacked via `is_square_attacked`, and `unmake_move` if illegal. This is simple, correct, and leverages the existing `make_move`/`unmake_move` and `is_square_attacked` infrastructure from M01. Pin detection is an optional optimization that must not affect correctness.

3. **Top-level `generate_legal_moves` entry point**: A single public function takes `&mut Position` (mutable because the legality filter calls `make_move`/`unmake_move` internally) and returns a complete list of legal moves. The position is left in its original state after the call — the mutation is purely internal. This becomes the canonical interface the rest of the engine uses for move generation.

4. **Perft as both library function and test suite**: The `perft` function is implemented as a public function in the `movegen` crate (usable programmatically) with integration tests in `tests/` validating against reference positions. A standalone perft runner binary in `tools/` provides CLI access per the project structure.

## Tasks

### T01 — Castling Move Generation

Generate O-O (kingside) and O-O-O (queenside) castling moves for both colors. For each potential castling move, verify: the castling right corresponding to that side and direction (white kingside, white queenside, black kingside, or black queenside) is present in `Position::castling_rights()`, no pieces occupy squares between the king and rook, and no square the king passes through (including origin and destination) is attacked by the opponent via `Position::is_square_attacked()`. Emit moves with `MoveFlag::KING_CASTLE` and `MoveFlag::QUEEN_CASTLE`. Include unit tests covering: castling available and generated, castling blocked by pieces, castling blocked by attacked squares, castling rights revoked, and both sides (white and black).

### T02 — Legal Move Filter

Implement a legality filter that takes the complete set of pseudo-legal moves (from all P01, P02, and T01 generators) and removes any move that leaves the own king in check. For each pseudo-legal move: call `make_move`, check if the own king's square is attacked by the opponent, and call `unmake_move`. Discard illegal moves, retain legal ones. Expose a top-level `generate_legal_moves(pos: &mut Position) -> Vec<Move>` function that calls all generators and applies the filter. The function takes `&mut Position` because the legality check internally calls `make_move`/`unmake_move`, but the position is restored to its original state before returning. Include unit tests covering: discovered check removal, pinned piece restriction, en passant legality (the rare case where en passant exposes the king to a rook/queen attack along the rank), and positions where all moves are legal versus positions where some are filtered.

### T03 — Perft Function and Test Suite

Implement a `perft(pos: &mut Position, depth: u32) -> u64` function that recursively counts leaf nodes by generating legal moves, making each move, recursing at `depth - 1`, and unmaking. Create integration tests that validate perft results against known reference counts for six standard positions: starting position, Kiwipete, CPW Position 3, CPW Position 4, CPW Position 5, and CPW Position 6, all to depth 5+. Exact match is required — any discrepancy indicates a bug. Additionally, create a standalone perft runner binary in `tools/` per the project structure.

## Exit Criteria

1. Castling move generation produces correct O-O and O-O-O moves for both white and black, respecting castling rights, path clearance, and attacked-square constraints.
2. The legal move filter correctly removes all pseudo-legal moves that leave the own king in check, producing no illegal moves and omitting no legal moves.
3. `generate_legal_moves` produces the complete set of legal moves for any position, combining all piece type generators, castling, and the legality filter.
4. `perft` results match known reference counts exactly for all six standard test positions (starting position, Kiwipete, CPW Position 3, CPW Position 4, CPW Position 5, CPW Position 6) to depth 5+.
5. A standalone perft runner binary exists in `tools/`.

## Dependencies

- **Phase P01 (Non-Sliding Pieces)** — pawn, knight, and king pseudo-legal move generators.
- **Phase P02 (Sliding Pieces & Magic Bitboards)** — bishop, rook, and queen move generators via magic bitboard lookup; `generate_sliding_moves` aggregator.
- **Milestone M01 complete** — `Position` struct with castling rights tracking, `make_move`/`unmake_move`, `is_square_attacked`, Zobrist hashing, and all core types (`Bitboard`, `Square`, `Piece`, `Color`, `Move`, `MoveFlag`, `CastlingRights`).

## Artifacts

- `crates/movegen/src/castling.rs` — castling move generation module
- `crates/movegen/src/lib.rs` — updated with castling module, legality filter, `generate_legal_moves`, and `perft` public functions
- `tests/perft.rs` — integration tests validating perft against six standard reference positions
- `tools/perft.rs` (or `tools/perft/main.rs`) — standalone perft runner binary