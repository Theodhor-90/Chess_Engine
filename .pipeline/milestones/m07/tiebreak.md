# Milestone M07 — Advanced Evaluation

## Goal

Transform the evaluation function from basic material + piece-square tables into a positionally aware evaluation that understands pawn structure, king safety, piece activity, and endgame patterns, targeting at least 1950 Elo as measured by a 500+ game gauntlet against reference engines using CuteChess-CLI at 10+0.1 time control.

## Scope

This milestone extends the `eval` crate with three categories of evaluation knowledge: pawn structure analysis (with a dedicated pawn hash table for caching), king safety and piece activity terms, and specialized endgame evaluation. All new evaluation terms integrate into the existing tapered evaluation framework (middlegame/endgame interpolation via `compute_phase` in `crates/eval/src/phase.rs`) established in M03. The pawn hash table is a new data structure within the eval crate keyed by a pawn-specific Zobrist hash derived from the existing `PIECE_SQUARE_KEYS` in `crates/board/src/zobrist.rs`, computed incrementally by XOR-ing only the pawn-square keys.

## Dependencies

- **M06 (Advanced Search Techniques)** — must be complete, providing the search framework (PVS, pruning, move ordering enhancements) that the evaluation feeds into
- **Existing `eval` crate** — M03 established tapered evaluation with `evaluate()`, `material_score()`, midgame/endgame piece-square tables (`mg_table`/`eg_table`), and `compute_phase()`; M07 extends this
- **Existing `board` crate** — `Position` struct with piece bitboards, Zobrist hashing (`PIECE_SQUARE_KEYS`), `is_square_attacked`, and make/unmake infrastructure
- **Existing `types` crate** — `Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`, `File`, `Rank` types with iteration and bitwise operations

## Phases

### Phase P01 — Pawn Structure

Implement a pawn hash table and pawn structure evaluation terms, including passed pawn bonuses.

- **T01**: Pawn hash table — fixed-size hash table (power-of-two entry count, default 16K entries) that caches pawn structure evaluation keyed by a pawn Zobrist hash. The pawn Zobrist hash is computed by XOR-ing `PIECE_SQUARE_KEYS` entries for all pawns on the board. On probe hit, return the cached midgame/endgame pawn evaluation scores. On probe miss, compute the pawn evaluation, store the result, and return it.
- **T02**: Pawn structure terms — doubled pawns (penalty), isolated pawns (penalty), backward pawns (penalty), passed pawns (bonus scaled by advancement rank), connected pawns (bonus). Each term contributes separate midgame and endgame scores for integration with tapered evaluation.
- **T03**: Passed pawn evaluation — bonus for advancement rank (exponentially increasing toward promotion), bonus for king proximity (own king close to passed pawn, opponent king far from passed pawn), penalty if the passed pawn is blocked by any piece on its advance square.

### Phase P02 — King Safety & Piece Activity

Add evaluation terms for king shelter quality, piece mobility, and piece-specific positional bonuses.

- **T01**: King safety — pawn shield evaluation (bonus for pawns on the three files around the castled king, scaled by rank distance from king), open file penalty near king (files adjacent to king with no friendly pawns), virtual mobility of attacking pieces toward king zone (the 8 squares surrounding the king plus 3 squares in front of those).
- **T02**: Mobility evaluation — count pseudo-legal moves per piece (excluding squares attacked by opponent pawns). Scale bonus by piece type: knights and bishops benefit most from mobility, rooks moderately, queens lightly (to avoid overvaluing queen wandering). Contribute separate midgame and endgame mobility scores.
- **T03**: Bishop pair bonus (awarded when side has two or more bishops), rook on open file bonus (no pawns on file) and semi-open file bonus (no friendly pawns on file), rook on seventh rank bonus. Knight outpost detection — knight on a square in ranks 4-6 on the opponent's half of the board, supported by a friendly pawn, and not attackable by any opponent pawn.

### Phase P03 — Endgame Knowledge

Implement specialized endgame evaluation, king centralization, and ensure all new terms integrate cleanly into tapered evaluation.

- **T01**: Specialized endgame evaluation — detect and evaluate KBN vs K (drive king to correct corner based on bishop color), KR vs K (drive king to edge), KQ vs K (drive king to edge) mating patterns. Detect drawn endgames due to insufficient material (K vs K, KB vs K, KN vs K, KNN vs K). When a specialized endgame is detected, bypass the general evaluation and return the endgame-specific score.
- **T02**: King centralization in endgame — bonus for king advancing toward center squares as material decreases. Use the endgame component of tapered evaluation so the bonus is weighted toward low-material positions. Additional bonus in pure pawn endgames for king proximity to passed pawns.
- **T03**: Tapered evaluation integration — ensure all new evaluation terms from P01 and P02 contribute separate middlegame and endgame scores that are combined via the existing `compute_phase()` interpolation. Verify that the phase calculation in `crates/eval/src/phase.rs` correctly weights the transition. Adjust evaluation term weights for balance (no single term should dominate; king safety should not outweigh material for quiet positions).

## Exit Criteria

1. Pawn hash table is probed before computing pawn evaluation; on a hit, cached midgame/endgame scores are returned; on a miss, the computed result is stored into the table before returning
2. Pawn structure evaluation correctly identifies and scores doubled, isolated, backward, passed, and connected pawns with unit tests verifying detection on known FEN positions
3. King safety evaluation penalizes weak king shelter (missing pawn shield, open files near king) and scores attacking potential toward the king zone
4. Mobility evaluation counts pseudo-legal moves per piece (excluding squares controlled by opponent pawns) and contributes scaled bonuses by piece type
5. Piece-specific bonuses (bishop pair, rook on open/semi-open file, rook on seventh rank, knight outposts) are correctly detected and scored with unit tests for each
6. Specialized endgame evaluation handles KBN vs K, KR vs K, and KQ vs K mating patterns, and detects drawn endgames from insufficient material
7. All new evaluation terms (pawn structure, king safety, mobility, piece bonuses, endgame) contribute separate middlegame and endgame scores that are combined via the existing tapered evaluation phase calculation from M03
8. Engine achieves at least 1950 Elo as measured by a 500+ game gauntlet against reference engines using CuteChess-CLI at 10+0.1 time control