Now I have all the context I need. Let me produce the phase specification.

# Phase P03 — Search Extensions

## Goal

Add selective search extensions — check extension, singular extension, and Principal Variation Search (PVS) — to the search loop in `crates/search/`, deepening the search in tactically critical positions to avoid missing forced tactics and replacing plain alpha-beta with PVS for more efficient tree exploration, building on the pruning techniques from P01 and the move ordering improvements from P02.

## Design Decisions

1. **Check extension placement**: Check extension is applied at the top of the `negamax` function, before any pruning decisions (null move pruning, LMR, futility pruning). When the side to move is in check, the search depth is extended by one ply. This uses the existing in-check detection from M01's `is_square_attacked`. Positions where the king is in check are never reduced or pruned.

2. **Singular extension depth threshold**: Singular extension is only attempted at nodes with remaining depth ≥ 6 to justify the cost of the verification search. This avoids overhead at shallow depths where the extra ply provides minimal benefit.

3. **Singular extension margin and verification search**: A margin parameter (initial value: 64 centipawns) determines what qualifies as "significantly better." The verification search uses a zero-window centered around `tt_score - margin` at a reduced depth of `depth / 2`, excluding the TT move from consideration. If all alternatives score below this threshold, the TT move is extended by one ply.

4. **PVS node type tracking**: PVS requires distinguishing PV nodes from non-PV nodes. The first move at each node is searched with the full `(alpha, beta)` window. All subsequent moves are searched with a zero-window `(alpha, alpha + 1)`. If the zero-window search fails high (score > alpha), a re-search with the full window is performed.

5. **PVS and LMR interaction**: Moves that are both late (LMR candidate) and non-PV receive both a reduced depth and a null window, providing compounding savings. The re-search cascade is: reduced zero-window → full-depth zero-window (on fail-high) → full-depth full-window (on fail-high).

## Tasks

### T01 — Check Extension

Implement check extension in the `negamax` function in `crates/search/src/lib.rs`. When the side to move is in check, increment the search depth by one ply before any pruning decisions. Ensure that nodes in check skip null move pruning, futility pruning, and reverse futility pruning (which should already be guarded by the P01 in-check checks, but verify correctness with the added extension). Include unit tests validating that check extension activates when the side to move is in check, and that it improves solve rate on WAC tactical test suite positions at fixed depth compared to baseline without the extension.

**Deliverables**: Check extension logic in `negamax` in `crates/search/src/lib.rs`; unit tests confirming extension activates in check positions; WAC solve-rate comparison test.

### T02 — Singular Extension

Implement singular extension in `negamax` in `crates/search/src/lib.rs`. When a TT move exists with sufficient depth, perform a reduced-depth verification search (depth `depth / 2`) excluding the TT move with a zero-window at `tt_score - margin` (margin = 64 cp). If all alternatives score below the threshold, extend the TT move's search depth by one ply. Only attempt at nodes with remaining depth ≥ 6. Include unit tests validating that singular extension triggers when the TT move is significantly better than alternatives, and does not trigger when alternatives score comparably.

**Deliverables**: Singular extension logic in `negamax` in `crates/search/src/lib.rs`; singular extension margin constant; unit tests for activation and non-activation conditions.

### T03 — Principal Variation Search (PVS)

Replace the plain alpha-beta search with PVS in `negamax` in `crates/search/src/lib.rs`. Search the first move at each node with the full `(alpha, beta)` window. Search all subsequent moves with a zero-window `(alpha, alpha + 1)`. On zero-window fail-high (score > alpha), re-search with the full window. Integrate with LMR so that late non-PV moves receive both reduced depth and a zero-window, with a re-search cascade: reduced zero-window → full-depth zero-window → full-depth full-window. Validate that PVS finds the same or better best move on WAC test suite positions with equal or fewer nodes searched compared to plain alpha-beta at the same depth.

**Deliverables**: PVS logic replacing plain alpha-beta in `negamax` in `crates/search/src/lib.rs`; PVS + LMR re-search cascade; WAC solve-rate and node-count comparison test.

## Exit Criteria

1. Check extension extends the search depth by one ply when the side to move is in check, and positions in check are not subject to null move pruning, LMR, or futility pruning
2. Singular extension triggers only when a TT move exists at sufficient depth (≥ 6), and a verification search confirms all alternatives score below `tt_score - margin`
3. Check and singular extensions improve solve rate on the WAC tactical test suite at fixed depth compared to baseline without extensions
4. PVS finds the same or better best move on WAC test suite positions with equal or fewer nodes searched compared to plain alpha-beta at the same depth
5. PVS interacts correctly with LMR, producing a reduced zero-window → full-depth zero-window → full-depth full-window re-search cascade

## Dependencies

- **M06/P01 (Pruning Techniques)** — null move pruning, LMR, and futility pruning must be in place so that check extension correctly interacts with pruning guards, and PVS integrates with LMR re-search
- **M06/P02 (Move Ordering Improvements)** — history, counter-move, and SEE ordering ensure the first move searched at PV nodes is likely the best, which is critical for PVS efficiency
- **M05 (Transposition Table & Hash)** — TT entries with score and depth information are required for singular extension to identify candidate moves; TT move is used as the first move for PVS
- **M03 (Basic Search & Evaluation)** — negamax framework, iterative deepening, and quiescence search must be functional
- **M01–M02 (Types, Board, Movegen)** — `is_square_attacked` for check extension, legal move generation for all extensions

## Artifacts

- `crates/search/src/lib.rs` — modified to add check extension, singular extension, and PVS logic to `negamax`, replacing plain alpha-beta
- Unit tests within `crates/search/` for check extension activation, singular extension triggering conditions, and PVS correctness
- WAC solve-rate and node-count comparison tests within `crates/search/` validating all three techniques