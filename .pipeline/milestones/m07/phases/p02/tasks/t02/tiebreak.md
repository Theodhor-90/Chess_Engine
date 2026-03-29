Good — the queen on e4 has 27 attack squares, confirming feedback-v3's correction. Now I have all the information needed to produce the definitive plan.

# T02 — Mobility Evaluation: Implementation Plan

## 1. Deliverables

### New File
- **`crates/eval/src/mobility.rs`** — Per-piece mobility counting and scoring module. Contains the `evaluate_mobility()` function that counts pseudo-legal moves for knights, bishops, rooks, and queens (excluding squares attacked by opponent pawns), applies piece-type-dependent scaling, and returns separate midgame and endgame `(i32, i32)` scores.

### Modified File
- **`crates/eval/src/lib.rs`** — Add `pub mod mobility;` declaration, import and call `mobility::evaluate_mobility()` for both colors, add the resulting midgame/endgame scores to the accumulators before tapered interpolation. Update the `tapered_eval_interpolation` test's expected value from `346` to `378` to account for the white knight's mobility bonus.

## 2. Dependencies

No new crate dependencies are required. The `chess-movegen` dependency (already present in `crates/eval/Cargo.toml` from T01) provides the attack generation functions needed:
- `chess_movegen::knights::KNIGHT_ATTACKS` — pre-computed knight attack table
- `chess_movegen::magic::{bishop_attacks, rook_attacks}` — magic bitboard sliding piece attacks

Existing crate dependencies used:
- `chess-types` — `Bitboard`, `Color`, `Piece`, `PieceKind`, `Square`
- `chess-board` — `Position` struct with `piece_bitboard()`, `occupied()`, `occupied_by()`

## 3. Implementation Details

### `crates/eval/src/mobility.rs`

**Purpose**: Count pseudo-legal moves for each non-pawn, non-king piece, exclude squares attacked by opponent pawns, and produce scaled midgame/endgame mobility scores.

**Constants**:

```rust
// Per-move mobility bonus (midgame, endgame) by piece type
const KNIGHT_MOBILITY_MG: i32 = 4;
const KNIGHT_MOBILITY_EG: i32 = 4;
const BISHOP_MOBILITY_MG: i32 = 5;
const BISHOP_MOBILITY_EG: i32 = 5;
const ROOK_MOBILITY_MG: i32 = 2;
const ROOK_MOBILITY_EG: i32 = 3;
const QUEEN_MOBILITY_MG: i32 = 1;
const QUEEN_MOBILITY_EG: i32 = 2;

// File masks for pawn attack computation
const NOT_A_FILE: Bitboard = Bitboard::new(0xFEFE_FEFE_FEFE_FEFE);
const NOT_H_FILE: Bitboard = Bitboard::new(0x7F7F_7F7F_7F7F_7F7F);
```

**Key logic**:

1. **Compute opponent pawn attacks**: Given the opponent's pawn bitboard, compute all squares attacked by opponent pawns using bitwise shifts. For white pawns (opponent when evaluating black): `((pawns & NOT_A_FILE) << 7) | ((pawns & NOT_H_FILE) << 9)`. For black pawns (opponent when evaluating white): `((pawns & NOT_H_FILE) >> 7) | ((pawns & NOT_A_FILE) >> 9)`. This produces a `Bitboard` of all squares controlled by opponent pawns.

2. **Compute safe squares**: The mobility mask is `!opponent_pawn_attacks` — squares NOT attacked by opponent pawns. Pieces can move to any square in their attack set that intersects this mask. Friendly pieces are NOT excluded from the mobility count (see Deviation 1).

3. **Per-piece mobility counting**:
   - **Knights**: For each friendly knight, get `KNIGHT_ATTACKS[sq]`, AND with safe squares, count bits.
   - **Bishops**: For each friendly bishop, get `bishop_attacks(sq, occupied)`, AND with safe squares, count bits.
   - **Rooks**: For each friendly rook, get `rook_attacks(sq, occupied)`, AND with safe squares, count bits.
   - **Queens**: For each friendly queen, get `bishop_attacks(sq, occupied) | rook_attacks(sq, occupied)`, AND with safe squares, count bits.

4. **Scoring**: Multiply each piece's mobility count by the piece-type-specific per-move bonus (separate MG and EG values). Sum all contributions.

**Exports**:

```rust
pub fn evaluate_mobility(pos: &Position, color: Color) -> (i32, i32)
```

Returns `(midgame_score, endgame_score)` for the given color.

**Helper** (private):

```rust
fn pawn_attacks(pawns: Bitboard, color: Color) -> Bitboard
```

Returns the bitboard of all squares attacked by the given color's pawns.

### `crates/eval/src/lib.rs`

**Changes**:
1. Add `pub mod mobility;` after `pub mod king_safety;`.
2. After the king safety scoring block (lines 62–65), add mobility evaluation for both colors and accumulate into `mg_score`/`eg_score`:

```rust
let (w_mob_mg, w_mob_eg) = mobility::evaluate_mobility(pos, Color::White);
let (b_mob_mg, b_mob_eg) = mobility::evaluate_mobility(pos, Color::Black);
mg_score += w_mob_mg - b_mob_mg;
eg_score += w_mob_eg - b_mob_eg;
```

3. Update the `tapered_eval_interpolation` test (line 113) from `assert_eq!(evaluate(&pos, &mut PawnHashTable::new()), 346)` to `assert_eq!(evaluate(&pos, &mut PawnHashTable::new()), 378)`.

**Rationale for new expected value (378)**:

The test position `4k3/8/8/8/3N4/8/8/4K3 w - - 0 1` has a white knight on d4 with 8 pseudo-legal safe moves (c2, e2, b3, f3, b5, f5, c6, e6) and no black pieces with mobility. The phase is 1 (one knight).

- Mobility adds +32 to both mg_score and eg_score (8 moves × 4 per-move bonus).
- Previous raw scores satisfied: `(mg * 1 + eg * 23) / 24 = 346`, so `mg + 23 * eg = 8304`.
- New raw scores: mg' = mg + 32, eg' = eg + 32.
- New tapered score: `((mg + 32) * 1 + (eg + 32) * 23) / 24 = (mg + 23*eg + 32 + 736) / 24 = (8304 + 768) / 24 = 9072 / 24 = 378`.

## 4. API Contracts

### `mobility::evaluate_mobility(pos: &Position, color: Color) -> (i32, i32)`

**Input**: A `Position` reference and a `Color` to evaluate mobility for.

**Output**: A tuple `(mg_score, eg_score)` where:
- `mg_score` is the total midgame mobility bonus for the given color (always >= 0)
- `eg_score` is the total endgame mobility bonus for the given color (always >= 0)

**Example**: In the starting position, each side has 2 knights with 2 moves each and 0 moves for all other pieces (bishops, rooks, queens are blocked). So `evaluate_mobility(startpos, Color::White)` returns `(2 * 2 * KNIGHT_MOBILITY_MG, 2 * 2 * KNIGHT_MOBILITY_EG)` = `(16, 16)`.

Since the starting position is symmetric, `evaluate_mobility(startpos, Color::White)` == `evaluate_mobility(startpos, Color::Black)`, so the net contribution to `evaluate()` is zero.

## 5. Test Plan

All tests are colocated in `crates/eval/src/mobility.rs` under `#[cfg(test)] mod tests`.

### Test 1: `startpos_mobility_symmetric`
**Setup**: Starting position FEN `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1`
**Verify**: `evaluate_mobility(pos, Color::White)` equals `evaluate_mobility(pos, Color::Black)`. Confirms symmetric positions produce equal mobility.

### Test 2: `open_position_higher_mobility_than_closed`
**Setup**: Two positions — one open (few pawns, pieces have room) and one closed (many pawns blocking pieces).
- Open: `4k3/8/8/8/8/8/8/4K2R w - - 0 1` (rook has many squares)
- Closed: `4k3/8/8/8/8/8/6PP/4K2R w - - 0 1` (rook blocked by pawns)
**Verify**: Open position white mobility MG score > closed position white mobility MG score.

### Test 3: `opponent_pawn_attacks_excluded`
**Setup**: Position where a knight has moves but some are controlled by opponent pawns.
- With pawns: `4k3/8/8/3pp3/8/4N3/8/4K3 w - - 0 1` (knight on e3, square index 20; black pawns on d5 and e5 attack c4, d4, e4, f4 — of the knight's eight target squares {c2, c4, d1, d5, f1, f5, g2, g4}, squares c4 and f4 are attacked by opponent pawns and excluded from safe mobility)
- Without pawns: `4k3/8/8/8/8/4N3/8/4K3 w - - 0 1` (knight on e3; no opponent pawns)
**Verify**: `evaluate_mobility(with_pawns, Color::White)` MG score < `evaluate_mobility(without_pawns, Color::White)` MG score, confirming opponent pawn attacks reduce mobility.

### Test 4: `queen_mobility_scaled_less_than_knight`
**Setup**: Two positions to demonstrate that queen per-move scaling is lighter than knight per-move scaling.
- Knight position: `4k3/8/8/8/4N3/8/8/4K3 w - - 0 1` — knight on e4, 8 safe moves (c3, c5, d2, d6, f2, f6, g3, g5), MG = 8 × 4 = 32
- Queen position: `4k3/8/8/8/4Q3/8/8/4K3 w - - 0 1` — queen on e4, 27 safe moves (bishop_attacks: 13 + rook_attacks: 14; no opponent pawns, friendly pieces not excluded per Deviation 1), MG = 27 × 1 = 27
**Verify**: Knight position MG score (32) > queen position MG score (27), demonstrating that queen per-move scaling is lighter than knight per-move scaling.

### Test 5: `mobility_produces_midgame_and_endgame_scores`
**Setup**: Position `4k3/8/8/8/4N3/8/8/4K3 w - - 0 1` (knight on e4 with 8 moves).
**Verify**: Both MG and EG components of the return tuple are > 0 when pieces have available moves. Specifically, MG = 32 and EG = 32.

### Existing test updates in `lib.rs`:
- `startpos_eval_is_zero`: Continues to pass since mobility is symmetric in the starting position and the net contribution is zero.
- `tapered_eval_interpolation`: Expected value updated from `346` to `378` to account for the white knight on d4 contributing +32 to both mg_score and eg_score (8 safe moves × KNIGHT_MOBILITY_MG=4), which under the tapered formula with phase=1 yields `(8304 + 768) / 24 = 378`.

## 6. Implementation Order

1. **Create `crates/eval/src/mobility.rs`**: Implement the `pawn_attacks()` helper function, the `evaluate_mobility()` public function with all constants and per-piece logic, and all `#[cfg(test)]` unit tests.

2. **Modify `crates/eval/src/lib.rs`**: Add `pub mod mobility;` declaration, integrate the mobility call into `evaluate()` between the king safety block and the tapered interpolation, and update the `tapered_eval_interpolation` test's expected value from `346` to `378`.

3. **Run verification**: `cargo test -p chess-eval`, `cargo clippy -p chess-eval -- -D warnings`, `cargo fmt --check -p chess-eval`.

## 7. Verification Commands

```bash
# Run all eval crate tests (includes new mobility tests and existing regression tests)
cargo test -p chess-eval

# Run only mobility tests for focused verification
cargo test -p chess-eval mobility

# Clippy lint check
cargo clippy -p chess-eval -- -D warnings

# Format check
cargo fmt -p chess-eval -- --check

# Full workspace build to catch any cross-crate issues
cargo build --workspace
```

## 8. Design Deviations

**Deviation 1: Mobility mask includes friendly pieces**

- **Parent spec states**: "count pseudo-legal moves per piece (excluding squares attacked by opponent pawns)"
- **Clarification**: Standard chess engine mobility evaluation counts all attack squares minus opponent-pawn-controlled squares, without also subtracting friendly-piece squares. This is the convention used by Stockfish and other engines. The task spec says "excluding squares attacked by opponent pawns" — it does not say "excluding friendly piece squares." Counting all attacks (including those landing on friendly pieces) is the standard approach and produces a more useful positional signal, so no subtraction of friendly occupancy is applied.

No other deviations are needed.