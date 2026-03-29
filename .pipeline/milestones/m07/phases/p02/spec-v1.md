# Phase P02 — King Safety & Piece Activity

## Goal

Add evaluation terms for king shelter quality, piece mobility, and piece-specific positional bonuses (bishop pair, rook on open/semi-open files, rook on seventh rank, knight outposts) to the `eval` crate, giving the engine positional understanding of king safety and piece activity. All new terms produce separate middlegame and endgame scores integrated into the existing tapered evaluation framework.

## Design Decisions

1. **King zone definition**: The king zone comprises the 8 squares surrounding the king plus 3 squares in front of those (11 squares total). This captures the area where attacking pieces threaten king safety.

2. **Pawn shield scaling by rank distance**: Pawn shield bonus is awarded for pawns on the three files around the castled king, scaled by how close the pawn is to the king's rank. Pawns further advanced provide less shelter.

3. **Mobility excludes opponent-pawn-controlled squares**: Pseudo-legal moves for each piece are counted, but squares attacked by opponent pawns are excluded. This prevents inflating mobility scores with moves into danger zones.

4. **Queen mobility scaled lightly**: Queen mobility receives a smaller per-move bonus than knights, bishops, or rooks to avoid the engine overvaluing early queen wandering.

5. **Knight outpost criteria**: A knight qualifies as an outpost when it sits on a square in ranks 4–6 on the opponent's half, is supported by a friendly pawn, and cannot be attacked by any opponent pawn. These conditions ensure the knight occupies a durable advanced post.

6. **Dependency on P01 pawn data**: King safety (pawn shield, open files near king), rook open/semi-open file detection, and knight outpost safety from opponent pawns all rely on pawn position information available from the `Position` struct's pawn bitboards.

## Tasks

- **T01 — King Safety Evaluation**: Implement pawn shield evaluation (bonus for pawns on the three files around the castled king, scaled by rank distance from king), open file penalty near king (files adjacent to king with no friendly pawns), and virtual mobility of attacking pieces toward the king zone. Produce separate midgame and endgame scores. Unit tests verifying correct scoring for positions with strong vs weak pawn shields, open files near king, and attacking piece pressure.

- **T02 — Mobility Evaluation**: Count pseudo-legal moves per piece (knights, bishops, rooks, queens), excluding squares attacked by opponent pawns. Apply piece-type-dependent scaling (knights and bishops highest, rooks moderate, queens light). Produce separate midgame and endgame mobility scores. Unit tests verifying mobility counts and score contributions for known positions.

- **T03 — Piece-Specific Positional Bonuses**: Implement bishop pair bonus (awarded when a side has two or more bishops), rook on open file bonus (no pawns on file), rook on semi-open file bonus (no friendly pawns on file), rook on seventh rank bonus, and knight outpost detection (ranks 4–6, opponent's half, pawn-supported, not attackable by opponent pawn). Unit tests verifying correct detection and scoring for each bonus type.

## Exit Criteria

1. King safety evaluation penalizes weak king shelter (missing pawn shield, open files near king) and scores attacking potential toward the king zone, verified by unit tests on known positions
2. Mobility evaluation counts pseudo-legal moves per piece (excluding squares controlled by opponent pawns) and contributes scaled bonuses by piece type, verified by unit tests
3. Bishop pair, rook on open/semi-open file, rook on seventh rank, and knight outpost bonuses are correctly detected and scored, each verified by unit tests
4. All new evaluation terms (king safety, mobility, piece bonuses) contribute separate middlegame and endgame scores that are combined via the existing tapered evaluation phase calculation from M03

## Dependencies

- **M07 Phase P01 (Pawn Structure)** — must be complete; pawn structure analysis provides pawn position data needed for king safety (pawn shield, open files near king), rook open/semi-open file detection, and knight outpost safety from opponent pawns
- **Existing `eval` crate** — M03 established tapered evaluation with `evaluate()`, `material_score()`, midgame/endgame piece-square tables (`mg_table`/`eg_table`), and `compute_phase()` in `crates/eval/src/phase.rs`
- **Existing `board` crate** — `Position` struct with piece bitboards, `is_square_attacked`, Zobrist hashing, and make/unmake infrastructure
- **Existing `types` crate** — `Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`, `File`, `Rank` types with iteration and bitwise operations

## Artifacts

- `crates/eval/src/king_safety.rs` — king safety evaluation (pawn shield, open file penalty, king zone attack scoring)
- `crates/eval/src/mobility.rs` — per-piece mobility counting and scoring
- `crates/eval/src/pieces.rs` — piece-specific positional bonuses (bishop pair, rook file bonuses, rook on seventh, knight outposts)
- `crates/eval/src/lib.rs` — modified to integrate king safety, mobility, and piece bonus terms into the `evaluate()` function