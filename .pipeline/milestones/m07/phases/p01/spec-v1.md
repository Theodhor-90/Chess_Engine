# Phase P01 — Pawn Structure

## Goal

Implement a pawn hash table and comprehensive pawn structure evaluation terms — doubled, isolated, backward, passed, and connected pawns — within the `eval` crate, along with advanced passed pawn scoring incorporating king proximity and blockade detection. This phase adds the first layer of positional knowledge beyond material and piece-square tables, cached via a dedicated pawn hash table for efficient reuse.

## Design Decisions

1. **Pawn Zobrist hash derived from existing keys**: The pawn hash key is computed by XOR-ing the `PIECE_SQUARE_KEYS` entries (from `crates/board/src/zobrist.rs`) for all white and black pawns on the board. No new random key tables are introduced. This is computed on demand from the position's pawn bitboards and the existing key array.

2. **Power-of-two table sizing with AND masking**: The pawn hash table defaults to 16K entries, using bitwise AND (`hash & (size - 1)`) for index computation instead of modulo, consistent with the transposition table pattern from M05.

3. **Separate midgame and endgame scores in cache entries**: Each pawn hash table entry stores both a midgame and an endgame score so that cached results integrate directly into the tapered evaluation framework without recomputation.

4. **Passed pawn evaluation split across T02 and T03**: T02 detects passed pawns and assigns a rank-based advancement bonus. T03 extends passed pawn scoring with king proximity and blockade detection, which require access to the full position (not just pawn structure), making them a natural separate task.

5. **All terms produce mg/eg score pairs**: Every pawn evaluation term contributes separate middlegame and endgame scores, consistent with the existing tapered evaluation framework (`compute_phase` in `crates/eval/src/phase.rs`).

## Tasks

### T01 — Pawn Hash Table

Implement the pawn hash table data structure and its probe/store interface within the `eval` crate.

**Deliverables:**
- A `PawnHashEntry` struct storing: pawn Zobrist key (or verification bits), midgame score, endgame score
- A `PawnHashTable` struct with a fixed-size `Vec<PawnHashEntry>` (default 16K entries, power-of-two)
- A function to compute the pawn Zobrist hash from a `Position` by XOR-ing `PIECE_SQUARE_KEYS` entries for all pawns on the board
- `probe(&self, key: u64) -> Option<(i32, i32)>` — returns cached (mg, eg) scores on hit
- `store(&mut self, key: u64, mg: i32, eg: i32)` — writes a new entry
- Unit tests verifying: probe miss returns `None`, store followed by probe returns correct scores, different pawn configurations produce different hash keys

### T02 — Pawn Structure Terms

Implement evaluation terms for doubled, isolated, backward, passed, and connected pawns.

**Deliverables:**
- A function that analyzes a position's pawn structure and returns (mg_score, eg_score) for White's pawn evaluation
- Detection and scoring of: doubled pawns (penalty), isolated pawns (penalty), backward pawns (penalty), passed pawns (bonus scaled by advancement rank), connected pawns (bonus)
- Each term contributes separate midgame and endgame scores
- Evaluation is computed for both colors and combined (White score minus Black score)
- Unit tests verifying detection of each pawn structure pattern on known FEN positions (e.g., a position with a doubled pawn correctly receives the doubled penalty)

### T03 — Passed Pawn Evaluation

Extend passed pawn scoring with king proximity and blockade detection, and integrate the full pawn evaluation into the engine's `evaluate()` function via the pawn hash table.

**Deliverables:**
- Passed pawn advancement bonus that increases exponentially toward promotion rank
- King proximity bonus: bonus when the friendly king is close to a passed pawn, bonus when the opponent king is far from it
- Blockade penalty: penalty when the passed pawn's advance square is occupied by any piece
- Integration into `evaluate()` in `crates/eval/src/lib.rs`: probe the pawn hash table before computing pawn structure evaluation; on miss, compute and store; combine the pawn mg/eg scores with existing evaluation via tapered interpolation
- Unit tests verifying: passed pawn on rank 6 scores higher than on rank 4, blocked passed pawn scores lower than unblocked, king proximity affects the score

## Exit Criteria

1. Pawn hash table is probed before computing pawn evaluation; on a hit, cached midgame/endgame scores are returned; on a miss, the computed result is stored into the table before returning
2. Pawn structure evaluation correctly identifies and scores doubled, isolated, backward, passed, and connected pawns with unit tests verifying detection on known FEN positions
3. Passed pawn evaluation awards bonuses scaled by advancement rank, adjusts for king proximity, and penalizes blocked passed pawns
4. All pawn evaluation terms contribute separate middlegame and endgame scores that are combined via the existing tapered evaluation phase calculation from M03
5. `cargo test` passes for all new and existing tests in the `eval` crate
6. `cargo clippy -- -D warnings` produces no warnings for the `eval` crate

## Dependencies

- **M06 (Advanced Search Techniques)** — must be complete
- **`eval` crate (M03)** — existing tapered evaluation with `evaluate()`, `material_score()`, `mg_table`/`eg_table`, and `compute_phase()`
- **`board` crate** — `Position` struct with piece bitboards, `PIECE_SQUARE_KEYS` in `crates/board/src/zobrist.rs`
- **`types` crate** — `Bitboard`, `Square`, `Piece`, `PieceKind`, `Color`, `File`, `Rank` with iteration and bitwise operations

## Artifacts

- `crates/eval/src/pawn.rs` — new module containing `PawnHashTable`, `PawnHashEntry`, pawn Zobrist computation, and pawn structure evaluation functions
- `crates/eval/src/lib.rs` — modified to integrate pawn evaluation into `evaluate()` via pawn hash table probe/store