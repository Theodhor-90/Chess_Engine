# Architecture

This document describes the engine's design decisions, core data structures, and search algorithms. It is targeted at engine authors and contributors who need a cohesive overview of the codebase.

## Design Decisions & Rationale

### Bitboard Representation

The board is represented with 12 piece bitboards (`piece_bb: [Bitboard; 12]`, one per piece-color combination) plus occupancy bitboards (`occupied_by: [Bitboard; 2]` for per-color occupancy and `occupied: Bitboard` for all pieces). Each `Bitboard` is a `u64` where each bit corresponds to a square.

Bitboards are used over mailbox because set-based operations (intersection, union, complement) map directly to single CPU instructions (AND, OR, NOT), enabling parallel move generation and attack detection for all pieces of a type simultaneously. The square mapping uses little-endian rank-file order: a1 = 0, h8 = 63.

Reference: `crates/types/src/bitboard.rs`

### Magic Bitboards

Sliding piece (bishop and rook) move generation uses the magic bitboard technique. For each square, a blocker mask is pre-computed that includes all relevant squares on the piece's rays, excluding edge squares to reduce the number of blocker configurations. The attack lookup formula is:

```
attacks = lookup_table[square][(blockers * magic) >> shift]
```

Pre-computed magic numbers are stored for all 64 bishop squares and 64 rook squares. Queen attacks are computed as the union of bishop and rook attacks for the same square and occupancy.

Reference: `crates/movegen/src/magic.rs`

### Packed Move Encoding

Moves are encoded in a 16-bit `u16` with the layout `from(6) | to(6) | flags(4)`:

- Bits 15..10: source square (6 bits)
- Bits 9..4: destination square (6 bits)
- Bits 3..0: move type flags (4 bits)

The 16 flag values are:

| Value | Flag |
|-------|------|
| 0 | Quiet |
| 1 | Double pawn push |
| 2 | King-side castle |
| 3 | Queen-side castle |
| 4 | Capture |
| 5 | En passant |
| 6–7 | Reserved |
| 8 | Knight promotion |
| 9 | Bishop promotion |
| 10 | Rook promotion |
| 11 | Queen promotion |
| 12 | Knight promotion capture |
| 13 | Bishop promotion capture |
| 14 | Rook promotion capture |
| 15 | Queen promotion capture |

This compact encoding fits in a single register and enables fast move comparison.

Reference: `crates/types/src/chess_move.rs`

### Zobrist Hashing

Position hashing uses an XORShift64-based deterministic PRNG to generate random keys at compile time. Four key tables are used:

- `PIECE_SQUARE_KEYS[768]` — 12 pieces × 64 squares
- `SIDE_TO_MOVE_KEY` — single key toggled each move
- `CASTLING_KEYS[16]` — indexed by the 4-bit castling rights value
- `EN_PASSANT_KEYS[8]` — one per file

Updates are incremental via XOR during `make_move`: XOR out the old state, XOR in the new state. The hash field lives directly in `Position` and is updated in-place.

Reference: `crates/board/src/zobrist.rs`

### Transposition Table Layout

`TtEntry` contains:

- `key: u16` — verification key (upper 16 bits of Zobrist hash via `hash >> 48`)
- `depth: u8`
- `score: i32`
- `bound: BoundType` — enum with variants `Exact`, `LowerBound`, `UpperBound`
- `best_move: Option<Move>`
- `age: u8` — generation counter

The table is a `Vec<TtEntry>` with power-of-two sizing for fast modulo via bitmask (`index = hash as usize & mask`).

Replacement policy: always replace stale entries (different generation); among same-generation entries, prefer deeper or equal depth. Mate scores are adjusted by ply depth on store/retrieve (`score_to_tt`/`score_from_tt`).

Reference: `crates/search/src/tt.rs`

### NNUE Architecture

The network uses HalfKP input features: 64 king squares × 10 colored non-king pieces × 64 piece squares = 40,960 features per perspective.

Network topology:

```
Input(40960) → L1(256, accumulator) → ClippedReLU → L2(32) → ClippedReLU → Output(1)
```

Quantization:

- Accumulator layer: `i16` weights and biases with shift = 6 (÷ 64)
- Hidden layers: `i8` weights and `i32` biases with shift = 6
- ClippedReLU clamps to `[0, 127]` (fits in `i8`)

Incremental accumulator updates: on non-king quiet moves, only the affected features are added/removed from both perspectives. On king moves or castling, a full refresh is performed from the position. SIMD-accelerated vector operations (`vec_add_i16`, `vec_sub_i16`) are used for accumulator updates.

Reference: `crates/nnue/src/`

## Core Data Structures

### `Bitboard`

Newtype `struct Bitboard(u64)`. Operator overloads: `BitAnd`, `BitOr`, `BitXor`, `Not`, `Shl`, `Shr` plus their assign variants (`BitAndAssign`, `BitOrAssign`, etc.). Key methods: `pop_count()`, `lsb()`, `msb()`, `is_empty()`. Implements `IntoIterator` to yield set bit indices via `BitboardIter`. Constants: `EMPTY`, `FULL`.

Reference: `crates/types/src/bitboard.rs`

### `Position`

Fields:

- `piece_bb: [Bitboard; 12]` — one per `Piece` (color × kind)
- `occupied_by: [Bitboard; 2]` — per `Color`
- `occupied: Bitboard` — all pieces
- `side_to_move: Color`
- `castling_rights: CastlingRights` — 4-bit `u8` bitfield with `WHITE_KINGSIDE` (0b0001), `WHITE_QUEENSIDE` (0b0010), `BLACK_KINGSIDE` (0b0100), `BLACK_QUEENSIDE` (0b1000) flags
- `en_passant: Option<Square>`
- `halfmove_clock: u8`
- `fullmove_counter: u16`
- `hash: u64` — Zobrist hash

Key operations:

- `make_move` returns `UndoInfo` containing the captured piece, prior castling rights, prior en passant, prior halfmove clock, and prior hash.
- `unmake_move` restores all state from `UndoInfo`.
- Castling rights are auto-updated via the `CASTLING_RIGHTS_MASK[64]` lookup table on every move.
- `is_square_attacked` checks if a square is attacked by a given side.
- FEN parsing/serialization via `from_fen`/`to_fen`.

Reference: `crates/board/src/position.rs`

### `Move`

`struct Move(u16)`. The bit layout is described in the Packed Move Encoding section above. Methods: `from_sq()`, `to_sq()`, `flags()`, `is_capture()`, `is_promotion()`, `promotion_piece()`, `is_castling()`, `is_en_passant()`, `is_double_pawn_push()`. The `Display` impl outputs UCI long-algebraic notation (e.g. `e2e4`, `e7e8q`).

Reference: `crates/types/src/chess_move.rs`

### Transposition Table

`TtEntry` and `TranspositionTable` structs. See the Transposition Table Layout section above for entry format.

`TranspositionTable` methods:

- `new(mb)` — allocates power-of-two entries from a megabyte budget
- `probe(hash)` — returns `Option<TtEntry>`
- `store(hash, entry)` — applies the replacement policy
- `new_generation()` — increments the age counter
- `clear()` — zeroes all entries
- `resize(mb)` — reallocates the table

Reference: `crates/search/src/tt.rs`

### NNUE Accumulator and Network Weights

`Network` holds all weight arrays:

- `input_weights: Box<[i16]>` — 40960 × 256
- `input_bias: Box<[i16]>` — 256
- `hidden1_weights: Box<[i8]>` — 512 × 32 (two perspectives concatenated)
- `hidden1_bias: Box<[i32]>` — 32
- `hidden2_weights: Box<[i8]>` — 32
- `hidden2_bias: i32`

`Accumulator` has `white: Vec<i16>` and `black: Vec<i16>` (each 256 elements) representing L1 activations from each king's perspective.

`AccumulatorStack` maintains a stack of `AccumulatorState` entries for make/unmake. Each state tracks dirty flags per perspective and a king-moved flag. `push_move` clones the current accumulator and applies feature deltas. `pop` restores the previous state. `refresh_if_needed` does a full recomputation when the king has moved or dirty flags are set.

Reference: `crates/nnue/src/network.rs`, `crates/nnue/src/accumulator.rs`

## Search Algorithm Explanations

### Alpha-Beta Framework and PVS

The search uses a negamax formulation of alpha-beta. Principal Variation Search (PVS) is applied: the first move is searched with the full `[alpha, beta]` window; subsequent moves are searched with a zero-window `[alpha, alpha + 1]`. On a fail-high (score > alpha && score < beta), the move is re-searched with the full window.

Reference: the `negamax()` function in `crates/search/src/lib.rs`

### Iterative Deepening with Aspiration Windows

The `search()` function iterates from depth 1 upward. At depth >= 5, aspiration windows are used: the initial window is `[prev_score - 25, prev_score + 25]`. On fail-low or fail-high, the window is widened by a factor of 4. After 2 consecutive failures in the same direction, that bound opens to +/- infinity. The search aborts when the time budget is exceeded, returning the best move found so far.

### Quiescence Search

Quiescence search extends the search beyond the main depth with captures and promotions only. A stand-pat evaluation is computed: if the static eval >= beta, return beta (beta cutoff). If the static eval > alpha, raise alpha. All tactical moves (captures and promotions) are searched, ordered by MVV-LVA. This prevents the horizon effect by resolving tactical sequences before returning an evaluation.

### Pruning Techniques

Four pruning methods are used:

**Null move pruning**: At depth >= 3, with non-pawn material present, not in check, and beta not close to mate — the side to move skips a move. A reduced-depth search with R = 2 (depth <= 5) or R = 3 (depth > 5) is performed at a zero-window `[-beta, -beta + 1]`. If the null-move score >= beta, prune the subtree.

**Late move reductions (LMR)**: For non-tactical, non-killer, non-TT, non-PV moves that are not giving check, searched after the first 3 moves at depth >= 3 — reduce the search depth by `floor(ln(depth) * ln(move_number) / 1.80)`, pre-computed in a 64 × 64 table. On fail-high, re-search at full depth with a zero-window, then full window on a second fail-high.

**Futility pruning**: At depth <= 3, if `static_eval + margin <= alpha`, skip quiet non-checking moves. Margins: `[0, 180, 360, 540]` indexed by depth.

**Reverse futility pruning**: At depth <= 3, if `static_eval - margin >= beta`, return the static eval. Margins: `[0, 120, 350, 550]` indexed by depth.

Additionally, losing captures (negative SEE) are pruned at depth <= 3 for moves that are not TT moves and not PV moves.

### Extensions

**Check extension**: When the side to move is in check, extend the search by +1 ply.

**Singular extension**: At depth >= 6, if the TT entry has a lower-bound or exact score at sufficient depth and provides a best move, a reduced-depth (depth / 2) verification search is performed excluding that move with a zero-window at `tt_score - 64`. If all alternatives score below this threshold, the TT move is extended by +1 ply.

### Move Ordering

Moves are ordered by priority (highest to lowest):

1. **TT move** — the move stored in the transposition table for this position
2. **PV move** — the move from the previous iteration's principal variation at this ply
3. **Captures** — ordered by MVV-LVA (Most Valuable Victim – Least Valuable Attacker); losing captures (negative SEE) are demoted below quiet moves
4. **Killer moves** — 2 per ply, quiet moves that caused beta cutoffs in sibling nodes
5. **Counter-moves** — indexed by the previous move's piece kind and destination square
6. **History heuristic** — quiet moves scored by accumulated beta-cutoff bonuses (`depth^2` per cutoff); negative penalties for searched but non-cutoff moves

Reference: `crates/search/src/ordering.rs`

### Time Management

Time allocation formula: `budget = time_left / moves_to_go + increment`. Default `moves_to_go` is 25 if not provided by the GUI. The minimum budget is 1 ms. Time is checked every 1024 nodes (`nodes & 1023 == 0`). The search also respects a `stop` flag via `Arc<AtomicBool>`. Additional search modes are supported: `depth` (fixed depth), `nodes` (fixed node count), `movetime` (fixed time), and `infinite` (until explicit stop).

Reference: `crates/uci/src/time.rs`
