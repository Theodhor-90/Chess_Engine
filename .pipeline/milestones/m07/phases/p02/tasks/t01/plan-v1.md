I now have all the information needed. Let me produce the plan.

## 1. Deliverables

- `crates/eval/src/king_safety.rs` — New file. King safety evaluation module containing pawn shield scoring, open file penalty near king, and virtual mobility of attacking pieces toward king zone. Produces separate midgame and endgame `(i32, i32)` scores.
- `crates/eval/src/lib.rs` — Modified. Add `pub mod king_safety;` declaration, import and call `king_safety::evaluate_king_safety()` for both colors, add resulting midgame/endgame scores to the accumulators before tapered interpolation.
- `crates/eval/Cargo.toml` — Modified. Add `chess-movegen = { path = "../movegen" }` dependency (required for magic bitboard attack lookups used in king zone virtual mobility).

## 2. Dependencies

- `chess-movegen` crate (already in workspace, not yet a dependency of `chess-eval`) — needed for `magic::bishop_attacks`, `magic::rook_attacks`, `knights::KNIGHT_ATTACKS`, `king::KING_ATTACKS` to compute attacking piece mobility toward the king zone.
- All other dependencies (`chess-types`, `chess-board`) are already available in the eval crate.
- No new external crates required.

## 3. Implementation Details

### `crates/eval/Cargo.toml`

Add `chess-movegen` to `[dependencies]`:
```toml
chess-movegen = { path = "../movegen" }
```

### `crates/eval/src/king_safety.rs`

#### Constants

```rust
// Pawn shield bonuses (midgame / endgame) — scaled by rank distance from king
const PAWN_SHIELD_BONUS_MG: [i32; 4] = [35, 20, 8, 0]; // index 0 = rank distance 1, 1 = distance 2, etc.
const PAWN_SHIELD_BONUS_EG: [i32; 4] = [10, 5, 2, 0];

// Open file penalty near king
const OPEN_FILE_NEAR_KING_PENALTY_MG: i32 = -25;
const OPEN_FILE_NEAR_KING_PENALTY_EG: i32 = -10;

// King zone attack scoring weights per attacking piece type
const KNIGHT_ATTACK_WEIGHT: i32 = 2;
const BISHOP_ATTACK_WEIGHT: i32 = 2;
const ROOK_ATTACK_WEIGHT: i32 = 3;
const QUEEN_ATTACK_WEIGHT: i32 = 5;

// Scaling factor applied to total attack score
const ATTACK_SCORE_SCALE_MG: i32 = 4;
const ATTACK_SCORE_SCALE_EG: i32 = 1;
```

#### Utility: `FILE_MASKS`

Re-use the same file mask constants as in `pawn.rs`. Since they're `const` in `pawn.rs` (not `pub`), define them locally in `king_safety.rs`:

```rust
const FILE_MASKS: [Bitboard; 8] = [ /* same 8 file bitboards */ ];
```

#### Utility: `king_zone(king_sq: u32, color: Color) -> Bitboard`

Private function. Computes the king zone as defined in the phase spec: the 8 squares surrounding the king plus 3 squares in front of those (from the opponent's perspective — i.e., the direction the opponent attacks from).

Algorithm:
1. Start with `KING_ATTACKS[king_sq as usize]` (8 surrounding squares).
2. Compute the 3 "extended front" squares: shift the king bitboard forward by 2 ranks (White: `<< 16`, Black: `>> 16`), then include the 3 squares across that rank (center, left, right). Specifically: take the king bitboard, shift it by 2 ranks toward the opponent's side, then OR the left/right neighbors of that shifted bit (with file clipping).
3. Return the union, which is up to 11 squares.

#### `pub fn evaluate_king_safety(pos: &Position, color: Color) -> (i32, i32)`

Public function. Called once per color. Returns `(mg, eg)` scores from the perspective of the given color (positive = good for `color`).

**Step 1: Locate king and determine file neighborhood**
```rust
let king_sq = pos.piece_bitboard(Piece::new(color, PieceKind::King)).into_iter().next()?;
let king_file = king_sq % 8;
```
Compute the 3 shield files: `king_file - 1`, `king_file`, `king_file + 1` (clamped to 0..7).

**Step 2: Pawn shield evaluation**
For each of the 3 shield files:
- Find the closest friendly pawn on that file (for White, the lowest rank pawn above rank 0; for Black, the highest rank pawn below rank 7).
- If a pawn exists, compute rank distance from king rank. Index into `PAWN_SHIELD_BONUS_MG/EG` (distance 1 = best shield, distance >= 4 = no bonus).
- If no pawn exists on the file, no shield bonus for that file.

**Step 3: Open file penalty near king**
For each of the shield files (same 3 files):
- Check if the file has **no friendly pawns** at all.
- If empty of friendly pawns, apply `OPEN_FILE_NEAR_KING_PENALTY_MG/EG`.

**Step 4: Virtual mobility of attacking pieces toward king zone**
```rust
let zone = king_zone(king_sq, color);
let opp = color.opposite();
let occupied = pos.occupied();
```

For each opponent piece type (Knight, Bishop, Rook, Queen):
- Iterate over the opponent's piece bitboard.
- For each opponent piece, compute its attack bitboard using the movegen attack tables (KNIGHT_ATTACKS, bishop_attacks, rook_attacks, bishop|rook for queen).
- Count how many squares in the `zone` are attacked: `(attacks & zone).pop_count()`.
- Accumulate `count * weight` for the piece type.

Sum the weighted attack counts into a total attack score. Apply:
```rust
mg -= total_attack * ATTACK_SCORE_SCALE_MG;
eg -= total_attack * ATTACK_SCORE_SCALE_EG;
```

(Negative because attacks on our king are bad for us.)

**Return `(mg, eg)`.**

#### Unit Tests (in `#[cfg(test)] mod tests`)

**Test 1: Strong pawn shield scores higher than weak shield**
- Strong: `"r1bqkb1r/pppppppp/2n2n2/8/8/8/PPPPPPPP/RNBQKB1R w KQkq - 0 1"` — or a simpler position with White king on g1 and pawns on f2, g2, h2.
- FEN: `"4k3/8/8/8/8/8/5PPP/6K1 w - - 0 1"` (intact shield on f2/g2/h2 for king on g1)
- Weak: `"4k3/8/8/8/8/5P2/7P/6K1 w - - 0 1"` (missing g-pawn, advanced f-pawn)
- Assert: strong shield `mg` > weak shield `mg`.

**Test 2: Open file penalty near king**
- Position with open file near king: `"4k3/8/8/8/8/8/P4P2/4K3 w - - 0 1"` — King on e1, no pawn on d/e/f files (e-file open, d-file open).
- Position with closed files: `"4k3/8/8/8/8/8/3PPP2/4K3 w - - 0 1"` — King on e1 with pawns on d2/e2/f2.
- Assert: open-file position `mg` < closed-file position `mg`.

**Test 3: Attacking piece pressure scores higher danger**
- Pressure: `"4k3/8/8/8/8/8/5PPP/r5K1 w - - 0 1"` — Black rook on a1 file, but let's use `"4k3/8/8/8/8/8/5PPP/4r1K1 w - - 0 1"` — Black rook on e1 near White king on g1.
- No pressure: `"4k3/r7/8/8/8/8/5PPP/6K1 w - - 0 1"` — Black rook far from king zone.
- Assert: pressure position `mg` < no-pressure position `mg` (more negative = worse for White king).

**Test 4: Separate midgame and endgame values produced**
- Any position. Call `evaluate_king_safety`. Assert both `mg` and `eg` components are not always identical (use a position where shield bonus is meaningful in mg but lighter in eg).

### `crates/eval/src/lib.rs`

#### Module declaration
Add after `pub mod pawn;`:
```rust
pub mod king_safety;
```

#### Integration in `evaluate()`
After the pawn structure section (after line 59 in current code) and before the tapered interpolation (line 61), add:

```rust
let (w_ks_mg, w_ks_eg) = king_safety::evaluate_king_safety(pos, Color::White);
let (b_ks_mg, b_ks_eg) = king_safety::evaluate_king_safety(pos, Color::Black);
mg_score += w_ks_mg - b_ks_mg;
eg_score += w_ks_eg - b_ks_eg;
```

This follows the existing pattern where scores are accumulated from White's perspective (positive = White advantage).

## 4. API Contracts

```rust
// Primary public function
pub fn evaluate_king_safety(pos: &Position, color: Color) -> (i32, i32)
// Returns (midgame_score, endgame_score) for the given color.
// Positive = good for `color`, negative = bad for `color`.
```

Integration within `evaluate()`:
```rust
// Called from lib.rs evaluate()
let (w_ks_mg, w_ks_eg) = king_safety::evaluate_king_safety(pos, Color::White);
let (b_ks_mg, b_ks_eg) = king_safety::evaluate_king_safety(pos, Color::Black);
mg_score += w_ks_mg - b_ks_mg;
eg_score += w_ks_eg - b_ks_eg;
```

N/A for REST/HTTP.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| `strong_shield_scores_higher_than_weak` | Intact pawn shield (f2/g2/h2 in front of g1 king) produces higher mg score than a position with missing/advanced shield pawns |
| `open_file_near_king_penalized` | King on a file with no friendly pawns on adjacent files scores lower (more negative mg) than king behind closed files |
| `attacking_pieces_toward_king_zone` | Opponent pieces aimed at king zone produce a lower (more negative) score than opponent pieces far from king zone |
| `produces_separate_mg_eg_values` | King safety returns distinct midgame and endgame values (mg impact larger than eg impact for pawn shield) |

All tests use `Position::from_fen()` with known FEN strings. Tests call `king_safety::evaluate_king_safety()` directly for White and compare scores between two contrasting positions.

## 6. Implementation Order

1. `crates/eval/Cargo.toml` — add `chess-movegen` dependency
2. `crates/eval/src/king_safety.rs` — implement constants, `king_zone()`, `evaluate_king_safety()`, and unit tests
3. `crates/eval/src/lib.rs` — add `pub mod king_safety;` and integrate into `evaluate()`

## 7. Verification Commands

```bash
cargo test -p chess-eval
cargo clippy -p chess-eval -- -D warnings
cargo test --workspace
```

## 8. Design Deviations

**Deviation: Adding `chess-movegen` as a dependency of `chess-eval`**

- The task spec and phase spec do not explicitly mention adding `chess-movegen` as a dependency of the eval crate.
- The king zone virtual mobility computation requires attack bitboards for opponent pieces (knight attacks, bishop attacks via magic bitboards, rook attacks via magic bitboards, queen attacks as bishop|rook). These are only available in the `chess-movegen` crate (specifically `knights::KNIGHT_ATTACKS`, `magic::bishop_attacks`, `magic::rook_attacks`).
- Without this dependency, the eval crate would need to reimplement magic bitboard lookups or use a much simpler approximation. Since the movegen crate already provides efficient, correct attack generation, depending on it is the sound approach. This does not create a circular dependency (`movegen` depends on `types` and `board`; `eval` will depend on `types`, `board`, and `movegen` — no cycle).