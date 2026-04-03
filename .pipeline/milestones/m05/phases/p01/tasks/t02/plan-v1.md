I now have a thorough understanding of all the relevant code. Let me produce the implementation plan.

## 1. Deliverables

| # | File | Action | Purpose |
|---|------|--------|---------|
| 1 | `crates/search/src/tt.rs` | Modify | Add `TranspositionTable` struct with `probe`, `store`, `clear`, `resize`, `new_generation` methods and unit tests |

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
- Compute the number of bytes available: `mb * 1024 * 1024`
- Compute entry size: `std::mem::size_of::<TtEntry>()`
- Compute `num_entries = bytes / entry_size`
- Round down to the nearest power of two: `num_entries.next_power_of_two() / 2` if `num_entries` is not already a power of two, otherwise use it directly. Use `num_entries.checked_next_power_of_two()` and handle appropriately — or simply use bit manipulation: `1 << (usize::BITS - num_entries.leading_zeros() - 1)` (which gives the largest power of two ≤ `num_entries`).
- If `mb == 0` or the computed count is 0, default to a minimum size of 1 entry (to avoid division by zero / empty vec issues).
- Allocate `Vec` of that length filled with `TtEntry::default()`.
- Set `mask = num_entries - 1`.
- Set `generation = 0`.

#### `pub fn probe(&self, hash: u64) -> Option<TtEntry>`
- Compute index: `(hash as usize) & self.mask`
- Read `self.entries[index]`
- Compute `verification_key(hash)` (the upper 16 bits)
- If the entry's `key()` matches the verification key AND the entry is not a default/empty entry (key != 0 or depth != 0 — actually, the simplest check: compare the stored key to the verification key; a default entry has key=0, so it only matches hash values whose upper 16 bits are also 0, which is rare and harmless), return `Some(entry)`
- Otherwise return `None`

**Key detail on empty entry detection:** A default `TtEntry` has `key = 0`. The `verification_key` of a hash could also be 0 (`hash` with upper 16 bits = 0). To handle this cleanly, the probe checks `entry.key() == verification_key(hash)` and additionally checks that the entry is not entirely default (i.e., has been written). The simplest reliable approach: store entries with their verification key as-is, and accept that a hash with upper bits `0x0000` might false-match an empty slot (returning a depth-0, score-0 entry — which is harmless since a depth-0 entry won't cause cutoffs at any meaningful search depth). This is standard practice in chess engine TTs.

#### `pub fn store(&self, hash: u64, entry: TtEntry)` → actually `pub fn store(&mut self, hash: u64, depth: u8, score: i32, bound: BoundType, best_move: Option<Move>)`

**Revised design:** The `store` method should accept raw fields and construct the `TtEntry` internally, setting the key from `verification_key(hash)` and the age from `self.generation`. This is cleaner than requiring the caller to pre-construct an entry with the correct key and age.

**Signature:** `pub fn store(&mut self, hash: u64, depth: u8, score: i32, bound: BoundType, best_move: Option<Move>)`

- Compute index: `(hash as usize) & self.mask`
- Read existing entry at that index
- Construct new entry: `TtEntry::new(verification_key(hash), depth, score, bound, best_move, self.generation)`
- Apply replacement policy:
  - If existing entry's `age != self.generation` (stale) → replace
  - Else if `depth >= existing.depth()` → replace (equal or greater depth preferred)
  - Else → keep existing entry (do not replace)
- Write the new entry if replacing

**Alternative:** Keep the task spec's signature `store(hash: u64, entry: TtEntry)` and let the caller construct the entry with the correct key and age. However, this pushes the responsibility of setting key/age to the caller. Looking at the task spec, it says `store(hash: u64, entry: TtEntry)`. I will follow the task spec signature exactly. The caller is responsible for constructing the `TtEntry` with the correct `verification_key(hash)` and the current generation age. The `store` method only applies the replacement policy.

**Final signature (per task spec):** `pub fn store(&mut self, hash: u64, entry: TtEntry)`

- Compute index: `(hash as usize) & self.mask`
- Read existing entry
- Replacement policy:
  - If existing `age() != self.generation` → replace (stale)
  - Else if `entry.depth() >= existing.depth()` → replace (deeper or equal)
  - Else → do not replace

#### `pub fn clear(&mut self)`
- Fill all entries with `TtEntry::default()`
- Reset `generation` to 0

#### `pub fn resize(&mut self, mb: usize)`
- Recompute the power-of-two entry count for the new size (same logic as `new`)
- Allocate a new `Vec<TtEntry>` of that size filled with defaults
- Update `self.entries`, `self.mask`
- Reset `generation` to 0

#### `pub fn new_generation(&mut self)`
- Increment `self.generation` with wrapping: `self.generation = self.generation.wrapping_add(1)`

**Exports:** `TranspositionTable` is `pub` so it can be used from `lib.rs` in T03.

## 4. API Contracts

### `TranspositionTable::new(mb: usize) -> TranspositionTable`
- **Input:** Size in megabytes (e.g., `16`)
- **Output:** A `TranspositionTable` with the largest power-of-two entry count fitting in that many megabytes

### `TranspositionTable::probe(hash: u64) -> Option<TtEntry>`
- **Input:** 64-bit Zobrist hash
- **Output:** `Some(entry)` if key verification matches, `None` otherwise
- **Example:** `tt.probe(0xABCD_0000_0000_0001)` returns `Some(entry)` if an entry was stored for a hash whose upper 16 bits are `0xABCD` at index `1 & mask`

### `TranspositionTable::store(hash: u64, entry: TtEntry)`
- **Input:** 64-bit Zobrist hash, a `TtEntry` (caller sets key via `verification_key(hash)` and age to current generation)
- **Output:** None (entry stored if replacement policy allows)

### `TranspositionTable::clear()`
- **Input:** None
- **Output:** None (all entries zeroed, generation reset)

### `TranspositionTable::resize(mb: usize)`
- **Input:** New size in megabytes
- **Output:** None (table reallocated and cleared)

### `TranspositionTable::new_generation()`
- **Input:** None
- **Output:** None (internal generation counter incremented with wrapping)

### `TranspositionTable::generation() -> u8`
- **Input:** None
- **Output:** Current generation value (needed by callers to set the age field on entries they construct)

## 5. Test Plan

All tests are added to the existing `#[cfg(test)] mod tests` block in `crates/search/src/tt.rs`.

### Test 1: `store_and_probe_round_trip`
- Create a `TranspositionTable` with 1 MB
- Store an entry with a known hash (using `verification_key(hash)` for the key, current generation for age)
- Probe with the same hash
- Assert `Some(entry)` with all fields matching

### Test 2: `probe_returns_none_for_missing`
- Create a table, store an entry at one hash
- Probe with a different hash (different index AND different key)
- Assert `None`

### Test 3: `replacement_deeper_does_not_replace_shallower_same_age`
- Store entry A with depth 10 at some hash
- Store entry B with depth 5 at the same hash index (same lower bits) but same key
- Probe and assert entry A is still present (shallow does not replace deeper, same age)

### Test 4: `replacement_stale_entry_replaced_by_shallower`
- Store entry A with depth 10 at generation 0
- Call `new_generation()`
- Store entry B with depth 3 at the same hash (now generation 1)
- Probe and assert entry B is present (stale entry replaced regardless of depth)

### Test 5: `replacement_equal_depth_same_age_replaces`
- Store entry A with depth 5
- Store entry B with depth 5 at the same hash (same generation)
- Probe and assert entry B is present (equal depth replaces)

### Test 6: `power_of_two_sizing`
- Create a table with 16 MB
- Assert entry count is a power of two
- Assert total memory (`entry_count * size_of::<TtEntry>()`) ≤ 16 MB

### Test 7: `clear_makes_all_probes_none`
- Store several entries
- Call `clear()`
- Probe all previously stored hashes
- Assert all return `None`

### Test 8: `new_generation_increments_age`
- Create table, assert `generation()` is 0
- Call `new_generation()`
- Assert `generation()` is 1

## 6. Implementation Order

1. **Add the `generation()` accessor method** to expose the current generation for callers constructing `TtEntry` values.
2. **Implement `TranspositionTable` struct** with fields `entries: Vec<TtEntry>`, `mask: usize`, `generation: u8`.
3. **Implement `new(mb: usize)`** with power-of-two sizing logic.
4. **Implement `probe(hash: u64) -> Option<TtEntry>`** with index computation and key verification.
5. **Implement `store(hash: u64, entry: TtEntry)`** with replacement policy.
6. **Implement `clear()`** — fill entries with default, reset generation.
7. **Implement `resize(mb: usize)`** — recompute sizing, reallocate, clear.
8. **Implement `new_generation()`** — wrapping increment of generation counter.
9. **Add unit tests** (all 8 tests from the test plan above).

## 7. Verification Commands

```bash
cargo test -p chess-search
cargo clippy -p chess-search -- -D warnings
cargo fmt -p chess-search --check
```

## 8. Design Deviations

**Deviation 1: `generation()` accessor method**
- **Parent spec requires:** The task spec lists `probe`, `store`, `clear`, `resize`, and `new_generation` methods. It does not mention a `generation()` accessor.
- **Why a deviation is needed:** Callers of `store` need to construct a `TtEntry` with the correct `age` field set to the table's current generation. Without a `generation()` accessor, there is no way for the caller to know what age value to use when constructing the `TtEntry` they pass to `store`.
- **Alternative chosen:** Add a `pub fn generation(&self) -> u8` accessor. This is the minimal addition needed to make the API usable. The alternative would be to change `store`'s signature to accept raw fields instead of a `TtEntry`, but that would deviate further from the task spec's prescribed `store(hash: u64, entry: TtEntry)` signature.