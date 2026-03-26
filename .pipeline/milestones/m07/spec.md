## Milestone M07 — Advanced Evaluation

### Goal

Transform the evaluation function from basic material + PST to positional understanding. Target: ~2000 Elo.

### Phases

#### Phase P01 — Pawn Structure
- **T01**: Pawn hash table — cache pawn structure evaluation keyed by pawn Zobrist hash. Pawn structure changes infrequently, so cache hit rate is very high.
- **T02**: Pawn structure terms — doubled pawns (penalty), isolated pawns (penalty), backward pawns (penalty), passed pawns (bonus scaled by advancement), connected pawns (bonus).
- **T03**: Passed pawn evaluation — bonus for advancement rank, bonus for king proximity (own king close, opponent king far), penalty if blocked.

#### Phase P02 — King Safety & Piece Activity
- **T01**: King safety — pawn shield evaluation (bonus for pawns in front of king), open file penalty near king, virtual mobility of attacking pieces toward king zone.
- **T02**: Mobility evaluation — count legal/pseudo-legal moves per piece (excluding squares attacked by opponent pawns). Scale bonus by piece type.
- **T03**: Bishop pair bonus, rook on open/semi-open file bonus, rook on seventh rank bonus. Knight outpost detection (knight on opponent's side, supported by pawn, can't be attacked by opponent pawn).

#### Phase P03 — Endgame Knowledge
- **T01**: Specialized endgame evaluation — KBN vs K, KR vs K, KQ vs K mating patterns. Detect drawn endgames (insufficient material).
- **T02**: King centralization in endgame — bonus for king advancing toward center as material decreases. Encourage king activity in pawn endgames.
- **T03**: Tapered evaluation tuning — adjust middlegame/endgame weights and phase calculation. Ensure smooth interpolation across all evaluation terms.

### Exit Criteria
- Pawn hash table caches pawn structure evaluation with high hit rate
- Pawn structure evaluation correctly identifies and scores doubled, isolated, backward, passed, and connected pawns
- King safety evaluation penalizes weak king shelter and rewards pawn shields
- Mobility evaluation rewards piece activity and penalizes restricted pieces
- Piece-specific bonuses (bishop pair, rook on open file, knight outposts) are correctly detected and scored
- Specialized endgame evaluation handles common endgame types (KBN vs K, KR vs K, etc.)
- Tapered evaluation smoothly interpolates all terms between middlegame and endgame
- Engine plays at ~2000 Elo level
- All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`