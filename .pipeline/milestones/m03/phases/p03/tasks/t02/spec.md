# T02 — Killer Move Heuristic

## Objective

Add a killer move table (2 slots per ply) to `SearchContext` that stores quiet moves causing beta cutoffs. During move ordering, give killer moves priority below captures but above other quiet moves. Validate killers as pseudo-legal in the current position before promoting them.

## Deliverables

- A killer move table implementation (2 slots per ply, fixed-size array indexed by ply)
- Integration into the negamax search for recording killers on beta cutoff and using them during move ordering
- Pseudo-legality validation of killer moves before use

## Key Implementation Details

- **Killer storage**: Two killer slots per ply, stored in a fixed-size array indexed by ply. When a quiet move causes a beta cutoff, it replaces the second slot and the former first slot shifts down.
- **Validation**: Killers originate from sibling nodes, so they must be validated as pseudo-legal before being given ordering priority in the current position.
- **Priority**: In the move ordering hierarchy: PV move (highest) > captures (MVV-LVA) > killer moves > remaining quiet moves (lowest).
- **Integration point**: Extend the existing `SearchContext` struct in `crates/search/src/lib.rs` with a killer table field, cleared at the start of each new search.
- **File to create**: `crates/search/src/killer.rs` — killer move table implementation.
- **Files to modify**:
  - `crates/search/src/ordering.rs` — integrate killer move scoring into the move ordering logic (killers scored below captures but above quiet moves).
  - `crates/search/src/lib.rs` — extend `SearchContext` with killer table; update negamax to record killer moves on beta cutoff of quiet moves.

## Dependencies

- **M01 types crate**: `Move`, `MoveFlag`, `PieceKind`
- **M01 board crate**: `Position`
- **M03/P02 search crate**: `SearchContext`, negamax framework
- **T01 (MVV-LVA capture ordering)**: The move ordering module (`ordering.rs`) created in T01 must exist so that killer move scoring can be integrated into it.

## Verification Criteria

1. Killer moves that caused beta cutoffs are stored per ply (2 slots per ply) with proper slot rotation.
2. Killer moves are validated as pseudo-legal before being given ordering priority.
3. During move ordering, killer moves are prioritized below captures but above other quiet moves.
4. All existing tests continue to pass (`cargo test`).
5. Code passes `cargo fmt --check` and `cargo clippy -- -D warnings`.