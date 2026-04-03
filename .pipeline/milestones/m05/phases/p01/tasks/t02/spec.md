# T02 — TT Hash Table with Replacement Policy

## Objective

Implement the `TranspositionTable` struct that wraps a `Vec<TtEntry>` with power-of-two sizing and provides probe/store operations with a depth-preferred, age-based replacement policy.

## Deliverables

- `TranspositionTable` struct added to `crates/search/src/tt.rs` (same file as T01)
- Methods:
  - `new(mb: usize) -> Self` — allocate table sized to the largest power-of-two entry count that fits in the given megabytes
  - `probe(hash: u64) -> Option<TtEntry>` — look up an entry by Zobrist hash; return `Some(entry)` if the key verification bits match, `None` otherwise
  - `store(hash: u64, entry: TtEntry)` — store an entry, applying the replacement policy
  - `clear()` — zero out all entries
  - `resize(mb: usize)` — reallocate the table to a new size
  - `new_generation()` — increment the internal age/generation counter (called at the start of each new search)
- Index computation: `hash & (num_entries - 1)` (bitmask, no modulo)
- Replacement policy: new entry replaces existing if (a) existing entry is from a prior generation (stale), OR (b) new entry has equal or greater depth than existing. Otherwise, keep the existing entry.

## Implementation Constraints

- Power-of-two sizing is mandatory — compute the largest power-of-two count of `TtEntry` that fits in the requested MB
- No external dependencies — standard library only
- The table must be efficient for repeated probe/store in a hot search loop
- The generation counter wraps or saturates safely (u8 age field)
- Follow project coding conventions

## Dependencies

- **T01** (this phase) — requires `TtEntry` and `BoundType` to be defined

## Verification Criteria

1. Unit test: store an entry, probe it back, verify all fields match
2. Unit test: replacement policy — deeper entry is NOT replaced by shallower entry of the same age
3. Unit test: replacement policy — stale entry (old generation) IS replaced by shallower new-generation entry
4. Unit test: replacement policy — entry with equal depth and same age IS replaced (new data preferred)
5. Unit test: power-of-two sizing — requesting e.g. 16 MB results in a table whose entry count is a power of two and total size ≤ 16 MB
6. Unit test: `clear()` makes all probes return `None`
7. Unit test: `new_generation()` increments the age so subsequent stores use the new generation
8. `cargo test -p chess-search` passes
9. `cargo clippy -- -D warnings` produces no warnings
10. `cargo fmt --check` passes