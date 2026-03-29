Now I have a thorough understanding of what exists. Let me produce the milestone specification.

# Milestone M07 — Advanced Evaluation

## Goal

Transform the evaluation function from basic material + piece-square tables into a positionally aware evaluation that understands pawn structure, king safety, piece activity, and endgame patterns, targeting approximately 2000 Elo strength.

## Scope

This milestone extends the `eval` crate with three categories of evaluation knowledge: pawn structure analysis (with a dedicated pawn hash table for caching), king safety and piece activity terms, and specialized endgame evaluation. All new evaluation terms integrate into the existing tapered evaluation framework (middlegame/endgame interpolation) established in M03. The pawn hash table is a new data structure within the eval crate keyed by a pawn-specific Zobrist hash.

## Phases

### Phase P01 — Pawn Structure

Implement a pawn hash table and pawn structure evaluation terms, including passed pawn bonuses.

- **T01**: Pawn hash table — cache pawn structure evaluation keyed by pawn Zobrist hash. Pawn structure changes infrequently, so cache hit rate is very high.
- **T02**: Pawn structure terms — doubled pawns (penalty), isolated pawns (penalty), backward pawns (penalty), passed pawns (bonus scaled by advancement), connected pawns (bonus).
- **T03**: Passed pawn evaluation — bonus for advancement rank, bonus for king proximity (own king close, opponent king far), penalty if blocked.

### Phase P02 — King Safety & Piece Activity

Add evaluation terms for king shelter quality, piece mobility, and piece-specific positional bonuses.

- **T01**: King safety — pawn shield evaluation (bonus for pawns in front of king), open file penalty near king, virtual mobility of attacking pieces toward king zone.
- **T02**: Mobility evaluation — count legal/pseudo-legal moves per piece (excluding squares attacked by opponent pawns). Scale bonus by piece type.
- **T03**: Bishop pair bonus, rook on open/semi-open file bonus, rook on seventh rank bonus. Knight outpost detection (knight on opponent's side, supported by pawn, can't be attacked by opponent pawn).

### Phase P03 — Endgame Knowledge

Implement specialized endgame evaluation, king centralization, and ensure all new terms integrate cleanly into tapered evaluation.

- **T01**: Specialized endgame evaluation — KBN vs K, KR vs K, KQ vs K mating patterns. Detect drawn endgames (insufficient material).
- **T02**: King centralization in endgame — bonus for king advancing toward center as material decreases. Encourage king activity in pawn endgames.
- **T03**: Tapered evaluation tuning — adjust middlegame/endgame weights and phase calculation. Ensure smooth interpolation across all evaluation terms.

## Exit Criteria

1. Pawn hash table caches pawn structure evaluation with high hit rate
2. Pawn structure evaluation correctly identifies and scores doubled, isolated, backward, passed, and connected pawns
3. King safety evaluation penalizes weak king shelter and rewards pawn shields
4. Mobility evaluation rewards piece activity and penalizes restricted pieces
5. Piece-specific bonuses (bishop pair, rook on open file, knight outposts) are correctly detected and scored
6. Specialized endgame evaluation handles common endgame types (KBN vs K, KR vs K, KQ vs K)
7. Tapered evaluation smoothly interpolates all new terms between middlegame and endgame
8. Engine plays at approximately 2000 Elo level
9. All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`

## Dependencies

- **M06 (Advanced Search Techniques)** — must be complete, providing the search framework (PVS, pruning, move ordering enhancements) that the evaluation feeds into
- **Existing `eval` crate** — M03 established tapered evaluation with material counting, piece-square tables, and game phase calculation; M07 extends this
- **Existing `board` crate** — `Position` struct with piece bitboards, Zobrist hashing, `is_square_attacked`, and make/unmake infrastructure
- **Existing `types` crate** — `Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`, `File`, `Rank` types with iteration and bitwise operations