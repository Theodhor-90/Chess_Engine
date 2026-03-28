# Run all tests in the search crate (includes new SEE tests and existing tests)
cargo test -p chess-search

# Run only the SEE tests
cargo test -p chess-search see::tests

# Run only the ordering tests (verify no regression)
cargo test -p chess-search ordering::tests

# Run the full workspace tests
cargo test --workspace

# Lint checks
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --check
```

## 8. Design Deviations

**Deviation 1: Attack table visibility**
- **Parent spec requires**: SEE uses "bitboard attacker/defender sets for the target square" — implies the function computes attackers internally.
- **Issue**: The `KNIGHT_ATTACKS` and `KING_ATTACKS` precomputed tables in the movegen crate are private (`const` without `pub`). Duplicating these 64-entry tables in the search crate would be wasteful and violate DRY.
- **Alternative chosen**: Make `KNIGHT_ATTACKS` and `KING_ATTACKS` public in the movegen crate (adding `pub` to the existing `const` declarations). This is a minimal change (one keyword per file) that gives SEE efficient O(1) knight and king attack lookups without duplicating data. The `bishop_attacks` and `rook_attacks` functions in `magic.rs` are already public.

**Deviation 2: Test scenario "PxN with NxP recapture (equal)"**
- **Parent spec says**: "PxN with NxP recapture (equal exchange)".
- **Issue**: A PxN capture where a knight recaptures the pawn yields SEE = 320 − 100 = 220 (winning for the initiator, not equal). A truly equal exchange would be, e.g., NxN or BxB. The spec labels this "equal" but the arithmetic gives 220.
- **Alternative chosen**: Implement the test as specified (PxN with recapture by a knight) and assert the mathematically correct SEE value of 220. The exchange is "equal" in the sense that both sides traded a piece, but the SEE score correctly reflects the material imbalance.

**Deviation 3: Test scenario "NxB (roughly equal)"**
- **Parent spec says**: "NxB — knight captures bishop (roughly equal, ~+10)".
- **Issue**: An undefended NxB yields SEE = 330 (full bishop value). To get ~+10, the bishop must be defended so that after NxB, the defender recaptures (capturing the knight worth 320), yielding 330 − 320 = 10.
- **Alternative chosen**: Set up the test with the bishop defended by a pawn, so the exchange is NxB, PxN, yielding SEE = 330 − 320 = 10 as the spec intends.