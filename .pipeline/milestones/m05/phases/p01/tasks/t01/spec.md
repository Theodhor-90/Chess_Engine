# T01 — TT Entry Structure

## Objective

Define the `TtEntry` struct and the `BoundType` enum in a new `crates/search/src/tt.rs` module. This is the foundational data structure for the transposition table — every subsequent task depends on it.

## Deliverables

- New file: `crates/search/src/tt.rs`
- `BoundType` enum with three variants: `Exact`, `LowerBound`, `UpperBound`
- `TtEntry` struct containing:
  - **Key verification bits** — 16-bit fragment of the upper Zobrist hash bits (not used for table indexing), for collision filtering
  - **Depth** — 8-bit search depth at which this entry was computed
  - **Score** — 32-bit search score
  - **Bound type** — 2-bit `BoundType` (exact, lower, upper)
  - **Best move** — 16-bit raw `Move` bits (the best move found at this node)
  - **Age** — 8-bit generation counter for replacement policy decisions
- Constructor and accessor methods for all fields
- The struct should be compact and cache-friendly
- Register `tt` as a module in `crates/search/src/lib.rs` (or `mod.rs`)

## Implementation Constraints

- No external dependencies — standard library only, per project convention
- Follow project coding conventions: `snake_case` functions, `PascalCase` types, `///` doc comments on public items
- Use `#[derive(...)]` for `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq` as appropriate
- `BoundType` should support safe conversion from/to integer representation
- The entry design must allow a default/empty state to initialize the table vector

## Dependencies

- `crates/types/` — uses the `Move` type (16-bit packed integer)
- No dependencies on T02 or T03 within this phase

## Verification Criteria

1. Unit tests verify round-trip correctness: construct a `TtEntry` with known values, read back all fields, assert equality
2. Unit tests cover all `BoundType` variants
3. Unit tests verify field accessors return exactly what was passed to the constructor
4. `cargo test -p chess-search` passes (or the equivalent package name for the search crate)
5. `cargo clippy -- -D warnings` produces no warnings for the new module
6. `cargo fmt --check` passes