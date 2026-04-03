Important finding: `Bitboard` has no `Mul` implementation. The magic bitboard lookup formula `((occupancy & mask) * magic) >> shift` requires multiplication. I need to note this — the implementation will need to use `Bitboard::inner()` to get the `u64`, do the multiplication, and wrap back. Let me also check if there's a `wrapping_mul` concern or if we can just use the raw u64.

Now I have all the context I need to write the task plan. Let me produce it.

## 1. Deliverables

- **Create**: `crates/movegen/src/magic.rs` — Magic bitboard infrastructure: blocker mask generation, pre-computed magic numbers and shift values for all 64 squares (bishop and rook), attack table initialization, and `bishop_attacks(square, occupancy) -> Bitboard` and `rook_attacks(square, occupancy) -> Bitboard` lookup functions. Includes unit tests.
- **Modify**: `crates/movegen/src/lib.rs` — Add `pub mod magic;` declaration.

## 2. Dependencies

- `chess-types` (already a dependency of `movegen`) — provides `Bitboard`, `Square`.
- No new crate dependencies required.
- **Phase P01 complete** — relies on `movegen` crate structure and module conventions established by `knights.rs`, `king.rs`, `pawns.rs`.
- **Milestone M01 complete** — requires `Bitboard` with bitwise operations, bit iteration, `inner()`, `new()`, `pop_count()`, `is_empty()`; `Square` with `index()`, `file()`, `rank()`, `from_file_rank()`, `File`, `Rank` enums.

## 3. Implementation Details

### `crates/movegen/src/magic.rs`

This is the sole new file. It contains all magic bitboard infrastructure.

#### Imports

```rust
use chess_types::{Bitboard, Square, File, Rank};
```

#### Blocker mask generation

Two `const fn` functions that compute the relevant occupancy mask for a given square:

- `const fn bishop_blocker_mask(sq: usize) -> Bitboard` — Traces diagonal and anti-diagonal rays from `sq`, excluding edge squares (rank 0, rank 7, file A, file H). Returns a `Bitboard` of squares whose occupancy affects the bishop's attack set.
- `const fn rook_blocker_mask(sq: usize) -> Bitboard` — Traces rank and file rays from `sq`, excluding the edge squares at the ends of each ray. Returns a `Bitboard` of squares whose occupancy affects the rook's attack set.

Both use `while` loops over ray directions with bounds checking (no `for` loops, since these are `const fn`). Square coordinates computed as `file = sq % 8`, `rank = sq / 8`.

#### Pre-computed magic numbers and shift values

Two pairs of `const` arrays, 64 entries each:

- `const BISHOP_MAGICS: [u64; 64]` — Known-good magic numbers for bishops, one per square. Sourced from published chess programming community values (e.g., from the Chess Programming Wiki or Stockfish).
- `const BISHOP_SHIFTS: [u8; 64]` — Shift values for bishops. `shift = 64 - mask.pop_count()` for each square.
- `const ROOK_MAGICS: [u64; 64]` — Known-good magic numbers for rooks.
- `const ROOK_SHIFTS: [u8; 64]` — Shift values for rooks.

#### Attack set computation (for table building)

- `const fn bishop_attacks_slow(sq: usize, occupied: u64) -> Bitboard` — Traces the four diagonal rays from `sq`, stopping each ray when it hits an occupied square (but including that square in the attack set). This is the reference implementation used to build the lookup table.
- `const fn rook_attacks_slow(sq: usize, occupied: u64) -> Bitboard` — Same for the four orthogonal rays.

Both use `while` loops stepping along each ray direction: `(df, dr)` pairs of `(-1,-1), (-1,1), (1,-1), (1,1)` for bishop and `(-1,0), (1,0), (0,-1), (0,1)` for rook.

#### Attack table storage

Since magic bitboard tables cannot be fully const-initialized (the total table size is ~800KB and involves dynamic indexing), use `std::sync::OnceLock` for lazy one-time initialization:

```rust
use std::sync::OnceLock;

struct MagicTable {
    bishop_table: Vec<Bitboard>,
    bishop_offsets: [usize; 64],
    rook_table: Vec<Bitboard>,
    rook_offsets: [usize; 64],
}

static MAGIC_TABLE: OnceLock<MagicTable> = OnceLock::new();
```

- `bishop_offsets[sq]` — Starting index into `bishop_table` for square `sq`.
- Each square occupies `1 << (64 - BISHOP_SHIFTS[sq])` entries.
- Total bishop table size: sum of `1 << bishop_mask[sq].pop_count()` for all 64 squares (≈5248 entries).
- Total rook table size: sum of `1 << rook_mask[sq].pop_count()` for all 64 squares (≈102400 entries).

Initialization function `fn init_magic_table() -> MagicTable`:
1. For each square, compute the blocker mask.
2. Enumerate all subsets of the blocker mask (using Carry-Rippler: `subset = (subset - mask) & mask`).
3. For each subset (occupancy), compute the magic index: `((occupancy.wrapping_mul(magic)) >> shift) as usize`.
4. Store `bishop_attacks_slow(sq, occupancy)` / `rook_attacks_slow(sq, occupancy)` at the computed index.

#### Public lookup functions

```rust
pub fn bishop_attacks(square: Square, occupancy: Bitboard) -> Bitboard
```
- Get or initialize `MAGIC_TABLE`.
- Mask occupancy with the bishop blocker mask for this square.
- Compute index: `((masked.inner().wrapping_mul(BISHOP_MAGICS[sq])) >> BISHOP_SHIFTS[sq]) as usize`.
- Return `table.bishop_table[table.bishop_offsets[sq] + index]`.

```rust
pub fn rook_attacks(square: Square, occupancy: Bitboard) -> Bitboard
```
- Same pattern using rook magic numbers, shifts, and rook table.

#### Blocker mask precomputation arrays

Two `const` arrays used by the lookup functions to mask occupancy:

- `const BISHOP_MASKS: [Bitboard; 64]` — Computed via `const fn init_bishop_masks() -> [Bitboard; 64]` calling `bishop_blocker_mask` for each square.
- `const ROOK_MASKS: [Bitboard; 64]` — Computed via `const fn init_rook_masks() -> [Bitboard; 64]` calling `rook_blocker_mask` for each square.

#### Unit tests (`#[cfg(test)] mod tests`)

All tests follow the pattern established in `knights.rs`:

| Test | What it verifies |
|------|-----------------|
| `test_bishop_blocker_mask_center` | Bishop mask for a center square (e.g., D4) has the correct number of bits and excludes edges |
| `test_bishop_blocker_mask_corner` | Bishop mask for A1 has the correct relevant bits (6 bits on the a1-h8 diagonal, excluding h8) |
| `test_rook_blocker_mask_center` | Rook mask for D4 has 10 bits (3 on file excluding edges + 3 on rank excluding edges × 2 - overlaps) |
| `test_rook_blocker_mask_corner` | Rook mask for A1 has 12 bits |
| `test_bishop_attacks_empty_board` | `bishop_attacks(D4, empty)` returns all diagonal squares from D4 |
| `test_bishop_attacks_with_blockers` | `bishop_attacks(D4, occupied)` stops at the first occupied square on each ray (including that square) |
| `test_rook_attacks_empty_board` | `rook_attacks(D4, empty)` returns all orthogonal squares from D4 |
| `test_rook_attacks_with_blockers` | `rook_attacks(D4, occupied)` stops at first blocker on each ray |
| `test_bishop_attacks_all_squares` | For all 64 squares on an empty board, `bishop_attacks` matches `bishop_attacks_slow` |
| `test_rook_attacks_all_squares` | For all 64 squares on an empty board, `rook_attacks` matches `rook_attacks_slow` |

### `crates/movegen/src/lib.rs`

Add one line:
```rust
pub mod magic;
```

No new re-exports from `lib.rs` for T01. The `bishop_attacks` and `rook_attacks` functions are accessed as `magic::bishop_attacks` and `magic::rook_attacks` by downstream code (T02).

## 4. API Contracts

```rust
use chess_types::{Bitboard, Square};
use chess_movegen::magic::{bishop_attacks, rook_attacks};

// Get bishop attack bitboard for square D4 with given board occupancy
let attacks: Bitboard = bishop_attacks(Square::D4, occupancy);

// Get rook attack bitboard for square A1 with given board occupancy
let attacks: Bitboard = rook_attacks(Square::A1, occupancy);
```

N/A for REST/HTTP.

## 5. Test Plan

Tests are colocated in `crates/movegen/src/magic.rs` under `#[cfg(test)] mod tests`, following the convention from `knights.rs`.

| Test | What it verifies |
|------|-----------------|
| `test_bishop_blocker_mask_center` | `bishop_blocker_mask(D4)` has correct bit count (9 relevant bits) and excludes edge squares |
| `test_bishop_blocker_mask_corner` | `bishop_blocker_mask(A1)` has correct bit count (6 diagonal bits, excluding H8) |
| `test_rook_blocker_mask_center` | `rook_blocker_mask(D4)` has correct bit count (10 relevant bits) and excludes edge squares |
| `test_rook_blocker_mask_corner` | `rook_blocker_mask(A1)` has correct bit count (12 bits: 6 on file + 6 on rank) |
| `test_bishop_attacks_empty_board` | `bishop_attacks(D4, EMPTY)` returns all diagonally reachable squares (13 squares) |
| `test_bishop_attacks_with_blockers` | `bishop_attacks(D4, blockers)` correctly stops at first blocker on each ray (including blocker square) |
| `test_rook_attacks_empty_board` | `rook_attacks(D4, EMPTY)` returns all orthogonally reachable squares (14 squares) |
| `test_rook_attacks_with_blockers` | `rook_attacks(D4, blockers)` correctly stops at first blocker on each ray |
| `test_bishop_attacks_all_squares` | For all 64 squares with empty occupancy, `bishop_attacks` matches the slow reference implementation |
| `test_rook_attacks_all_squares` | For all 64 squares with empty occupancy, `rook_attacks` matches the slow reference implementation |

## 6. Implementation Order

1. **Blocker mask functions** — `bishop_blocker_mask`, `rook_blocker_mask`, and their precomputed arrays `BISHOP_MASKS`, `ROOK_MASKS`.
2. **Slow reference attack functions** — `bishop_attacks_slow`, `rook_attacks_slow` (needed for table building and test validation).
3. **Magic number and shift constants** — `BISHOP_MAGICS`, `BISHOP_SHIFTS`, `ROOK_MAGICS`, `ROOK_SHIFTS` (hardcoded known-good values).
4. **Attack table initialization** — `MagicTable` struct, `init_magic_table()` function, `MAGIC_TABLE` static.
5. **Public lookup functions** — `bishop_attacks`, `rook_attacks`.
6. **Update `lib.rs`** — Add `pub mod magic;`.
7. **Unit tests** — All tests listed in the test plan.

## 7. Verification Commands

```bash
cargo test -p chess-movegen
cargo clippy -p chess-movegen -- -D warnings
cargo fmt --check -p chess-movegen
```

## 8. Design Deviations

**Deviation: Runtime initialization via `OnceLock` instead of `const` initialization**

- **What the task spec requires**: Design constraint #4 says "Const-initialization pattern: follow the same pattern established by king and knight attack tables in Phase P01."
- **Why that approach is problematic**: King and knight attack tables are small fixed-size arrays (`[Bitboard; 64]`) that can be fully computed at compile time. Magic bitboard tables are fundamentally different — they require variable-sized per-square sub-tables (bishop: ~5,248 entries total; rook: ~102,400 entries total) with dynamic indexing based on magic hash values. Rust `const fn` evaluation cannot allocate `Vec`s or dynamically-sized structures, and a flat `const` array of ~107,648 `Bitboard` entries with correct per-square offset computation is impractical in a `const` context due to the Carry-Rippler enumeration and magic-indexed writes. Attempting to force this into a `const` array would require a single monolithic `const fn` with deeply nested loops that exceeds const-eval step limits.
- **Alternative chosen**: Use `std::sync::OnceLock<MagicTable>` for lazy one-time initialization on first access. This is safe, thread-safe, zero-cost after initialization, and idiomatic Rust. The blocker masks, magic numbers, and shift values remain `const` — only the attack lookup tables are lazily initialized. This preserves the spirit of the pattern (deterministic, computed-once tables) while accommodating the data structure requirements of magic bitboards.