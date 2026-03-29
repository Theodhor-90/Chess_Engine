## Phase P02 — King Safety & Piece Activity

### Goal

Add evaluation terms for king shelter quality, piece mobility, and piece-specific positional bonuses (bishop pair, rook on open files, knight outposts). This phase gives the engine the ability to evaluate positional features related to how safe the king is and how active the pieces are.

### Deliverables

- **King safety** (T01): Pawn shield evaluation (bonus for pawns on the three files around the castled king, scaled by rank distance from king), open file penalty near king (files adjacent to king with no friendly pawns), and virtual mobility of attacking pieces toward the king zone (the 8 squares surrounding the king plus 3 squares in front of those).
- **Mobility evaluation** (T02): Count pseudo-legal moves per piece, excluding squares attacked by opponent pawns. Scale bonus by piece type: knights and bishops benefit most from mobility, rooks moderately, queens lightly (to avoid overvaluing queen wandering). Contribute separate midgame and endgame mobility scores.
- **Piece-specific positional bonuses** (T03): Bishop pair bonus (awarded when side has two or more bishops), rook on open file bonus (no pawns on file) and semi-open file bonus (no friendly pawns on file), rook on seventh rank bonus. Knight outpost detection — knight on a square in ranks 4–6 on the opponent's half of the board, supported by a friendly pawn, and not attackable by any opponent pawn.

### Technical Decisions & Constraints

- All new terms must produce separate middlegame and endgame scores for the existing tapered evaluation framework.
- Mobility calculation excludes squares controlled by opponent pawns to avoid counting "moves" into danger zones.
- Queen mobility is scaled lightly to avoid the engine overvaluing queen wandering early.
- King zone is defined as the 8 squares surrounding the king plus 3 squares in front of those (11 squares total).
- Knight outpost criteria: square must be in ranks 4–6, on the opponent's half, supported by a friendly pawn, and not attackable by any opponent pawn.
- Unit tests must verify correct detection and scoring of each positional feature.

### Dependencies

- **Phase P01 (Pawn Structure)** must be complete — pawn structure analysis (especially pawn positions on files) is needed for king safety (pawn shield, open files near king) and for determining whether rook files are open/semi-open, and whether knight outpost squares are safe from opponent pawns.
- Depends on the existing `eval` crate and tapered evaluation from M03.
- Depends on the `board` crate (`Position`, `is_square_attacked`, piece bitboards).
- Depends on the `types` crate for bitboard operations and square/file/rank manipulation.