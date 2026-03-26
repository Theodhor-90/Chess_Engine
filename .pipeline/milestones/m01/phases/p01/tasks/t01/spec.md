# Task T01 — Bitboard

## Objective

Implement the `Bitboard` newtype wrapper `struct Bitboard(u64)` in the `crates/types/` crate. This is the foundational bit-manipulation type that the entire engine operates on.

## Deliverables

- `Bitboard` struct as a newtype around `u64`
- Operator overloads: `BitAnd`, `BitOr`, `BitXor`, `Not`, `Shl`, `Shr` (and their `Assign` variants where appropriate)
- Population count method — returns the number of set bits
- LSB extraction — returns the index of the least significant set bit
- MSB extraction — returns the index of the most significant set bit
- Bit iteration — an iterator yielding the index of each set bit in ascending order
- `Display` or `Debug` formatting useful for debugging (e.g., 8x8 grid representation)

## Files to Create or Modify

- Create `crates/types/src/bitboard.rs` (or equivalent module file)
- Create or modify `crates/types/src/lib.rs` to export the `Bitboard` type
- Create `crates/types/Cargo.toml` with crate metadata and `thiserror` dependency
- Create or modify workspace root `Cargo.toml` to include `types` as a workspace member

## Key Implementation Details

- **Newtype, not alias**: `struct Bitboard(u64)` provides type safety — prevents accidental mixing of raw `u64` with board representations
- **No external dependencies** for the core logic — only the Rust standard library (plus `thiserror` for error types)
- LSB/MSB extraction should use efficient intrinsics (`u64::trailing_zeros()`, `u64::leading_zeros()`)
- Population count should use `u64::count_ones()`
- Bit iteration should destructively pop LSBs for efficiency
- No `unwrap()` in library code — use `Result` or `Option` propagation
- Follow project naming conventions: `snake_case` functions, `PascalCase` types, `SCREAMING_SNAKE` constants
- `///` doc comments on all public items; only explain *why*, not *what*

## Dependencies

- None — this is the first task of the first phase of the first milestone

## Verification Criteria

1. Unit tests cover operator behavior for all overloaded operators (`&`, `|`, `^`, `!`, `<<`, `>>`)
2. Unit tests cover population count for empty bitboard (0), single bit, multiple bits, all bits set
3. Unit tests cover LSB/MSB extraction including edge cases (single bit at position 0, single bit at position 63)
4. Unit tests cover bit iteration: empty bitboard yields no elements, single bit yields one element, multiple bits yield correct indices in ascending order, all 64 bits set yields 0..=63
5. `cargo fmt --check` passes
6. `cargo clippy -- -D warnings` passes
7. `cargo test` passes with all unit tests green