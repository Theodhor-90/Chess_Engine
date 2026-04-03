# T03 — Passed Pawn Evaluation

## Objective

Extend passed pawn scoring with king proximity bonuses and blockade detection, and integrate the full pawn evaluation (from T01 and T02) into the engine's `evaluate()` function via the pawn hash table.

## Deliverables

- **Passed pawn advancement bonus** that increases exponentially toward promotion rank (e.g., rank 7 bonus >> rank 5 bonus >> rank 4 bonus)
- **King proximity bonus**: bonus when the friendly king is close to a passed pawn (Chebyshev distance), bonus when the opponent king is far from the passed pawn
- **Blockade penalty**: penalty when the passed pawn's advance square (one square ahead toward promotion) is occupied by any piece
- **Integration into `evaluate()`** in `crates/eval/src/lib.rs`:
  1. Compute the pawn Zobrist hash from the position
  2. Probe the pawn hash table
  3. On hit, use the cached (mg, eg) pawn scores
  4. On miss, call the pawn structure evaluation function (from T02), store the result in the pawn hash table, then use the scores
  5. Add the king-proximity and blockade adjustments (these depend on the full position, not just pawns, so they are computed outside the cache)
  6. Combine the pawn mg/eg scores with existing evaluation via the tapered interpolation using `compute_phase()`
- Unit tests verifying:
  - A passed pawn on rank 6 scores higher than one on rank 4
  - A blocked passed pawn scores lower than an unblocked one
  - King proximity to a passed pawn affects the score (friendly king close = higher score, opponent king close = lower score)
  - Pawn hash table caching works end-to-end: first call computes and stores, second call with same pawn structure retrieves from cache

## Files to Create or Modify

- **Modify**: `crates/eval/src/pawn.rs` — add passed pawn king proximity and blockade evaluation functions
- **Modify**: `crates/eval/src/lib.rs` — integrate pawn evaluation into `evaluate()` via pawn hash table probe/store; combine pawn scores into the tapered evaluation result

## Key Implementation Details

- King proximity and blockade detection require access to the full `Position` (not just pawn bitboards), which is why they are evaluated outside the pawn hash table cache — the cache only stores the pawn-structure-only terms from T02
- Use Chebyshev distance (max of file distance and rank distance) for king-to-pawn proximity measurement
- The advancement bonus should scale non-linearly (e.g., exponentially or via a lookup table indexed by rank): promotion-adjacent passed pawns are dramatically more valuable
- The pawn hash table must be accessible from `evaluate()` — it can be passed as a `&mut PawnHashTable` parameter or stored in a shared location accessible during search
- Ensure the `evaluate()` function signature change (if any) is propagated to all call sites in the search crate
- All new terms produce separate middlegame and endgame scores for tapered evaluation

## Dependencies

- **T01 (Pawn Hash Table)** — the pawn hash table must be implemented for caching
- **T02 (Pawn Structure Terms)** — the pawn structure evaluation function must exist to compute the cacheable pawn scores
- Depends on existing `eval` crate (`evaluate()`, `compute_phase()` in `crates/eval/src/phase.rs`, `mg_table`/`eg_table`)
- Depends on existing `board` crate (`Position` struct with piece bitboards, king square access)
- Depends on existing `types` crate (`Square` with distance/file/rank methods)

## Verification Criteria

1. Passed pawn on rank 6 receives a higher bonus than one on rank 4
2. A blocked passed pawn (advance square occupied) receives a lower score than an unblocked passed pawn
3. A passed pawn with the friendly king nearby scores higher than one with the friendly king far away
4. A passed pawn with the opponent king nearby scores lower than one with the opponent king far away
5. Pawn hash table integration works: first evaluation computes and stores; second evaluation of the same position retrieves cached scores
6. All pawn evaluation terms (doubled, isolated, backward, passed, connected, king proximity, blockade) are reflected in the final `evaluate()` output
7. The tapered evaluation correctly combines pawn mg/eg scores with existing material and PST scores
8. `cargo test` passes for all new and existing tests in the `eval` crate
9. `cargo clippy -- -D warnings` produces no warnings for the `eval` crate
10. No regressions in existing search/eval integration (engine still plays correctly via UCI)