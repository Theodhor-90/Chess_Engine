Good — there's no `Move::NONE` or `Default` for `Move`. The codebase uses `Option<Move>` consistently for representing the absence of a move (seen in `KillerTable`, `PvTable`, `negamax` return type). Now I have everything needed.

## 1. Deliverables

- **New file: `crates/search/src/tt.rs`** — Contains `BoundType` enum and `TtEntry` struct with constructor, accessors, and unit tests.
- **Modified file: `crates/search/src/lib.rs`** — Adds `pub mod tt;` module declaration to register the new module.

## 2. Dependencies

- `chess_types::Move` — the 16-bit packed move type, already a dependency of `chess-search` via `chess-types = { path = "../types" }`.
- No new crate dependencies required. Standard library only.

## 3. Implementation Details

### `crates/search/src/tt.rs`

#### `BoundType` enum

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BoundType {
    Exact = 0,
    LowerBound = 1,
    UpperBound = 2,
}
```

- Three variants: `Exact`, `LowerBound`, `UpperBound`.
- `#[repr(u8)]` for safe integer conversion and compact storage.
- Derive `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`.
- Implement `TryFrom<u8>` for safe conversion from integer (returns `None` or `Err` for invalid values). Following the project convention of avoiding `unwrap()` in library crates, this returns a `Result`.

```rust
impl TryFrom<u8> for BoundType {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(BoundType::Exact),
            1 => Ok(BoundType::LowerBound),
            2 => Ok(BoundType::UpperBound),
            _ => Err(()),
        }
    }
}
```

#### `TtEntry` struct

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TtEntry {
    key: u16,
    depth: u8,
    score: i32,
    bound: BoundType,
    best_move: Option<Move>,
    age: u8,
}
```

Fields:
- `key: u16` — Upper 16 bits of the Zobrist hash for collision filtering (the lower bits are used for table indexing). Extracted as `(hash >> 48) as u16`.
- `depth: u8` — Search depth at which this entry was computed.
- `score: i32` — Search score (matches the `i32` return type used throughout `negamax`).
- `bound: BoundType` — Whether the score is exact, a lower bound (fail-high), or an upper bound (fail-low).
- `best_move: Option<Move>` — The best move found at this node. Uses `Option<Move>` consistent with the codebase pattern (see `KillerTable`, `PvTable`, `negamax` return type).
- `age: u8` — Generation counter for replacement policy decisions.

Constructor:
```rust
impl TtEntry {
    pub fn new(
        key: u16,
        depth: u8,
        score: i32,
        bound: BoundType,
        best_move: Option<Move>,
        age: u8,
    ) -> TtEntry {
        TtEntry { key, depth, score, bound, best_move, age }
    }
}
```

Accessors (one method per field, all `&self -> T` where T is Copy):
```rust
pub fn key(&self) -> u16 { self.key }
pub fn depth(&self) -> u8 { self.depth }
pub fn score(&self) -> i32 { self.score }
pub fn bound(&self) -> BoundType { self.bound }
pub fn best_move(&self) -> Option<Move> { self.best_move }
pub fn age(&self) -> u8 { self.age }
```

Default/empty state for table initialization — implement `Default`:
```rust
impl Default for TtEntry {
    fn default() -> Self {
        TtEntry {
            key: 0,
            depth: 0,
            score: 0,
            bound: BoundType::Exact,
            best_move: None,
            age: 0,
        }
    }
}
```

This allows the TT table (T02) to initialize with `vec![TtEntry::default(); capacity]`.

#### Helper function for key extraction

```rust
pub fn verification_key(hash: u64) -> u16 {
    (hash >> 48) as u16
}
```

A free function that extracts the 16-bit verification key from a 64-bit Zobrist hash. This will be used by T02 when storing/probing.

### `crates/search/src/lib.rs`

Add one line to the module declarations at the top of the file:

```rust
pub mod tt;
```

This is added after the existing `pub mod pv_table;` line, following alphabetical order of the existing modules (`killer`, `ordering`, `pv_table`, `tt`).

## 4. API Contracts

```rust
use chess_search::tt::{BoundType, TtEntry, verification_key};
use chess_types::{Move, MoveFlag, Square};

// Create an entry
let mv = Move::new(Square::E2, Square::E4, MoveFlag::DOUBLE_PAWN_PUSH);
let entry = TtEntry::new(
    0xABCD,                // key verification bits
    12,                    // depth
    150,                   // score
    BoundType::Exact,      // bound type
    Some(mv),              // best move
    3,                     // age
);

// Read fields
assert_eq!(entry.key(), 0xABCD);
assert_eq!(entry.depth(), 12);
assert_eq!(entry.score(), 150);
assert_eq!(entry.bound(), BoundType::Exact);
assert_eq!(entry.best_move(), Some(mv));
assert_eq!(entry.age(), 3);

// Extract verification key from Zobrist hash
let hash: u64 = 0xABCD_1234_5678_9ABC;
let vkey = verification_key(hash);
assert_eq!(vkey, 0xABCD);

// Default entry for table initialization
let empty = TtEntry::default();
assert_eq!(empty.depth(), 0);
assert_eq!(empty.best_move(), None);

// BoundType integer conversion
let b = BoundType::try_from(1u8);
assert_eq!(b, Ok(BoundType::LowerBound));
let invalid = BoundType::try_from(5u8);
assert!(invalid.is_err());
```

N/A for REST/HTTP.

## 5. Test Plan

All tests are colocated in `crates/search/src/tt.rs` within a `#[cfg(test)] mod tests` block, following the project convention.

| Test | What it verifies |
|------|-----------------|
| `round_trip_all_fields` | Construct a `TtEntry` with known values for all fields, read back each via accessor, assert equality. Covers the core verification criterion. |
| `round_trip_no_move` | Construct a `TtEntry` with `best_move: None`, verify `best_move()` returns `None`. |
| `bound_type_exact` | Construct entry with `BoundType::Exact`, verify `bound()` returns `Exact`. |
| `bound_type_lower_bound` | Construct entry with `BoundType::LowerBound`, verify `bound()` returns `LowerBound`. |
| `bound_type_upper_bound` | Construct entry with `BoundType::UpperBound`, verify `bound()` returns `UpperBound`. |
| `bound_type_try_from_valid` | `BoundType::try_from(0)` → `Exact`, `try_from(1)` → `LowerBound`, `try_from(2)` → `UpperBound`. |
| `bound_type_try_from_invalid` | `BoundType::try_from(3)` and `try_from(255)` both return `Err(())`. |
| `default_entry` | `TtEntry::default()` has depth 0, score 0, `Exact` bound, `None` best move, age 0, key 0. |
| `verification_key_extracts_upper_bits` | `verification_key(0xABCD_0000_0000_0000)` returns `0xABCD`. Test with several hash values to confirm upper 16 bits are extracted. |

Test helper function (following the pattern from `killer.rs` and `pv_table.rs`):
```rust
fn quiet_move(from: u8, to: u8) -> Move {
    Move::new(
        Square::new(from).unwrap(),
        Square::new(to).unwrap(),
        MoveFlag::QUIET,
    )
}
```

## 6. Implementation Order

1. **`crates/search/src/tt.rs`** — Define `BoundType` enum with `TryFrom<u8>`, `TtEntry` struct with `Default`, constructor, accessors, and `verification_key` free function.
2. **`crates/search/src/tt.rs`** — Add `#[cfg(test)] mod tests` with all test cases.
3. **`crates/search/src/lib.rs`** — Add `pub mod tt;` to module declarations.

## 7. Verification Commands

```bash
cargo test -p chess-search
cargo clippy -p chess-search -- -D warnings
cargo fmt --check -p chess-search
```

## 8. Design Deviations

**Deviation: `Option<Move>` instead of raw `u16` for `best_move` field.**

- The phase spec states: "16-bit raw `Move` bits (the best move found at this node)."
- Storing raw `u16` would require a sentinel value (e.g., `0`) to represent "no move," but `Move(0)` is a valid move encoding (a1→a1 quiet), so there is no safe sentinel. Using `Option<Move>` is the idiomatic Rust approach and is consistent with every other move-storage type in the codebase (`KillerTable`, `PvTable`, `negamax` return type). Since `Move` is `Copy` and 2 bytes, `Option<Move>` is 4 bytes due to niche optimization not applying to newtypes over `u16` — this is an acceptable 2-byte overhead per entry for correctness and consistency.
- Alternative chosen: `best_move: Option<Move>` with `Option<Move>` accessors. This avoids an unsafe sentinel, matches codebase conventions, and provides type-safe "no move" semantics.