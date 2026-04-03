Now I have all the context. Let me produce the revised plan.

## 1. Deliverables

| # | File | Action | Purpose |
|---|------|--------|---------|
| 1 | `crates/search/src/tt.rs` | Modify | Add `TranspositionTable` struct with `probe`, `store`, `clear`, `resize`, `new_generation`, `generation` methods and unit tests |

## 2. Dependencies

- **T01 (completed)**: `TtEntry`, `BoundType`, `verification_key` — already present in `crates/search/src/tt.rs`
- **No new crate dependencies** — standard library only, consistent with project conventions

## 3. Implementation Details

### `TranspositionTable` struct (in `crates/search/src/tt.rs`)

**Fields:**
- `entries: Vec<TtEntry>` — the backing storage, length is always a power of two
- `mask: usize` — bitmask for index computation (`num_entries - 1`); avoids recomputing from `entries.len()`
- `generation: u8` — current search generation; incremented each new search via `new_generation()`

**Methods:**

#### `pub fn new(mb: usize) -> Self`
- Compute the number of bytes available: `mb * 1024 * 1024`.
- Compute entry size: `std::mem::size_of::<TtEntry>()`.
- Compute `max_entries = bytes / entry_size`.
- If `max_entries == 0` (because `mb == 0` or `TtEntry` is larger than the requested allocation), default to `num_entries = 1`.
- Otherwise, round down to the largest power of two ≤ `max_entries` using: `1 << (usize::BITS - 1 - max_entries.leading_zeros())`.
- Allocate `Vec` of `num_entries` elements filled with `TtEntry::default()`.
- Set `mask = num_entries - 1`.
- Set `generation = 0`.

#### `pub fn probe(&self, hash: u64) -> Option<TtEntry>`
- Compute index: `(hash as usize) & self.mask`.
- Read `self.entries[index]`.
- Compute `verification_key(hash)` (the upper 16 bits of `hash`).
- If `entry.key() == verification_key(hash)`, return `Some(entry)`.
- Otherwise return `None`.

No additional empty-entry guard is performed. A default `TtEntry` has `key = 0`, so it would only false-match a hash whose upper 16 bits are `0x0000`. This is rare and harmless — such an entry has depth 0 and score 0, so it will never cause a meaningful cutoff in search. This is standard practice in chess engine TTs.

#### `pub fn store(&mut self, hash: u64, entry: TtEntry)`
- Compute index: `(hash as usize) & self.mask`.
- Read existing entry at that index.
- Apply replacement policy:
  - If existing entry's `age() != self.generation` → replace (stale entry from prior generation).
  - Else if `entry.depth() >= existing.depth()` → replace (equal or greater depth preferred).
  - Else → do not replace (keep existing deeper entry of same generation).
- Write the new entry to `self.entries[index]` if replacing.

The caller is responsible for constructing the `TtEntry` with the correct key (via `verification_key(hash)`) and age (via `self.generation()`).

#### `pub fn clear(&mut self)`
- Fill all entries with `TtEntry::default()`.
- Reset `generation` to 0.

#### `pub fn resize(&mut self, mb: usize)`
- Recompute the power-of-two entry count for the new size using the same logic as `new`. If `mb == 0` or computed count is 0, default to 1 entry.
- Allocate a new `Vec<TtEntry>` of that size filled with defaults.
- Update `self.entries` and `self.mask`.
- Reset `generation` to 0.

#### `pub fn new_generation(&mut self)`
- Increment `self.generation` with wrapping: `self.generation = self.generation.wrapping_add(1)`.

#### `pub fn generation(&self) -> u8`
- Return `self.generation`.

**Exports:** `TranspositionTable` is `pub` so it can be used from `lib.rs` in T03.

## 4. API Contracts

### `TranspositionTable::new(mb: usize) -> TranspositionTable`
- **Input:** Size in megabytes (e.g., `16`).
- **Output:** A `TranspositionTable` with the largest power-of-two entry count fitting in that many megabytes.
- **Edge case:** If `mb == 0` or the entry count computes to 0, the table is created with exactly 1 entry (minimum viable table).

### `TranspositionTable::probe(hash: u64) -> Option<TtEntry>`
- **Input:** 64-bit Zobrist hash.
- **Output:** `Some(entry)` if `entry.key() == verification_key(hash)`, `None` otherwise.
- **Example:** `tt.probe(0xABCD_0000_0000_0001)` returns `Some(entry)` if an entry was stored for a hash whose upper 16 bits are `0xABCD` at index `1 & mask`.

### `TranspositionTable::store(hash: u64, entry: TtEntry)`
- **Input:** 64-bit Zobrist hash, a `TtEntry` (caller sets key via `verification_key(hash)` and age to `tt.generation()`).
- **Output:** None. Entry is stored only if the replacement policy permits it.

### `TranspositionTable::clear()`
- **Input:** None.
- **Output:** None. All entries are zeroed and generation is reset to 0.

### `TranspositionTable::resize(mb: usize)`
- **Input:** New size in megabytes.
- **Output:** None. Table is reallocated and cleared.
- **Edge case:** If `mb == 0` or the entry count computes to 0, the table is resized to exactly 1 entry (minimum viable table).

### `TranspositionTable::new_generation()`
- **Input:** None.
- **Output:** None. Internal generation counter is incremented with wrapping (`u8::wrapping_add(1)`).

### `TranspositionTable::generation() -> u8`
- **Input:** None.
- **Output:** Current generation value. Callers use this to set the `age` field on entries they construct before passing to `store`.

## 5. Test Plan

All tests are added to the existing `#[cfg(test)] mod tests` block in `crates/search/src/tt.rs`.

### Test 1: `store_and_probe_round_trip`
- Create a `TranspositionTable` with 1 MB.
- Construct a `TtEntry` with key = `verification_key(hash)`, age = `tt.generation()`, and known depth/score/bound/move values.
- Call `tt.store(hash, entry)`.
- Call `tt.probe(hash)`.
- Assert `Some(entry)` with all fields matching.

### Test 2: `probe_returns_none_for_missing`
- Create a table, store an entry at one hash.
- Probe with a different hash (different index AND different upper 16 bits).
- Assert `None`.

### Test 3: `shallower_does_not_replace_deeper_same_age`
- Store entry A with depth 10 at some hash.
- Store entry B with depth 5 at the same hash (same index, same generation).
- Probe and assert entry A is still present (shallower entry does not replace deeper entry of the same age).

### Test 4: `replacement_stale_entry_replaced_by_shallower`
- Store entry A with depth 10 at generation 0.
- Call `new_generation()`.
- Store entry B with depth 3 at the same hash (now generation 1, with age set to 1).
- Probe and assert entry B is present (stale entry replaced regardless of depth).

### Test 5: `replacement_equal_depth_same_age_replaces`
- Store entry A with depth 5.
- Store entry B with depth 5 at the same hash (same generation), with different score or move to distinguish.
- Probe and assert entry B is present (equal depth replaces).

### Test 6: `power_of_two_sizing`
- Create a table with 16 MB.
- Assert entry count is a power of two (using `count & (count - 1) == 0`).
- Assert total memory (`entry_count * size_of::<TtEntry>()`) ≤ 16 MB.

### Test 7: `clear_makes_all_probes_none`
- Store several entries at different hashes.
- Call `clear()`.
- Probe all previously stored hashes.
- Assert all return `None`.

### Test 8: `new_generation_increments_age`
- Create table, assert `generation()` is 0.
- Call `new_generation()`.
- Assert `generation()` is 1.
- Store an entry with age = `tt.generation()` (which is 1).
- Probe the entry back.
- Assert the returned entry has `age() == 1`, confirming entries stored after `new_generation()` carry the new generation value.

### Test 9: `new_with_zero_mb_creates_minimum_table`
- Create a `TranspositionTable` with 0 MB.
- Assert the table has exactly 1 entry.
- Store and probe an entry to verify the minimum table is functional.

## 6. Implementation Order

1. **Implement `TranspositionTable` struct** with fields `entries: Vec<TtEntry>`, `mask: usize`, `generation: u8`.
2. **Implement `new(mb: usize)`** with power-of-two sizing logic and the `mb == 0` edge case.
3. **Implement `generation(&self) -> u8`** accessor.
4. **Implement `probe(hash: u64) -> Option<TtEntry>`** with index computation and key verification.
5. **Implement `store(hash: u64, entry: TtEntry)`** with replacement policy.
6. **Implement `clear()`** — fill entries with default, reset generation.
7. **Implement `resize(mb: usize)`** — recompute sizing, reallocate, clear.
8. **Implement `new_generation()`** — wrapping increment of generation counter.
9. **Add unit tests** (all 9 tests from the test plan above).

## 7. Verification Commands

```bash
cargo test -p chess-search
cargo clippy -p chess-search -- -D warnings
cargo fmt -p chess-search --check
```

## 8. Design Deviations

**Deviation 1: `generation()` accessor method**
- **Parent spec requires:** The task spec lists `probe`, `store`, `clear`, `resize`, and `new_generation` methods. It does not mention a `generation()` accessor.
- **Why a deviation is needed:** The task spec prescribes `store(hash: u64, entry: TtEntry)`, meaning the caller constructs the `TtEntry` before passing it to `store`. The `TtEntry` requires an `age` field that must match the table's current generation for the replacement policy to work correctly. Without a `generation()` accessor, the caller has no way to obtain the current generation value to set on the entry.
- **Alternative considered:** Change `store`'s signature to accept raw fields and construct the `TtEntry` internally. This would eliminate the need for the accessor but would deviate further from the task spec's prescribed `store(hash: u64, entry: TtEntry)` signature.
- **Decision:** Add `pub fn generation(&self) -> u8`. This is the minimal addition required to make the prescribed API usable.