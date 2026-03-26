# Phase P02 — Sliding Pieces & Magic Bitboards

## Goal

Implement magic bitboard lookup for sliding piece attack generation, enabling O(1) move generation for bishops, rooks, and queens. Magic bitboards are the standard technique for fast sliding piece move generation in competitive chess engines — they use a multiplication-and-shift hash to index into a pre-computed attack table based on the current board occupancy.

## Deliverables

- **Magic bitboard infrastructure** — blocker mask generation for each square (relevant occupancy bits along rays, excluding edge squares), magic number discovery (or use pre-computed/known-good magic numbers), and attack table initialization that maps each (square, occupancy) pair to the correct attack bitboard.
- **Bishop and rook move generation** — using magic bitboard lookup to produce attack bitboards, then masking with friendly piece occupancy to generate pseudo-legal moves.
- **Queen move generation** — computed as the union of bishop and rook attacks (bishop_attacks | rook_attacks) for the given square and occupancy. A combined sliding piece interface should be provided.

## Technical Constraints

- Magic numbers may be pre-computed constants (known-good values from the chess programming community) or discovered at startup via brute-force search. If discovered at runtime, initialization must complete in reasonable time (< 1 second).
- Attack tables must be indexed by square and use the magic bitboard hashing scheme: `((occupancy & mask) * magic) >> shift` to produce a table index.
- The sliding piece interface should cleanly separate the attack lookup (returns `Bitboard` of attacked squares) from move generation (converts attacks to `Move` list filtered by friendly occupancy).
- Code belongs in the `movegen` crate. All move types use the `Move` encoding from `types` crate.
- No `unsafe` unless provably necessary for performance (magic bitboard table access may warrant `unsafe` for unchecked indexing — if used, must include `// SAFETY:` comment). No `unwrap()` in library code.

## Dependencies

- **Phase P01 (Non-Sliding Pieces)** — P02 builds on the `movegen` crate infrastructure established in P01 (module structure, common move generation patterns, attack table conventions).
- **Milestone M01 complete** — requires `Bitboard` type with bitwise operations, `Square`, `Move` encoding, and `Position` for occupancy bitboards.