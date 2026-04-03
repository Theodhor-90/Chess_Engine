# T02 — Search Integration

## Objective

Integrate Syzygy tablebase probing into the engine's search, both at the root (to filter losing moves and prefer the fastest winning path) and during in-search nodes (to return exact scores at low piece counts), gated by piece count against a configurable probe depth limit.

## Deliverables

- Root search tablebase integration: use WDL to filter out losing moves so the engine never plays a theoretically lost move when tablebase data is available
- Root DTZ usage: among WDL-equivalent moves at the root, prefer the fastest winning path (lowest DTZ for wins) or the longest path to loss (highest DTZ for losses)
- In-search WDL probing: at nodes where the piece count is at or below the configured probe depth limit, use WDL probes to return exact win/draw/loss scores, short-circuiting normal evaluation
- Piece count gating: all probing gated by piece count versus the configured probe depth limit to avoid unnecessary overhead

## Key Implementation Details

- **Root filtering**: After generating legal moves at the root, probe WDL for each resulting position. Exclude moves that lead to a lost position if any non-losing moves exist. Among winning moves, use DTZ to select the one with the shortest path to conversion.
- **In-search probing**: At non-root nodes, if piece count <= configured probe depth, call WDL probe. Convert result to an exact score (mate-scale for wins/losses, 0 for draws) and return immediately without further evaluation or deeper search.
- **Score conversion**: Wins map to large positive scores (near mate), losses map to large negative scores, draws map to 0. Adjust mate-scale scores for ply distance to ensure correct ordering in search.
- **Performance**: WDL probes are cheaper than DTZ; use WDL during search and DTZ only at root.
- **Graceful degradation**: If probing fails (e.g., missing tablebase files for a given piece count), fall through to normal search without error.

## Files to Create or Modify

- Modify: `crates/search/` — integrate tablebase probes into the root search loop and in-search logic
- Modify: Search function(s) to check piece count and call WDL probes at appropriate nodes
- Possibly modify: Root move filtering logic in the search or engine crate

## Dependencies

- **T01** (Syzygy tablebase probing) must be complete — the probe interface and initialization function are required
- `crates/search/` must have a functioning search implementation (guaranteed by M06+ completion)

## Verification Criteria

1. In a known won endgame position (e.g., KRK), the engine at the root filters out losing/drawing moves and selects a winning move
2. In a known drawn endgame (e.g., KBK), the engine returns a draw score
3. In-search probes at low piece counts return exact scores matching tablebase WDL
4. Probing is correctly gated: no probing occurs when piece count exceeds the configured limit
5. When tablebase files are missing for a piece count, the engine falls through to normal search gracefully
6. `cargo test` passes for modified search crate
7. `cargo clippy -- -D warnings` passes
8. `cargo fmt --check` passes