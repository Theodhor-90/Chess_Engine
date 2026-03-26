## 1. Deliverables

- **`Cargo.toml`** (workspace root) — Cargo workspace configuration declaring `crates/types` as a member
- **`crates/types/Cargo.toml`** — Crate manifest for the `types` crate with `thiserror` dependency
- **`crates/types/src/lib.rs`** — Crate root that declares the `bitboard` module and re-exports `Bitboard`
- **`crates/types/src/bitboard.rs`** — `Bitboard` newtype implementation with operator overloads, bit-manipulation methods, iterator, formatting, and colocated unit tests

## 2. Dependencies

- Rust stable toolchain (2021 edition) — must be installed on the system
- `thiserror` crate — declared as a dependency in `crates/types/Cargo.toml` per the phase spec (used by the `types` crate for structured error types; not directly used by this task's Bitboard code, but the phase spec requires it in the crate manifest)
- No other external crates required

## 3. Implementation Details

### `Cargo.toml` (workspace root)
- Define `[workspace]` with `resolver = "2"`
- Set `members = ["crates/types"]`
- Set workspace-level `edition = "2021"` under `[workspace.package]`

### `crates/types/Cargo.toml`
- `[package]` with `name = "chess-types"`, `version = "0.1.0"`, `edition.workspace = true`
- `[dependencies]` with `thiserror = "2"` (per phase spec: "crates/types/Cargo.toml with crate metadata and thiserror dependency")

### `crates/types/src/lib.rs`
- Declare `pub mod bitboard;`
- Re-export: `pub use bitboard::Bitboard;`

### `crates/types/src/bitboard.rs`

**Struct definition:**
```rust
/// Bitboard representation — a set of squares encoded as a 64-bit integer.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Bitboard(u64);
```

**Constants:**
- `Bitboard::EMPTY` — `Bitboard(0)` (no bits set)
- `Bitboard::FULL` — `Bitboard(u64::MAX)` (all 64 bits set)

**Constructor and raw access:**
- `Bitboard::new(val: u64) -> Self` — wraps a raw `u64`
- `Bitboard::inner(self) -> u64` — returns the wrapped `u64`

**Bit-manipulation methods:**
- `pop_count(self) -> u32` — delegates to `self.0.count_ones()`
- `lsb(self) -> Option<u32>` — returns `Some(self.0.trailing_zeros())` when non-empty, `None` when empty
- `msb(self) -> Option<u32>` — returns `Some(63 - self.0.leading_zeros())` when non-empty, `None` when empty
- `is_empty(self) -> bool` — `self.0 == 0`

**Operator overloads (implementing the standard `std::ops` traits):**

| Trait | RHS | Description |
|-------|-----|-------------|
| `BitAnd<Bitboard>` | `Bitboard` | Intersection of two bitboards |
| `BitOr<Bitboard>` | `Bitboard` | Union of two bitboards |
| `BitXor<Bitboard>` | `Bitboard` | Symmetric difference |
| `Not` | — | Complement (flip all bits) |
| `Shl<u32>` | `u32` | Left shift by `u32` positions |
| `Shr<u32>` | `u32` | Right shift by `u32` positions |
| `BitAndAssign<Bitboard>` | `Bitboard` | In-place intersection |
| `BitOrAssign<Bitboard>` | `Bitboard` | In-place union |
| `BitXorAssign<Bitboard>` | `Bitboard` | In-place symmetric difference |
| `ShlAssign<u32>` | `u32` | In-place left shift |
| `ShrAssign<u32>` | `u32` | In-place right shift |

Each `Output` type is `Bitboard`.

**Bit iteration:**
- Implement `IntoIterator for Bitboard` returning a `BitboardIter` struct
- `BitboardIter` holds a `Bitboard` and implements `Iterator<Item = u32>`
- `next()` pops the LSB via `trailing_zeros()`, clears it with `self.0 &= self.0 - 1`, returns the index
- Returns `None` when the inner value is `0`

**Display formatting:**
- Implement `fmt::Display` for `Bitboard` — renders an 8×8 grid with `1` for set bits and `.` for unset bits, rank 8 at the top and rank 1 at the bottom (standard board orientation). The task spec says "Display or Debug formatting useful for debugging" — `Display` is chosen because an 8×8 grid is the most useful debugging representation for a bitboard, and the derived `Debug` would be insufficient (it would just show the struct wrapper). The automatically derived `Debug` is not used; no manual `Debug` impl is provided.

**Unit tests (`#[cfg(test)] mod tests`):**

| Test function | What it verifies |
|--------------|-----------------|
| `test_bitand` | `Bitboard(0xFF) & Bitboard(0x0F)` == `Bitboard(0x0F)` |
| `test_bitor` | `Bitboard(0xF0) \| Bitboard(0x0F)` == `Bitboard(0xFF)` |
| `test_bitxor` | `Bitboard(0xFF) ^ Bitboard(0x0F)` == `Bitboard(0xF0)` |
| `test_not` | `!Bitboard(0)` == `Bitboard(u64::MAX)` |
| `test_shl` | `Bitboard(1) << 8` == `Bitboard(256)` |
| `test_shr` | `Bitboard(256) >> 8` == `Bitboard(1)` |
| `test_assign_ops` | Verifies `&=`, `\|=`, `^=`, `<<=`, `>>=` mutate correctly |
| `test_pop_count_empty` | `Bitboard::EMPTY.pop_count()` == 0 |
| `test_pop_count_single` | `Bitboard(1).pop_count()` == 1 |
| `test_pop_count_multiple` | `Bitboard(0b1010_1010).pop_count()` == 4 |
| `test_pop_count_full` | `Bitboard::FULL.pop_count()` == 64 |
| `test_lsb_empty` | `Bitboard::EMPTY.lsb()` == `None` |
| `test_lsb_bit0` | `Bitboard(1).lsb()` == `Some(0)` |
| `test_lsb_bit63` | `Bitboard(1u64 << 63).lsb()` == `Some(63)` |
| `test_msb_empty` | `Bitboard::EMPTY.msb()` == `None` |
| `test_msb_bit63` | `Bitboard(1u64 << 63).msb()` == `Some(63)` |
| `test_msb_bit0` | `Bitboard(1).msb()` == `Some(0)` |
| `test_iter_empty` | Iterating `Bitboard::EMPTY` yields no elements |
| `test_iter_single` | Iterating `Bitboard(1 << 5)` yields `[5]` |
| `test_iter_multiple` | Iterating `Bitboard(0b1001_0010)` yields `[1, 4, 7]` in ascending order |
| `test_iter_full` | Iterating `Bitboard::FULL` yields all indices 0..=63 |

## 4. API Contracts

```rust
use chess_types::Bitboard;

// Construction
let bb = Bitboard::new(0xFF00);
let empty = Bitboard::EMPTY;
let full = Bitboard::FULL;

// Raw access
let raw: u64 = bb.inner();

// Bit manipulation
let count: u32 = bb.pop_count();
let lsb: Option<u32> = bb.lsb();
let msb: Option<u32> = bb.msb();
let is_zero: bool = bb.is_empty();

// Operators
let intersection = bb & Bitboard::new(0x00FF);
let union = bb | Bitboard::new(0x00FF);
let xor = bb ^ Bitboard::new(0x00FF);
let complement = !bb;
let shifted_left = bb << 8;
let shifted_right = bb >> 8;

// Iteration
for square_index in bb {
    // square_index: u32, indices of set bits in ascending order
}
```

## 5. Test Plan

All tests are colocated unit tests in `crates/types/src/bitboard.rs` under `#[cfg(test)] mod tests`.

**Test setup:** No special setup required. Standard `cargo test` execution.

**Per-test specification** (see the table in Section 3 under "Unit tests"). Tests are organized into four groups matching the verification criteria:

1. **Operator behavior** (`test_bitand`, `test_bitor`, `test_bitxor`, `test_not`, `test_shl`, `test_shr`, `test_assign_ops`) — verifies all six operator overloads and their assign variants produce correct results.

2. **Population count** (`test_pop_count_empty`, `test_pop_count_single`, `test_pop_count_multiple`, `test_pop_count_full`) — covers empty bitboard (0), single bit, multiple bits, and all bits set.

3. **LSB/MSB extraction** (`test_lsb_empty`, `test_lsb_bit0`, `test_lsb_bit63`, `test_msb_empty`, `test_msb_bit63`, `test_msb_bit0`) — covers empty bitboard returning `None`, single bit at position 0, and single bit at position 63.

4. **Bit iteration** (`test_iter_empty`, `test_iter_single`, `test_iter_multiple`, `test_iter_full`) — covers empty bitboard yielding no elements, single bit yielding one element, multiple bits yielding correct indices in ascending order, and all 64 bits set yielding 0..=63.

## 6. Implementation Order

1. **`Cargo.toml`** (workspace root) — set up the Cargo workspace with `crates/types` as a member
2. **`crates/types/Cargo.toml`** — create crate manifest with metadata and `thiserror` dependency
3. **`crates/types/src/bitboard.rs`** — implement `Bitboard` struct, constants, constructor/access methods, bit-manipulation methods, operator overloads, assign operator overloads, `BitboardIter` and `IntoIterator` impl, `Display` impl, and all unit tests
4. **`crates/types/src/lib.rs`** — declare the `bitboard` module and re-export `Bitboard`
5. **Verification** — run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`

## 7. Verification Commands

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test -p chess-types
```

## 8. Design Deviations

None.