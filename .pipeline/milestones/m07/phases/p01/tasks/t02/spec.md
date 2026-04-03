# T02 — Pawn Structure Terms

## Objective

Implement evaluation terms for doubled, isolated, backward, passed, and connected pawns. Each term produces separate midgame and endgame scores for integration with the existing tapered evaluation framework.

## Deliverables

- A function (e.g., `evaluate_pawns(position: &Position) -> (i32, i32)`) that analyzes a position's pawn structure and returns `(mg_score, eg_score)` representing White's pawn structure evaluation advantage
- Detection and scoring of the following pawn structure patterns:
  - **Doubled pawns** — two or more friendly pawns on the same file; penalty per extra pawn on a file
  - **Isolated pawns** — a pawn with no friendly pawns on adjacent files; penalty
  - **Backward pawns** — a pawn that cannot advance without being captured by an opponent pawn and has no friendly pawn on adjacent files that could support its advance; penalty
  - **Passed pawns** — a pawn with no opponent pawns ahead on its file or adjacent files that could block or capture it; bonus scaled by advancement rank (higher ranks = larger bonus)
  - **Connected pawns** — pawns that are on adjacent files and protect each other (a pawn that is defended by or defends a friendly pawn on an adjacent file); bonus
- Each term contributes separate midgame and endgame score components
- Evaluation is computed for both colors and combined (White score minus Black score)
- Unit tests verifying detection of each pawn structure pattern on known FEN positions:
  - A position with doubled pawns correctly receives the doubled penalty
  - A position with an isolated pawn correctly receives the isolated penalty
  - A position with a backward pawn correctly receives the backward penalty
  - A position with a passed pawn correctly receives the passed bonus
  - A position with connected pawns correctly receives the connected bonus

## Files to Create or Modify

- **Modify**: `crates/eval/src/pawn.rs` — add pawn structure evaluation functions and constants for penalties/bonuses

## Key Implementation Details

- Use bitboard operations for efficient pawn structure analysis (e.g., shifting pawn bitboards to detect adjacency, using file masks to detect doubled/isolated pawns)
- Passed pawn detection: a pawn is passed if there are no opponent pawns on its file or adjacent files that are at the same rank or ahead (toward promotion)
- Backward pawn detection: a pawn is backward if it cannot be supported by a friendly pawn on adjacent files (no friendly pawns on adjacent files at the same rank or behind) AND advancing it would put it under attack by an opponent pawn
- Connected pawn detection: check if a pawn is defended by a friendly pawn on an adjacent file (pawn attacks)
- Passed pawn bonus in T02 is rank-based only; king proximity and blockade detection are handled in T03
- All bonus/penalty constants should be defined as named constants for tuning

## Dependencies

- **T01 (Pawn Hash Table)** — the pawn hash table structure must exist so T02's evaluation function can eventually be cached through it (though the actual cache integration into `evaluate()` is in T03)
- Depends on existing `types` crate (`Bitboard`, `Square`, `Color`, `File`, `Rank` with bitwise operations and iteration)
- Depends on existing `board` crate (`Position` struct with pawn bitboards)

## Verification Criteria

1. `evaluate_pawns()` returns `(mg_score, eg_score)` with correct sign (positive = White advantage)
2. A position with doubled white pawns on the e-file produces a penalty in the evaluation
3. A position with an isolated white pawn produces a penalty
4. A position with a backward white pawn produces a penalty
5. A position with a white passed pawn produces a bonus that increases with rank advancement
6. A position with connected white pawns produces a bonus
7. Evaluation handles both colors correctly (Black penalties also detected and subtracted)
8. `cargo test` passes for all new and existing tests in the `eval` crate
9. `cargo clippy -- -D warnings` produces no warnings for the `eval` crate