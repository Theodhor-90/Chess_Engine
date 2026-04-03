Now I have all the context. Let me analyze the feedback patterns and synthesize the definitive spec.

**Recurring feedback issues:**
1. **LLM preamble text** — flagged in all 3 feedback rounds; specs must start with the heading
2. **Exit criterion #4 (CI gates)** — restates global conventions; should be removed (v1 feedback)
3. **Vague terms** — "multiple", "(or equivalent)" in T03 (v2 feedback); v3 fixed these
4. **Design Decision #5 (module organization)** — v3 feedback says it's implementation-level detail; Artifacts section is borderline redundant

**Best elements from each draft:**
- v1: Strongest overall structure, clean Design Decisions
- v2: Removed redundant exit criterion #4 (correct)
- v3: Fixed vague terms in T03 with specific FEN positions and committed to `generate_sliding_moves` name

**Tiebreaker decisions:**
- Remove Design Decision #5 (module organization) per v3 feedback — it's implementation detail, and Artifacts covers it
- Keep Artifacts section — it's useful for implementers and non-prescriptive
- Keep the specific FEN positions from v3 — removes ambiguity
- Remove exit criterion #4 per v1 feedback — global CI is pipeline-enforced

# Phase P02 — Sliding Pieces & Magic Bitboards

## Goal

Implement magic bitboard infrastructure and use it for O(1) sliding piece attack generation, delivering bishop, rook, and queen move generation functions that integrate with the existing `movegen` crate and produce correct pseudo-legal moves for all board positions.

## Design Decisions

1. **Pre-computed magic numbers**: Use known-good magic numbers (published values from the chess programming community) as compile-time constants rather than discovering them at runtime. This eliminates startup cost, keeps initialization deterministic, and follows the same const-initialization pattern established by king and knight attack tables in P01.

2. **Fancy vs plain magics**: Use "plain" (fixed-shift) magic bitboards. Each square gets a magic number and a fixed shift value. The lookup formula is `((occupancy & mask) * magic) >> shift`. This is simpler to implement and debug than "fancy" (variable-shift) magics, and the memory overhead difference is negligible for engine strength.

3. **Shared attack table structure**: Bishop and rook each get their own attack table arrays. Queen attack generation is computed as the union of bishop and rook attacks for the same square and occupancy — no separate queen tables are stored. This halves the table memory and avoids redundancy.

4. **Separation of attack lookup from move generation**: Attack lookup functions return a `Bitboard` of attacked squares. Move generation functions layer on top by masking out friendly pieces and converting to `Move` values. This separation allows `is_square_attacked` and future search code to use the attack lookup without constructing move lists.

## Tasks

- **T01 — Magic bitboard infrastructure**: Implement blocker mask generation for bishops and rooks (relevant occupancy bits along rays, excluding edge squares), define pre-computed magic numbers and shift values for all 64 squares for both piece types, and build the attack table initialization that maps each (square, occupancy) pair to the correct attack bitboard. Deliverable: `magic.rs` module with `bishop_attacks(square, occupancy) -> Bitboard` and `rook_attacks(square, occupancy) -> Bitboard` lookup functions, plus unit tests verifying correct attack sets for known square/occupancy combinations.

- **T02 — Bishop and rook move generation**: Implement `generate_bishop_moves` and `generate_rook_moves` functions that use the magic bitboard lookup from T01, mask out friendly piece occupancy, and produce pseudo-legal `Move` values appended to a move list. Deliverable: `bishops.rs` and `rooks.rs` modules with generation functions matching the existing `fn(pos: &Position, moves: &mut Vec<Move>)` signature, plus unit tests verifying correct move generation for representative positions.

- **T03 — Queen move generation and combined sliding piece interface**: Implement `generate_queen_moves` as the union of bishop and rook attacks for the queen's square. Implement `generate_sliding_moves` as a single function that calls `generate_bishop_moves`, `generate_rook_moves`, and `generate_queen_moves` to generate all sliding piece moves in one call. Re-export `generate_bishop_moves`, `generate_rook_moves`, `generate_queen_moves`, and `generate_sliding_moves` from `lib.rs`. Deliverable: `queens.rs` module, updated `lib.rs` exports, and unit tests verifying queen move generation for three specific positions: the starting position (queens blocked by own pieces), a midgame position with a queen, bishop, and rook on an open board (FEN: `r1bqk2r/ppppbppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQ1RK1 b kq - 5 4`), and a position with all sliding pieces blocked by pawns (FEN: `4k3/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQ - 0 1`).

## Exit Criteria

1. `bishop_attacks(square, occupancy)` and `rook_attacks(square, occupancy)` return correct attack bitboards for all 64 squares across all relevant occupancy configurations.
2. `generate_bishop_moves`, `generate_rook_moves`, and `generate_queen_moves` produce correct pseudo-legal moves that include all valid destination squares and exclude squares occupied by friendly pieces.
3. Queen move generation equals the union of bishop and rook attacks for every position.

## Dependencies

- **Phase P01 (Non-Sliding Pieces) complete** — P02 builds on the `movegen` crate structure, module conventions, and attack table patterns established in P01 (king and knight pre-computed attack tables, move generation function signature, `MoveFlag` usage).
- **Milestone M01 complete** — requires `Bitboard` with bitwise operations and bit manipulation, `Square` for indexing, `Move` encoding with flags, and `Position` for piece and occupancy bitboards.

## Artifacts

- `crates/movegen/src/magic.rs` — magic bitboard infrastructure (blocker masks, magic numbers, attack table initialization, attack lookup functions)
- `crates/movegen/src/bishops.rs` — bishop move generation
- `crates/movegen/src/rooks.rs` — rook move generation
- `crates/movegen/src/queens.rs` — queen move generation and combined sliding piece interface
- `crates/movegen/src/lib.rs` — updated with new module declarations and re-exports