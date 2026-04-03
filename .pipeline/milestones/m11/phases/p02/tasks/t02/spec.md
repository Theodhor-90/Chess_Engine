# T02 — Architecture Documentation

## Objective

Write an architecture document explaining the engine's design decisions, core data structures, and search algorithms, targeted at other engine authors and potential contributors.

## Deliverable

- **File**: `ARCHITECTURE.md` at the repository root

## Required Content

### Design Decisions & Rationale
- Bitboard representation (12 piece bitboards + occupancy bitboards, why bitboards over mailbox)
- Magic bitboards for sliding piece move generation
- Packed move encoding (16-bit `u16` — from(6) | to(6) | flags(4))
- Zobrist hashing for position identification and transposition table keying
- Transposition table layout (entry structure, replacement policy, power-of-two sizing)
- NNUE architecture (HalfKP/HalfKAv2 input features, accumulator structure, quantized weights, incremental updates)

### Core Data Structures
- `Bitboard` newtype (`u64`) with operator overloads
- `Position` struct (piece bitboards, occupancy, side to move, castling rights, en passant, clocks)
- `Move` packed `u16` encoding
- Transposition table entry and hash table
- NNUE accumulator and network weights

### Search Algorithm Explanations
- Alpha-beta framework and Principal Variation Search (PVS)
- Iterative deepening with aspiration windows
- Quiescence search
- Pruning techniques: null move pruning, LMR, futility pruning, reverse futility pruning
- Extensions: check extension, singular extension
- Move ordering: TT move, MVV-LVA, killer moves, history heuristic, counter-move heuristic, SEE
- Time management

## Constraints

- Standalone document (not scattered inline comments) — provides a cohesive overview
- Target audience: other engine authors and contributors who need to understand the codebase
- Covers the final engine state including all milestones M01–M10

## Dependencies

- **T01 (README)**: No hard dependency, but both documents should be consistent in their descriptions of the engine
- Phase P01 (Build & Distribution) must be complete — architecture docs reference the final engine structure

## Verification Criteria

1. `ARCHITECTURE.md` exists at the repository root
2. Contains design decisions section covering bitboards, magic bitboards, move encoding, Zobrist hashing, TT layout, and NNUE architecture
3. Contains data structures section covering all major types (Bitboard, Position, Move, TT, NNUE accumulator)
4. Contains search algorithms section covering PVS, iterative deepening, quiescence, pruning, extensions, and move ordering
5. Content is technically accurate and consistent with the actual implementation