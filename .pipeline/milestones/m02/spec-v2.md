Good. Now I have all the context I need. Let me produce the revised specification addressing both feedback issues:

1. **Format issue**: Remove the stray LLM preamble on line 1
2. **Ambiguity issue**: Replace vague "etc." in P03/T03 with specific test positions

# Milestone M02 — Move Generation

## Goal

Implement complete legal move generation for all chess piece types — including pawn promotions, en passant, castling, and sliding piece attacks via magic bitboards — and validate correctness by matching perft results against known reference counts for standard positions to depth 5+.

## Scope

- Pawn move generation: single push, double push, captures, en passant, all four promotion piece types, with separate logic for white and black direction
- Knight move generation using pre-computed attack tables indexed by square
- King move generation using pre-computed attack tables, excluding squares attacked by the opponent
- Magic bitboard infrastructure: blocker mask generation, magic number discovery (or use pre-computed magics), attack table initialization
- Bishop and rook move generation using magic bitboard lookup
- Queen move generation as the union of bishop and rook attacks, with a combined sliding piece interface
- Castling move generation: verify king/rook haven't moved, no pieces between, no squares attacked, generate O-O and O-O-O for both sides
- Legal move filter: generate pseudo-legal moves, then filter moves that leave the king in check; efficient pin detection
- `perft` function and test suite validating against standard positions (starting position, Kiwipete, and CPW Positions 3–6) to depth 5+

## Phases

### Phase P01 — Non-Sliding Pieces

Generate moves for pawns, knights, and kings — the pieces whose attack patterns do not depend on board occupancy (except pawns, which need occupancy for pushes/captures).

- **T01**: Pawn move generation — single push, double push, captures, en passant, promotions (4 piece types). Separate logic for white/black direction.
- **T02**: Knight move generation — pre-computed attack tables indexed by square.
- **T03**: King move generation — pre-computed attack tables, excluding squares attacked by opponent.

### Phase P02 — Sliding Pieces & Magic Bitboards

Implement magic bitboard lookup for sliding piece attack generation, enabling O(1) move generation for bishops, rooks, and queens.

- **T01**: Magic bitboard infrastructure — blocker mask generation, magic number discovery (or use pre-computed magics), attack table initialization.
- **T02**: Bishop and rook move generation using magic bitboard lookup.
- **T03**: Queen move generation (bishop | rook attacks). Combined sliding piece interface.

### Phase P03 — Castling, Legality & Perft

Handle castling, enforce full legality by filtering pseudo-legal moves, and validate the entire move generator with perft.

- **T01**: Castling move generation — check king/rook haven't moved, no pieces between, no squares attacked, generate O-O and O-O-O.
- **T02**: Legal move filter — generate pseudo-legal moves, filter those that leave king in check. Efficient pin detection.
- **T03**: `perft` function and test suite — validate against standard positions (starting position, Kiwipete, CPW Position 3, CPW Position 4, CPW Position 5, CPW Position 6) to depth 5+. This is the correctness gate for all of movegen.

## Exit Criteria

1. Legal move generation produces correct moves for all piece types: pawns (single push, double push, captures, en passant, all four promotion types), knights, bishops, rooks, queens, and kings (including castling O-O and O-O-O).
2. Magic bitboards are initialized and produce correct attack sets for bishops and rooks on all squares with all occupancy configurations.
3. The legal move filter correctly removes pseudo-legal moves that leave the king in check, generating no illegal moves and missing no legal moves.
4. `perft` results match known reference counts for standard test positions (starting position, Kiwipete, CPW Position 3, CPW Position 4, CPW Position 5, CPW Position 6) to depth 5+.
5. All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.

## Dependencies

- **Milestone M01 complete** — M02 requires the `types` crate (`Bitboard`, `Square`, `Piece`, `Color`, `Move`, `File`, `Rank`) and the `board` crate (`Position` struct, FEN parsing, Zobrist hashing, `make_move`/`unmake_move`, `is_square_attacked`).