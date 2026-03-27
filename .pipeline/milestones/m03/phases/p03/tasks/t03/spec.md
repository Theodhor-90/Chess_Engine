# T03 — PV Move Ordering

## Objective

Implement a triangular PV table that collects the principal variation during search. After each iterative deepening iteration, the PV is available for the next iteration. During move ordering, the PV move for the current ply gets highest priority (above captures and killers). Clear the PV table at the start of each new search.

## Deliverables

- A triangular PV table implementation for collecting and retrieving the principal variation
- Integration into iterative deepening so the PV from the previous iteration feeds into the next
- PV move given highest ordering priority at each ply

## Key Implementation Details

- **PV table structure**: A triangular PV table (array of arrays) that stores the principal variation at each ply during search. At each node, if the current position's PV move matches an available move, it gets highest ordering priority.
- **Priority order**: PV move (highest) > captures (MVV-LVA) > killer moves > remaining quiet moves (lowest). The PV move is the single most important move to search first.
- **Iterative deepening integration**: After each iterative deepening iteration, the root PV is extracted and used for the next iteration's move ordering.
- **Clear on new search**: The PV table is cleared at the start of each new search.
- **Integration point**: Extend the existing `SearchContext` struct in `crates/search/src/lib.rs` with a PV table field.
- **File to create**: `crates/search/src/pv_table.rs` — triangular PV table for collecting and retrieving the principal variation.
- **Files to modify**:
  - `crates/search/src/ordering.rs` — integrate PV move priority into the move ordering logic (PV move gets highest score).
  - `crates/search/src/lib.rs` — extend `SearchContext` with PV table; update negamax to populate PV table during search; update iterative deepening to extract and use PV.

## Dependencies

- **M01 types crate**: `Move`
- **M03/P02 search crate**: `SearchContext`, negamax, iterative deepening framework
- **T01 (MVV-LVA capture ordering)**: The move ordering module (`ordering.rs`) must exist for PV move scoring integration.
- **T02 (Killer move heuristic)**: The killer move table must be integrated into ordering so that the full priority hierarchy (PV > captures > killers > quiet) is established.

## Verification Criteria

1. The principal variation from the previous iterative deepening iteration is used to order the PV move first at each ply.
2. The PV table correctly collects the full principal variation during search.
3. The PV move gets highest ordering priority (above captures scored by MVV-LVA and killer moves).
4. Move ordering produces a measurable node-count reduction compared to unordered search at the same depth.
5. All existing tests continue to pass (`cargo test`).
6. Code passes `cargo fmt --check` and `cargo clippy -- -D warnings`.