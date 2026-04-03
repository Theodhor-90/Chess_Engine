# Task T01 — Magic Bitboard Infrastructure

## Objective

Implement the magic bitboard lookup infrastructure that enables O(1) sliding piece attack generation for bishops and rooks. This is the foundational layer upon which all sliding piece move generation (T02, T03) depends.

## Deliverables

- A `crates/movegen/src/magic.rs` module containing:
  - **Blocker mask generation** for bishops and rooks: compute the relevant occupancy bits along rays for each square, excluding edge squares.
  - **Pre-computed magic numbers and shift values** for all 64 squares for both bishops and rooks, defined as compile-time constants. Use known-good published values from the chess programming community (no runtime discovery).
  - **Attack table initialization**: build the mapping from each `(square, occupancy)` pair to the correct attack bitboard.
  - **Lookup functions**: `bishop_attacks(square, occupancy) -> Bitboard` and `rook_attacks(square, occupancy) -> Bitboard`.
  - **Unit tests** verifying correct attack sets for known square/occupancy combinations.

## Design Constraints

1. **Plain (fixed-shift) magic bitboards**: each square gets a magic number and a fixed shift value. The lookup formula is `((occupancy & mask) * magic) >> shift`.
2. **Separate tables for bishops and rooks**: bishop and rook each get their own attack table arrays. No separate queen tables (queen is handled in T03 as union of bishop + rook).
3. **Separation of attack lookup from move generation**: attack lookup functions return a `Bitboard` of attacked squares. Move generation (T02) layers on top by masking out friendly pieces and converting to `Move` values.
4. **Const-initialization pattern**: follow the same pattern established by king and knight attack tables in Phase P01.
5. **No `unsafe` unless provably necessary** — if used, must have a `// SAFETY:` comment.
6. **No `unwrap()` in library code** — use `Result` or `Option` propagation.

## Dependencies

- **Phase P01 (Non-Sliding Pieces) complete** — relies on the `movegen` crate structure, module conventions, and attack table patterns from P01.
- **Milestone M01 complete** — requires `Bitboard` with bitwise operations and bit manipulation, `Square` for indexing.

## Files to Create or Modify

- **Create**: `crates/movegen/src/magic.rs`
- **Modify**: `crates/movegen/src/lib.rs` — add `mod magic;` declaration

## Verification Criteria

1. `bishop_attacks(square, occupancy)` returns correct attack bitboards for all 64 squares across all relevant occupancy configurations.
2. `rook_attacks(square, occupancy)` returns correct attack bitboards for all 64 squares across all relevant occupancy configurations.
3. Unit tests pass covering known square/occupancy combinations for both piece types.
4. `cargo test -p movegen` passes with no failures.
5. `cargo clippy -p movegen -- -D warnings` passes.
6. `cargo fmt --check -p movegen` passes.