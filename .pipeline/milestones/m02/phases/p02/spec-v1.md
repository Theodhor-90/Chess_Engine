# Phase P02 — Sliding Pieces & Magic Bitboards

## Goal

Implement magic bitboard infrastructure and use it for O(1) sliding piece attack generation, delivering bishop, rook, and queen move generation functions that integrate with the existing `movegen` crate and produce correct pseudo-legal moves for all board positions.

## Design Decisions

1. **Pre-computed magic numbers**: Use known-good magic numbers (published values from the chess programming community) as compile-time constants rather than discovering them at runtime. This eliminates startup cost, keeps initialization deterministic, and follows the same const-initialization pattern established by king and knight attack tables in P01.

2. **Fancy vs plain magics**: Use "plain" (fixed-shift) magic bitboards. Each square gets a magic number and a fixed shift value. The lookup formula is `((occupancy & mask) * magic) >> shift`. This is simpler to implement and debug than "fancy" (variable-shift) magics, and the memory overhead difference is negligible for engine strength.

3. **Shared attack table structure**: Bishop and rook each get their own attack table arrays. Queen attack generation is computed as the union of bishop and rook attacks for the same square and occupancy — no separate queen tables are stored. This halves the table memory and avoids redundancy.

4. **Separation of attack lookup from move generation**: Attack lookup functions return a `Bitboard` of attacked squares. Move generation functions layer on top by masking out friendly pieces and converting to `Move` values. This separation allows `is_square_attacked` and future search code to use the attack lookup without constructing move lists.

5. **Module organization**: Add `magic.rs` for magic bitboard infrastructure (masks, magics, table init), `bishops.rs` for bishop move generation, `rooks.rs` for rook move generation, and `queens.rs` for queen move generation — all within the existing `movegen` crate. Re-export generation functions from `lib.rs` following the pattern established in P01.

## Tasks

- **T01 — Magic bitboard infrastructure**: Implement blocker mask generation for bishops and rooks (relevant occupancy bits along rays, excluding edge squares), define pre-computed magic numbers and shift values for all 64 squares for both piece types, and build the attack table initialization that maps each (square, occupancy) pair to the correct attack bitboard. Deliverable: `magic.rs` module with `bishop_attacks(square, occupancy) -> Bitboard` and `rook_attacks(square, occupancy) -> Bitboard` lookup functions, plus unit tests verifying correct attack sets for known square/occupancy combinations.

- **T02 — Bishop and rook move generation**: Implement `generate_bishop_moves` and `generate_rook_moves` functions that use the magic bitboard lookup from T01, mask out friendly piece occupancy, and produce pseudo-legal `Move` values appended to a move list. Deliverable: `bishops.rs` and `rooks.rs` modules with generation functions matching the existing `fn(pos: &Position, moves: &mut Vec<Move>)` signature, plus unit tests verifying correct move generation for representative positions.

- **T03 — Queen move generation and combined sliding piece interface**: Implement `generate_queen_moves` as the union of bishop and rook attacks for the queen's square. Provide a combined `generate_sliding_moves` function (or equivalent) that generates all sliding piece moves (bishops, rooks, queens) in a single call. Re-export all new generation functions from `lib.rs`. Deliverable: `queens.rs` module, updated `lib.rs` exports, and unit tests verifying queen move generation for positions with multiple sliding pieces.

## Exit Criteria

1. `bishop_attacks(square, occupancy)` and `rook_attacks(square, occupancy)` return correct attack bitboards for all 64 squares across all relevant occupancy configurations.
2. `generate_bishop_moves`, `generate_rook_moves`, and `generate_queen_moves` produce correct pseudo-legal moves that include all valid destination squares and exclude squares occupied by friendly pieces.
3. Queen move generation equals the union of bishop and rook attacks for every position.
4. All new modules compile with `cargo clippy -- -D warnings` and pass `cargo test`.

## Dependencies

- **Phase P01 (Non-Sliding Pieces) complete** — P02 builds on the `movegen` crate structure, module conventions, and attack table patterns established in P01 (king and knight pre-computed attack tables, move generation function signature, `MoveFlag` usage).
- **Milestone M01 complete** — requires `Bitboard` with bitwise operations and bit manipulation, `Square` for indexing, `Move` encoding with flags, and `Position` for piece and occupancy bitboards.

## Artifacts

- `crates/movegen/src/magic.rs` — magic bitboard infrastructure (blocker masks, magic numbers, attack table initialization, attack lookup functions)
- `crates/movegen/src/bishops.rs` — bishop move generation
- `crates/movegen/src/rooks.rs` — rook move generation
- `crates/movegen/src/queens.rs` — queen move generation and combined sliding piece interface
- `crates/movegen/src/lib.rs` — updated with new module declarations and re-exports