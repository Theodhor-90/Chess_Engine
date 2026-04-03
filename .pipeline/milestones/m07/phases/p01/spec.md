## Phase P01 — Pawn Structure

### Goal

Implement a pawn hash table and comprehensive pawn structure evaluation terms — including passed pawn bonuses — within the `eval` crate. This phase adds the first layer of positional knowledge beyond material and piece-square tables, giving the engine the ability to recognize and score pawn weaknesses and strengths.

### Deliverables

- **Pawn hash table** (T01): A fixed-size, power-of-two hash table (default 16K entries) that caches pawn structure evaluation results. The hash key is a pawn-specific Zobrist hash computed by XOR-ing `PIECE_SQUARE_KEYS` entries (from `crates/board/src/zobrist.rs`) for all pawns on the board. On probe hit, return cached midgame/endgame pawn evaluation scores. On probe miss, compute the pawn evaluation, store the result, and return it.
- **Pawn structure terms** (T02): Evaluation terms for doubled pawns (penalty), isolated pawns (penalty), backward pawns (penalty), passed pawns (bonus scaled by advancement rank), and connected pawns (bonus). Each term contributes separate midgame and endgame scores for integration with the existing tapered evaluation framework.
- **Passed pawn evaluation** (T03): Advanced passed pawn scoring with bonus for advancement rank (exponentially increasing toward promotion), bonus for king proximity (own king close to passed pawn, opponent king far), and penalty if the passed pawn is blocked by any piece on its advance square.

### Technical Decisions & Constraints

- The pawn Zobrist hash is derived from the existing `PIECE_SQUARE_KEYS` in `crates/board/src/zobrist.rs` — no new random key tables are needed for pawns; XOR only the pawn-square key entries.
- All evaluation terms must produce separate middlegame and endgame scores that integrate into the existing tapered evaluation framework (`compute_phase` in `crates/eval/src/phase.rs`) established in M03.
- The pawn hash table uses power-of-two sizing for fast modulo (bitwise AND masking).
- Unit tests must verify detection of each pawn structure pattern on known FEN positions.

### Dependencies

- No prior M07 phases (this is the first phase).
- Depends on M06 (Advanced Search Techniques) being complete.
- Depends on the existing `eval` crate from M03 (tapered evaluation with `evaluate()`, `material_score()`, `mg_table`/`eg_table`, and `compute_phase()`).
- Depends on the `board` crate (`Position` struct, `PIECE_SQUARE_KEYS` for Zobrist hashing).
- Depends on the `types` crate (`Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`, `File`, `Rank`).