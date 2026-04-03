The feedback has a single issue: the file begins with agent commentary on line 1 that must be removed. The rest of the spec was praised as "well-structured, fully traceable, and free of ambiguity or scope creep." I'll output the complete specification with the preamble removed.

# Phase P01 — Pruning Techniques

## Goal

Implement null move pruning, late move reductions (LMR), and futility pruning within the existing negamax/alpha-beta search loop in `crates/search/`, reducing the effective branching factor by skipping moves or subtrees unlikely to affect the search result, validated by node count benchmarks and tactical solve-rate preservation.

## Design Decisions

1. **Null move mechanism**: Implement a `make_null_move` method on `Position` (in `crates/board/src/position.rs`) that flips the side to move, clears the en passant square, and updates the Zobrist hash — without moving any piece. A corresponding `unmake_null_move` restores the prior state. This keeps the null move concept at the board level where position manipulation belongs.

2. **Zugzwang guard for null move pruning**: Disable null move pruning when the side to move has zero non-pawn material (only king and pawns remain). This is checked by inspecting the piece bitboards for knights, bishops, rooks, and queens of the side to move. This simple heuristic avoids the most common zugzwang-prone positions without complex verification searches.

3. **Null move reduction depth**: Use R=2 when remaining depth ≤ 6, and R=3 when remaining depth > 6. The null move search is performed with a zero-window around beta: `negamax(pos, depth - 1 - R, -beta, -beta + 1, ...)`.

4. **Mate score threshold for null move pruning**: Null move pruning is disabled when `beta.abs() >= MATE_SCORE - MAX_PLY`, where `MATE_SCORE` is the engine's constant representing checkmate (e.g., 30000) and `MAX_PLY` is the maximum search depth (e.g., 128). This prevents pruning in positions where mate scores need exact resolution.

5. **LMR reduction table**: Pre-compute a reduction table indexed by `(depth, move_index)` using the formula `floor(ln(depth) * ln(move_index) / 1.75)` where the divisor constant C=1.75 is the initial tuning value. Only reduce quiet, non-killer, non-TT, non-PV moves that do not give check. Re-search at full depth if the reduced search returns a score above alpha.

6. **Futility pruning margins**: Use fixed depth-scaled margins for both futility pruning and reverse futility pruning:

   | Depth | Futility Margin (static eval + margin < alpha) | Reverse Futility Margin (static eval - margin ≥ beta) |
   |-------|------------------------------------------------|-------------------------------------------------------|
   | 1     | 200 cp                                         | 100 cp                                                |
   | 2     | 400 cp                                         | 300 cp                                                |
   | 3     | 600 cp                                         | 500 cp                                                |

   Apply futility pruning only to quiet moves at depth ≤ 3. Skip pruning when in check or when the move gives check.

7. **Static eval caching**: Since futility pruning and reverse futility pruning both require a static evaluation at each node, compute the static eval once at the top of `negamax` and reuse it. The eval function from `crates/eval/` already provides this.

8. **Integration approach**: All three pruning techniques are added as conditional blocks within the existing `negamax` function in `crates/search/src/lib.rs`. No new modules are needed — the pruning logic is local to the search loop. The `make_null_move`/`unmake_null_move` methods are the only additions outside `crates/search/`.

## Tasks

### T01 — Null Move Pruning

Add `make_null_move` and `unmake_null_move` to `Position` in `crates/board/src/position.rs`. Implement null move pruning in `negamax` in `crates/search/src/lib.rs`: before searching child moves, perform a null-move search at reduced depth with a zero-window around beta. If the null-move search score ≥ beta, return beta (prune). Disable when: (a) the side to move has zero non-pawn material, (b) the current node is already a null-move search (no consecutive null moves), (c) the position is in check, or (d) `beta.abs() >= MATE_SCORE - MAX_PLY`. Include unit tests validating that null-move pruning activates in positions where both sides have non-pawn material and are not in check, and is skipped in king-and-pawns-only positions.

**Deliverables**: `Position::make_null_move` and `Position::unmake_null_move` methods in `crates/board/src/position.rs`; null move pruning logic in `negamax` in `crates/search/src/lib.rs`; unit tests for correctness and zugzwang guard.

### T02 — Late Move Reductions (LMR)

Implement a pre-computed LMR reduction table using the formula `floor(ln(depth) * ln(move_index) / 1.75)` and integrate late move reductions into `negamax` in `crates/search/src/lib.rs`. After searching the first few moves at full depth, reduce the search depth for later quiet moves that are not TT moves, PV moves, killers, or moves that give check. If the reduced search returns a score above alpha, re-search at full depth. Validate that LMR achieves ≥30% node count reduction at depth 10 on the standard benchmark position set (startpos, Kiwipete, perft suite positions) compared to baseline without LMR, with no regression in WAC solve rate.

**Deliverables**: LMR reduction table; LMR logic in `negamax` with re-search on fail-high; benchmark test confirming ≥30% node reduction at depth 10.

### T03 — Futility Pruning

Implement futility pruning and reverse futility pruning in `negamax` in `crates/search/src/lib.rs`. At frontier nodes (depth ≤ 1), if the static eval plus the futility margin (depth 1: 200 cp) is below alpha, skip quiet moves (futility pruning). At pre-frontier nodes (depth ≤ 3), if the static eval minus the reverse futility margin (depth 1: 100 cp, depth 2: 300 cp, depth 3: 500 cp) exceeds beta, return the static eval (reverse futility pruning). Do not apply either technique when in check, for moves that give check, or for tactical moves (captures, promotions). Validate that the WAC tactical test suite solve rate is not reduced at fixed depth compared to baseline without futility pruning.

**Deliverables**: Futility pruning and reverse futility pruning logic in `negamax`; margin constants; WAC solve-rate validation test.

## Exit Criteria

1. Null move pruning correctly prunes futile branches and is disabled when the side to move has zero non-pawn material (king and pawns only), when in check, during consecutive null-move searches, and when beta is a mate score
2. LMR achieves ≥30% node count reduction at depth 10 on the standard benchmark position set (startpos, Kiwipete, perft suite positions) compared to baseline without LMR, with no net Elo loss in self-play
3. Futility pruning does not reduce the solve rate on the WAC tactical test suite at fixed depth compared to baseline without futility pruning
4. All three techniques are integrated into the existing `negamax` function and interact correctly with TT probing, killer moves, PV ordering, and quiescence search

## Dependencies

- **M05 (Transposition Table & Hash)** — TT probing and TT move ordering must be in place
- **M03 (Basic Search & Evaluation)** — negamax with alpha-beta, quiescence search, iterative deepening, MVV-LVA ordering, killer moves, and PV move ordering must be functional
- **M01–M02 (Types, Board, Movegen)** — legal move generation, make/unmake, and `is_square_attacked` must be correct and performant
- No dependency on other M06 phases (P02, P03)

## Artifacts

- `crates/board/src/position.rs` — modified to add `Position::make_null_move` and `Position::unmake_null_move`
- `crates/search/src/lib.rs` — modified to add null move pruning, LMR, and futility pruning to `negamax`
- Unit tests within `crates/board/` for null move correctness
- Unit tests and benchmark tests within `crates/search/` for pruning validation