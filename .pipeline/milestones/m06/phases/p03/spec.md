# Phase P03 — Search Extensions

## Goal

Add selective search extensions that deepen the search in tactically critical positions to avoid missing forced tactics, and replace plain alpha-beta with Principal Variation Search (PVS) for more efficient tree exploration. These extensions integrate into the search loop in the `search` crate alongside the pruning and move ordering enhancements from P01 and P02.

## Deliverables

- **T01 — Check Extension**: Extend search by one ply when the side to move is in check. This avoids the horizon effect for forced tactical sequences and ensures that checks near the search horizon are properly resolved. Must improve solve rate on the WAC (Win At Chess) tactical test suite at fixed depth compared to baseline without extensions.
- **T02 — Singular Extension**: If the TT move is significantly better than all alternatives (verified by a reduced-depth search excluding the TT move), extend the TT move by one ply. This ensures that uniquely strong moves are searched more deeply. Requires TT entries with score and depth information to identify candidate moves for singular extension.
- **T03 — Principal Variation Search (PVS)**: Replace plain alpha-beta with PVS. Search the first move (expected best move) with a full alpha-beta window. Search all remaining moves with a zero-width window (alpha, alpha+1). If a zero-window search fails high, re-search with the full window. PVS must find the same or better best move on WAC test suite positions with equal or fewer nodes searched compared to plain alpha-beta at the same depth.

## Technical Decisions & Constraints

- Check extension uses the existing `is_square_attacked` / in-check detection from M01. The extension is applied before any pruning decisions so that positions where the king is in check are never reduced or pruned.
- Singular extension requires a margin parameter (typically ~50–100 centipawns) to determine what qualifies as "significantly better." The reduced-depth verification search uses a zero-window centered around the TT score minus the margin.
- Singular extension should only be attempted at nodes with sufficient remaining depth (e.g., depth ≥ 6) to justify the cost of the verification search.
- PVS requires tracking whether a node is a PV node (first child of a PV node) to determine window type. The first move at each node is searched with the full window; subsequent moves use the null window.
- PVS interacts with LMR: moves that are both late (LMR candidate) and non-PV get both a reduced depth and a null window, providing compounding savings.
- All extensions and PVS must be integrated with the existing iterative deepening framework and transposition table.

## Dependencies

- Depends on M06/P01 (Pruning Techniques) — LMR and futility pruning must be in place for PVS to interact with them correctly.
- Depends on M06/P02 (Move Ordering Improvements) — history, counter-move, and SEE ordering ensure the first move searched at PV nodes is likely the best, which is critical for PVS efficiency.
- Depends on M05 (Transposition Table & Hash) — singular extension relies on TT entries with score/depth; PVS uses TT move as the first move to search.
- Depends on M03 (Basic Search & Evaluation) — negamax framework, iterative deepening, and quiescence search must be functional.
- Depends on M01–M02 (Types, Board, Movegen) — is_square_attacked for check extension, legal move generation for all extensions.