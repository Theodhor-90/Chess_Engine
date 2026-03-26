- **`Cargo.toml`** (created) — workspace root with `resolver = "2"`, `members = ["crates/types"]`, workspace edition `"2021"`
- **`crates/types/Cargo.toml`** (created) — crate manifest for `chess-types` v0.1.0 with `thiserror = "2"` dependency
- **`crates/types/src/lib.rs`** (created) — declares `pub mod bitboard` and re-exports `Bitboard`
- **`crates/types/src/bitboard.rs`** (created) — `Bitboard(u64)` newtype with:
  - Constants: `EMPTY`, `FULL`
  - Constructor/access: `new()`, `inner()`
  - Bit manipulation: `pop_count()`, `lsb()`, `msb()`, `is_empty()`
  - Operator overloads: `BitAnd`, `BitOr`, `BitXor`, `Not`, `Shl<u32>`, `Shr<u32>` + all `Assign` variants
  - `IntoIterator` via `BitboardIter` (LSB-popping iteration)
  - `Display` impl (8×8 grid, rank 8 top)
  - 21 unit tests (all passing)

Verification: `cargo fmt --check` clean, `cargo clippy -- -D warnings` clean, `cargo test -p chess-types` — 21/21 pass.