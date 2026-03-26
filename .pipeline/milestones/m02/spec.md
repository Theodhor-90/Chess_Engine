## Milestone M02 — Move Generation

### Goal

Complete legal move generation for all piece types, including special moves. Validated by perft to match known reference counts.

### Phases

#### Phase P01 — Non-Sliding Pieces
- **T01**: Pawn move generation — single push, double push, captures, en passant, promotions (4 piece types). Separate logic for white/black direction.
- **T02**: Knight move generation — pre-computed attack tables indexed by square.
- **T03**: King move generation — pre-computed attack tables, excluding squares attacked by opponent.

#### Phase P02 — Sliding Pieces & Magic Bitboards
- **T01**: Magic bitboard infrastructure — blocker mask generation, magic number discovery (or use pre-computed magics), attack table initialization.
- **T02**: Bishop and rook move generation using magic bitboard lookup.
- **T03**: Queen move generation (bishop | rook attacks). Combined sliding piece interface.

#### Phase P03 — Castling, Legality & Perft
- **T01**: Castling move generation — check king/rook haven't moved, no pieces between, no squares attacked, generate O-O and O-O-O.
- **T02**: Legal move filter — generate pseudo-legal moves, filter those that leave king in check. Efficient pin detection.
- **T03**: `perft` function and test suite — validate against standard positions (startpos, Kiwipete, etc.) to depth 5+. This is the correctness gate for all of movegen.

### Exit Criteria
- Legal move generation works correctly for all piece types including pawns (promotions, en passant), knights, bishops, rooks, queens, and kings (including castling)
- Magic bitboards are initialized and produce correct sliding piece attacks
- `perft` results match known reference counts for standard test positions (startpos, Kiwipete, etc.) to depth 5+
- No illegal moves are generated; no legal moves are missed
- All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`