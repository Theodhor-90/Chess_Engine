I now have all the information needed to produce the implementation plan. Here is the complete specification:

## 1. Deliverables

- **`ARCHITECTURE.md`** at the repository root — a standalone architecture document explaining the engine's design decisions, core data structures, and search algorithms. Targeted at other engine authors and potential contributors who need a cohesive overview of the codebase.

## 2. Dependencies

None. This task produces a Markdown documentation file with no build-time or runtime dependencies.

## 3. Implementation Details

### `ARCHITECTURE.md`

**Purpose:** Provide a cohesive technical reference for engine authors and contributors, covering the three required sections: design decisions & rationale, core data structures, and search algorithm explanations.

**Structure (top-level sections in order):**

#### Section: Design Decisions & Rationale

Six subsections as specified in the task spec:

1. **Bitboard Representation** — Explain the 12 piece bitboards (`piece_bb: [Bitboard; 12]`, one per piece-color combination) plus occupancy bitboards (`occupied_by: [Bitboard; 2]` for per-color, `occupied: Bitboard` for all pieces). Explain why bitboards over mailbox: set-based operations (intersection, union, complement) map to single CPU instructions (AND, OR, NOT), enabling parallel move generation and attack detection for all pieces of a type simultaneously. Reference little-endian rank-file mapping (a1 = 0, h8 = 63).

2. **Magic Bitboards** — Explain the magic bitboard technique for sliding piece (bishop/rook) move generation. Blocker mask generation excludes edge squares to reduce the number of blocker configurations. The formula: `attacks = lookup_table[square][(blockers * magic) >> shift]`. Pre-computed magic numbers for 64 bishop and 64 rook squares. Queen attacks computed as the union of bishop and rook attacks. Reference file: `crates/movegen/src/magic.rs`.

3. **Packed Move Encoding** — Document the 16-bit `u16` layout: `from(6) | to(6) | flags(4)` where bits 15..10 = source square, bits 9..4 = destination square, bits 3..0 = move type flags. List all 16 flag values (0=quiet, 1=double pawn push, 2=king castle, 3=queen castle, 4=capture, 5=en passant, 6–7=reserved, 8–11=promotions N/B/R/Q, 12–15=promotion captures N/B/R/Q). Note the compact encoding fits in a single register and enables fast move comparison. Reference: `crates/types/src/chess_move.rs`.

4. **Zobrist Hashing** — XORShift64-based deterministic PRNG generates random keys. Four key tables: `PIECE_SQUARE_KEYS[768]` (12 pieces × 64 squares), `SIDE_TO_MOVE_KEY` (single key toggled each move), `CASTLING_KEYS[16]` (indexed by 4-bit castling rights), `EN_PASSANT_KEYS[8]` (one per file). Incremental updates via XOR during `make_move`: XOR out old state, XOR in new state. The hash field lives directly in `Position` and is updated in-place. Reference: `crates/board/src/zobrist.rs`.

5. **Transposition Table Layout** — `TtEntry` structure: 16-bit verification key (upper 16 bits of Zobrist hash via `hash >> 48`), `u8` depth, `i32` score, `BoundType` enum (Exact/LowerBound/UpperBound), `Option<Move>` best move, `u8` age (generation counter). Table is a `Vec<TtEntry>` with power-of-two sizing for fast modulo via bitmask (`index = hash as usize & mask`). Replacement policy: always replace stale entries (different generation); among same-generation entries, prefer deeper or equal depth. Mate scores adjusted by ply depth on store/retrieve (`score_to_tt`/`score_from_tt`). Reference: `crates/search/src/tt.rs`.

6. **NNUE Architecture** — HalfKP input features: 64 king squares × 10 colored non-king pieces × 64 piece squares = 40,960 features per perspective. Network topology: Input(40960) → L1(256, accumulator) → ClippedReLU → L2(32) → ClippedReLU → Output(1). Quantization: accumulator layer uses `i16` weights/biases with shift=6 (÷64); hidden layers use `i8` weights and `i32` biases with shift=6. ClippedReLU clamps to `[0, 127]` (fits `i8`). Incremental accumulator updates: on non-king quiet moves, only add/remove affected features from both perspectives; on king moves or castling, full refresh from position. SIMD-accelerated vector operations (`vec_add_i16`, `vec_sub_i16`) for accumulator updates. Reference: `crates/nnue/src/`.

#### Section: Core Data Structures

Five subsections as specified:

1. **`Bitboard`** — Newtype `struct Bitboard(u64)`. Operator overloads: `BitAnd`, `BitOr`, `BitXor`, `Not`, `Shl`, `Shr` plus assign variants. Key methods: `pop_count()`, `lsb()`, `msb()`, `is_empty()`. Implements `Iterator` to yield set bit indices. Constants: `EMPTY`, `FULL`. Reference: `crates/types/src/bitboard.rs`.

2. **`Position`** — Fields: `piece_bb: [Bitboard; 12]` (one per Piece), `occupied_by: [Bitboard; 2]` (per Color), `occupied: Bitboard`, `side_to_move: Color`, `castling_rights: CastlingRights` (4-bit `u8` bitfield with WK/WQ/BK/BQ flags), `en_passant: Option<Square>`, `halfmove_clock: u8`, `fullmove_counter: u16`, `hash: u64` (Zobrist). Key operations: `make_move` returns `UndoInfo` (captured piece, prior castling rights, prior en passant, prior halfmove clock, prior hash). `unmake_move` restores all state from `UndoInfo`. Castling rights auto-updated via `CASTLING_RIGHTS_MASK[64]` lookup table on every move. `is_square_attacked` checks if a square is attacked by a given side. FEN parsing/serialization via `from_fen`/`to_fen`. Reference: `crates/board/src/position.rs`.

3. **`Move`** — `struct Move(u16)`. Described fully in the design decisions section; cross-reference it. Methods: `from_sq()`, `to_sq()`, `flags()`, `is_capture()`, `is_promotion()`, `promotion_piece()`, `is_castling()`, `is_en_passant()`, `is_double_pawn_push()`. `Display` impl outputs UCI long-algebraic notation (e.g. `e2e4`, `e7e8q`). Reference: `crates/types/src/chess_move.rs`.

4. **Transposition Table** — `TtEntry` and `TranspositionTable` struct. Cross-reference the design decisions section for layout details. `TranspositionTable` methods: `new(mb)` allocates power-of-two entries from megabyte budget, `probe(hash)` returns `Option<TtEntry>`, `store(hash, entry)` applies replacement policy, `new_generation()` increments age counter, `clear()`, `resize(mb)`. Reference: `crates/search/src/tt.rs`.

5. **NNUE Accumulator and Network Weights** — `Network` struct holds all weight arrays: `input_weights: Box<[i16]>` (40960×256), `input_bias: Box<[i16]>` (256), `hidden1_weights: Box<[i8]>` (512×32), `hidden1_bias: Box<[i32]>` (32), `hidden2_weights: Box<[i8]>` (32), `hidden2_bias: i32`. `Accumulator` struct with `white: Vec<i16>` and `black: Vec<i16>` (each 256 elements) representing L1 activations from each king's perspective. `AccumulatorStack` maintains a stack of `AccumulatorState` entries for make/unmake; each state tracks dirty flags and king-moved flag. `push_move` clones the current accumulator and applies feature deltas; `pop` restores the previous state; `refresh_if_needed` does a full recomputation when the king has moved. Reference: `crates/nnue/src/network.rs`, `crates/nnue/src/accumulator.rs`.

#### Section: Search Algorithm Explanations

Seven subsections as specified:

1. **Alpha-Beta Framework and PVS** — Negamax formulation of alpha-beta. PVS: first move searched with full `[alpha, beta]` window; subsequent moves searched with zero-window `[alpha, alpha+1]`; on fail-high (score > alpha && score < beta), re-search with full window. Reference the `negamax()` function in `crates/search/src/lib.rs`.

2. **Iterative Deepening with Aspiration Windows** — `search()` function iterates from depth 1 upward. At depth ≥ 5, uses aspiration windows: initial window `[prev_score - 25, prev_score + 25]`. On fail-low/fail-high, widen by factor of 4; after 2 consecutive failures in one direction, open to ±infinity. Search aborts when time budget exceeded, returning best move found so far.

3. **Quiescence Search** — Extends search beyond main depth with captures and promotions only. Stand-pat evaluation: if static eval ≥ beta, return beta. If static eval > alpha, raise alpha. Searches all tactical moves ordered by MVV-LVA. Prevents the horizon effect by resolving tactical sequences.

4. **Pruning Techniques** — Four pruning methods: (a) **Null move pruning**: at depth ≥ 3, non-pawn material present, not in check — skip a move, search with reduction R=2 (depth ≤ 5) or R=3 (depth > 5) at zero window; prune if score ≥ beta. (b) **Late move reductions (LMR)**: for non-tactical, non-killer, non-TT, non-PV moves searched after the first 3 moves at depth ≥ 3 — reduce by `floor(ln(depth) × ln(move_number) / 1.80)`, pre-computed in a 64×64 table; re-search at full depth on fail-high, then full window on second fail-high. (c) **Futility pruning**: at depth ≤ 3, if `static_eval + margin ≤ alpha`, skip quiet non-checking moves; margins `[0, 180, 360, 540]`. (d) **Reverse futility pruning**: at depth ≤ 3, if `static_eval - margin ≥ beta`, return static eval; margins `[0, 120, 350, 550]`. Also: losing captures (negative SEE) pruned at depth ≤ 3 for non-TT, non-PV moves.

5. **Extensions** — (a) **Check extension**: +1 ply when the side to move is in check. (b) **Singular extension**: at depth ≥ 6, if TT entry has a lower-bound or exact score at sufficient depth and provides a best move, do a reduced-depth (depth/2) search excluding that move with zero window at `tt_score - 64`; if all alternatives score below this threshold, extend the TT move by +1 ply.

6. **Move Ordering** — Priority (highest to lowest): TT move, PV move from previous iteration, captures ordered by MVV-LVA, killer moves (2 per ply), counter-moves (indexed by previous move's piece kind and to-square), history heuristic (quiet moves scored by accumulated beta-cutoff bonuses, `depth²` per cutoff, negative penalties for searched but non-cutoff moves). SEE used to prune losing captures. Reference: `crates/search/src/ordering.rs`.

7. **Time Management** — Time allocation: `budget = time_left / moves_to_go + increment`. Default `moves_to_go` = 25 if not provided. Minimum budget 1ms. Time checked every 1024 nodes (`nodes & 1023 == 0`). Also respects `stop` flag via `Arc<AtomicBool>`. Supports `depth`, `nodes`, `movetime`, `infinite` search modes. Reference: `crates/uci/src/time.rs`.

## 4. API Contracts

N/A — this task produces a documentation file with no programmatic API surface.

## 5. Test Plan

### What to Test

The task spec's verification criteria are:

1. `ARCHITECTURE.md` exists at the repository root
2. Contains design decisions section covering bitboards, magic bitboards, move encoding, Zobrist hashing, TT layout, and NNUE architecture
3. Contains data structures section covering Bitboard, Position, Move, TT, NNUE accumulator
4. Contains search algorithms section covering PVS, iterative deepening, quiescence, pruning, extensions, and move ordering
5. Content is technically accurate and consistent with the actual implementation

### Test Setup

No automated test infrastructure required — this is a documentation file. Verification is manual.

### Per-Test Specification

1. **File existence**: Run `test -f ARCHITECTURE.md` at the repository root — must succeed.
2. **Design decisions completeness**: Grep `ARCHITECTURE.md` for section headers covering all six required topics: "Bitboard Representation", "Magic Bitboards", "Move Encoding" (or "Packed Move Encoding"), "Zobrist Hashing", "Transposition Table", "NNUE".
3. **Data structures completeness**: Grep for all five required types: "Bitboard", "Position", "Move", "Transposition Table" (or "TtEntry"), "Accumulator" (or "Network").
4. **Search algorithms completeness**: Grep for required algorithm topics: "PVS" (or "Principal Variation Search"), "Iterative Deepening", "Quiescence", "Null Move", "LMR" (or "Late Move Reductions"), "Futility", "Check Extension", "Singular Extension", "Move Ordering".
5. **Technical accuracy**: Manually verify that struct field names, bit layouts, constant values, and algorithmic descriptions match the source code (cross-reference against `crates/types/src/chess_move.rs`, `crates/board/src/position.rs`, `crates/search/src/lib.rs`, `crates/search/src/tt.rs`, `crates/nnue/src/arch.rs`, `crates/nnue/src/accumulator.rs`, `crates/nnue/src/network.rs`).

## 6. Implementation Order

1. Write `ARCHITECTURE.md` at the repository root containing all three top-level sections (Design Decisions & Rationale, Core Data Structures, Search Algorithm Explanations) with all subsections as detailed in section 3 above.

This is a single-file deliverable. The entire content is written in one step.

## 7. Verification Commands

```bash
# 1. Verify file exists
test -f /Users/theodhor/Desktop/Portfolio/Chess_Engine/ARCHITECTURE.md && echo "EXISTS" || echo "MISSING"

# 2. Verify design decisions section covers all six required topics
grep -c "Bitboard Representation\|Magic Bitboard\|Move Encoding\|Packed Move Encoding\|Zobrist Hash\|Transposition Table\|NNUE" /Users/theodhor/Desktop/Portfolio/Chess_Engine/ARCHITECTURE.md

# 3. Verify data structures section present
grep -c "Position\|Bitboard\|Move\|TtEntry\|TranspositionTable\|Accumulator\|Network" /Users/theodhor/Desktop/Portfolio/Chess_Engine/ARCHITECTURE.md

# 4. Verify search algorithm topics present
grep -c "Principal Variation Search\|PVS\|Iterative Deepening\|Quiescence\|Null Move\|Late Move Reduction\|LMR\|Futility\|Check Extension\|Singular Extension\|Move Ordering" /Users/theodhor/Desktop/Portfolio/Chess_Engine/ARCHITECTURE.md

# 5. Verify project still builds (no accidental code changes)
cd /Users/theodhor/Desktop/Portfolio/Chess_Engine && cargo check 2>&1 | tail -1
```

## 8. Design Deviations

None.