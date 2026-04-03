Now I have all the context. Let me produce the complete revised plan.

# T03 — Static Exchange Evaluation (SEE): Implementation Plan

## 1. Overview

This task implements Static Exchange Evaluation (SEE) for the chess engine's search crate. SEE evaluates whether a capture sequence on a given square is winning, equal, or losing by simulating alternating recaptures from least-valuable to most-valuable attacker. The result is integrated into two places: (1) move ordering in `ordering.rs`, where good captures (SEE >= 0) are scored above killers and bad captures (SEE < 0) below quiet moves; and (2) the negamax search loop in `lib.rs`, where captures with negative SEE are pruned in non-critical nodes.

## 2. Deliverables

| # | File | Action | Description |
|---|------|--------|-------------|
| 1 | `crates/movegen/src/knights.rs` | Modify | Change `const KNIGHT_ATTACKS` to `pub const KNIGHT_ATTACKS` (line 36) |
| 2 | `crates/movegen/src/king.rs` | Modify | Change `const KING_ATTACKS` to `pub const KING_ATTACKS` (line 36) |
| 3 | `crates/search/src/see.rs` | Create | New module containing the `see` function, attack helper functions, and unit tests |
| 4 | `crates/search/src/ordering.rs` | Modify | Update `score_move` to use SEE for capture scoring; add `BAD_CAPTURE_BASE` constant |
| 5 | `crates/search/src/lib.rs` | Modify | Add `pub mod see;`, add bad capture pruning in `negamax` |

## 3. Detailed Implementation

### Step 1: Expose Attack Tables in Movegen Crate

**File: `crates/movegen/src/knights.rs` (line 36)**

Change:
```rust
const KNIGHT_ATTACKS: [Bitboard; 64] = init_knight_attacks();
```
To:
```rust
pub const KNIGHT_ATTACKS: [Bitboard; 64] = init_knight_attacks();
```

**File: `crates/movegen/src/king.rs` (line 36)**

Change:
```rust
const KING_ATTACKS: [Bitboard; 64] = init_king_attacks();
```
To:
```rust
pub const KING_ATTACKS: [Bitboard; 64] = init_king_attacks();
```

**Rationale**: SEE needs O(1) knight and king attack lookups. The `bishop_attacks` and `rook_attacks` functions in `magic.rs` are already public. Duplicating the 64-entry const tables in the search crate would violate DRY. Adding `pub` is a one-keyword change per file. Both modules are already `pub mod` in `crates/movegen/src/lib.rs`.

### Step 2: Create `crates/search/src/see.rs`

#### 2a. Module-Level Constants and Imports

```rust
use chess_board::Position;
use chess_types::{Bitboard, Color, Move, Piece, PieceKind, Square};

const NOT_A_FILE: Bitboard = Bitboard::new(0xFEFE_FEFE_FEFE_FEFE);
const NOT_H_FILE: Bitboard = Bitboard::new(0x7F7F_7F7F_7F7F_7F7F);

/// Piece values used for SEE, matching eval crate values.
const SEE_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 20000];
```

The `SEE_VALUES` array is indexed by `PieceKind` discriminant (Pawn=0, Knight=1, Bishop=2, Rook=3, Queen=4, King=5). The King value is set to 20000 (a large sentinel) so that king "captures" are never profitable to recapture against — the king is always the last resort attacker.

**Dependency note**: The search crate's `Cargo.toml` already lists `chess-movegen` as a dependency, so no changes are needed to `Cargo.toml`.

#### 2b. Attack Helper Functions

**Pawn attacks for a single square** (no precomputed table exists for single-square pawn attacks):

```rust
fn pawn_attacks(sq: Square, side: Color) -> Bitboard {
    let bb = Bitboard::new(1u64 << sq.index());
    match side {
        Color::White => ((bb & NOT_A_FILE) << 7) | ((bb & NOT_H_FILE) << 9),
        Color::Black => ((bb & NOT_H_FILE) >> 7) | ((bb & NOT_A_FILE) >> 9),
    }
}
```

This mirrors the shift logic in `crates/movegen/src/pawns.rs`. White pawns attack northwest (`<<7`) and northeast (`<<9`); black pawns attack southeast (`>>7`) and southwest (`>>9`). File masks prevent wrapping. The `Bitboard` type implements `Shl<u32>` and `Shr<u32>` (see `crates/types/src/bitboard.rs` lines 76-88).

**All attackers to a square** (given an occupancy bitboard):

```rust
fn attackers_to(pos: &Position, sq: Square, occupied: Bitboard) -> Bitboard {
    let knights = chess_movegen::knights::KNIGHT_ATTACKS[sq.index() as usize]
        & (pos.piece_bitboard(Piece::new(Color::White, PieceKind::Knight))
            | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Knight)));

    let kings = chess_movegen::king::KING_ATTACKS[sq.index() as usize]
        & (pos.piece_bitboard(Piece::new(Color::White, PieceKind::King))
            | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::King)));

    let bishop_rays = chess_movegen::magic::bishop_attacks(sq, occupied);
    let rook_rays = chess_movegen::magic::rook_attacks(sq, occupied);

    let bishops_queens = bishop_rays
        & (pos.piece_bitboard(Piece::new(Color::White, PieceKind::Bishop))
            | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Bishop))
            | pos.piece_bitboard(Piece::new(Color::White, PieceKind::Queen))
            | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Queen)));

    let rooks_queens = rook_rays
        & (pos.piece_bitboard(Piece::new(Color::White, PieceKind::Rook))
            | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Rook))
            | pos.piece_bitboard(Piece::new(Color::White, PieceKind::Queen))
            | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Queen)));

    let white_pawns = pawn_attacks(sq, Color::Black)
        & pos.piece_bitboard(Piece::new(Color::White, PieceKind::Pawn));
    let black_pawns = pawn_attacks(sq, Color::White)
        & pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Pawn));

    knights | kings | bishops_queens | rooks_queens | white_pawns | black_pawns
}
```

Note the pawn attack symmetry: to find white pawns that attack square `sq`, we compute where a black pawn on `sq` would attack (the reverse direction) and intersect with white pawns.

The key property for X-ray handling: `bishop_attacks` and `rook_attacks` take an `occupied` bitboard parameter. When we remove a piece from `occupied` during the swap loop, re-calling these functions with the updated occupancy will automatically reveal any sliding piece that was hidden behind the removed piece (X-ray attack).

#### 2c. Least Valuable Attacker

```rust
fn least_valuable_attacker(
    pos: &Position,
    attackers: Bitboard,
    side: Color,
) -> Option<(Square, PieceKind)> {
    for &kind in &[
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ] {
        let piece_bb = pos.piece_bitboard(Piece::new(side, kind)) & attackers;
        if !piece_bb.is_empty() {
            let sq_idx = piece_bb.lsb().expect("non-empty bitboard has lsb");
            let sq = Square::new(sq_idx as u8).expect("valid square index");
            return Some((sq, kind));
        }
    }
    None
}
```

Iterates from least valuable (Pawn) to most valuable (King), returning the first matching piece from the attacker set for the given side.

#### 2d. The SEE Function

```rust
/// Static Exchange Evaluation: returns the material gain/loss for the
/// side making `mv`, assuming both sides capture optimally on the target
/// square.
pub fn see(pos: &Position, mv: Move) -> i32 {
    let from = mv.from_sq();
    let to = mv.to_sq();

    // Determine the initial victim value.
    let victim_value = if mv.is_en_passant() {
        SEE_VALUES[PieceKind::Pawn as usize]
    } else {
        match pos.piece_on(to) {
            Some(p) => SEE_VALUES[p.kind as usize],
            None => 0, // quiet move — SEE is 0
        }
    };

    // The attacker's piece kind.
    let attacker_piece = pos
        .piece_on(from)
        .expect("attacker must exist on from_sq");
    let mut attacker_value = SEE_VALUES[attacker_piece.kind as usize];

    // For promotions, the attacker becomes the promoted piece.
    if mv.is_promotion() {
        if let Some(promo_kind) = mv.promotion_piece() {
            attacker_value = SEE_VALUES[promo_kind as usize];
        }
    }

    // Build the gain array. gain[i] is the material gain for the side
    // making capture i (relative to that side, always positive perspective).
    let mut gain = [0i32; 32];
    let mut depth = 0usize;

    gain[0] = victim_value;
    if mv.is_promotion() {
        if let Some(promo_kind) = mv.promotion_piece() {
            gain[0] += SEE_VALUES[promo_kind as usize] - SEE_VALUES[PieceKind::Pawn as usize];
        }
    }

    // Remove the initial attacker from occupancy.
    let mut occupied = pos.occupied();
    occupied = occupied ^ Bitboard::new(1u64 << from.index());

    // For en passant, also remove the captured pawn.
    if mv.is_en_passant() {
        let ep_victim_sq = match attacker_piece.color {
            Color::White => Square::new(to.index() - 8).expect("valid ep square"),
            Color::Black => Square::new(to.index() + 8).expect("valid ep square"),
        };
        occupied = occupied ^ Bitboard::new(1u64 << ep_victim_sq.index());
    }

    // Compute all attackers to the target square with updated occupancy.
    let mut attackers = attackers_to(pos, to, occupied) & occupied;

    let mut side = attacker_piece.color.opposite();

    loop {
        depth += 1;
        if depth >= 32 {
            break;
        }

        // The current side's gain is: capturing the piece left on the
        // square (worth attacker_value from the previous capture) minus
        // whatever the opponent can gain after.
        gain[depth] = attacker_value - gain[depth - 1];

        // Find the least valuable attacker for the current side.
        let side_attackers = attackers & pos.occupied_by(side);
        let lva = least_valuable_attacker(pos, side_attackers, side);
        let (attacker_sq, attacker_kind) = match lva {
            Some(pair) => pair,
            None => break, // No more attackers; stop.
        };

        // The piece now on the square (to be captured next) is worth
        // the current attacker's value.
        attacker_value = SEE_VALUES[attacker_kind as usize];

        // Remove the attacker from occupancy (reveals X-ray attacks).
        occupied = occupied ^ Bitboard::new(1u64 << attacker_sq.index());

        // Selectively re-check X-ray attacks based on piece type removed.
        if matches!(
            attacker_kind,
            PieceKind::Pawn | PieceKind::Bishop | PieceKind::Queen
        ) {
            let diag = chess_movegen::magic::bishop_attacks(to, occupied);
            let diag_pieces = diag
                & (pos.piece_bitboard(Piece::new(Color::White, PieceKind::Bishop))
                    | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Bishop))
                    | pos.piece_bitboard(Piece::new(Color::White, PieceKind::Queen))
                    | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Queen)));
            attackers = attackers | diag_pieces;
        }
        if matches!(
            attacker_kind,
            PieceKind::Rook | PieceKind::Queen
        ) {
            let straight = chess_movegen::magic::rook_attacks(to, occupied);
            let straight_pieces = straight
                & (pos.piece_bitboard(Piece::new(Color::White, PieceKind::Rook))
                    | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Rook))
                    | pos.piece_bitboard(Piece::new(Color::White, PieceKind::Queen))
                    | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Queen)));
            attackers = attackers | straight_pieces;
        }

        // Remove used attacker from the active set.
        attackers = attackers & occupied;

        // Switch sides.
        side = side.opposite();

        // If the attacker was a king and the opponent still has attackers,
        // the king capture is illegal — stop before it.
        if attacker_kind == PieceKind::King && !(attackers & pos.occupied_by(side)).is_empty() {
            depth -= 1;
            break;
        }
    }

    // Backward propagation: negamax-style unwind.
    while depth > 0 {
        depth -= 1;
        gain[depth] = -i32::max(-gain[depth], gain[depth + 1]);
    }

    gain[0]
}
```

**Algorithm summary**:
1. Start with the initial capture's material gain in `gain[0]`.
2. Loop: find the least valuable attacker for the opposing side. Record `gain[depth] = attacker_value_on_square - gain[depth-1]` (what the current side gains by capturing, minus what the opponent gained so far).
3. Remove the attacker from occupancy, which may reveal X-ray sliding pieces behind it. Only re-check diagonal X-rays when the removed piece moved on a diagonal (pawn, bishop, queen) and straight X-rays when the removed piece moved on a rank/file (rook, queen).
4. Continue alternating sides until no attackers remain or a king would capture into an attacked square.
5. Backward propagation: unwind from the end of the sequence. At each step, the side has the choice of not capturing (taking `-gain[depth-1]`) or capturing (taking `gain[depth]`). The minimax unwind selects the best outcome for each side.

### Step 3: Update `crates/search/src/ordering.rs`

#### 3a. Add SEE Import and New Constant

Add at the top of `ordering.rs`:
```rust
use crate::see;
```

Add a new constant for bad capture scoring:
```rust
const BAD_CAPTURE_BASE: i32 = -20_000;
```

`BAD_CAPTURE_BASE` is set to -20,000, which is below the history heuristic range (approximately `[-16384, 16384]`), ensuring bad captures always sort after all quiet moves (including history-scored ones).

#### 3b. Update `score_move` Capture Branch

Current code (line 54-55 in `ordering.rs`):
```rust
    } else if mv.is_capture() {
        CAPTURE_BASE + score_mvv_lva(mv, pos)
```

Replace with:
```rust
    } else if mv.is_capture() {
        let see_score = see::see(pos, mv);
        if see_score >= 0 {
            CAPTURE_BASE + score_mvv_lva(mv, pos)
        } else {
            BAD_CAPTURE_BASE + see_score
        }
```

**Rationale**: Good captures (SEE >= 0) retain their position above killers using the existing `CAPTURE_BASE` (100,000) + MVV-LVA score. MVV-LVA is kept for sub-ordering among good captures because it is cheaper than using raw SEE scores and produces a reasonable ordering. Bad captures (SEE < 0) are scored at `BAD_CAPTURE_BASE + see_score` (e.g., a SEE of -200 gets scored as -20,200), placing them below all quiet moves.

**Updated ordering hierarchy**:
| Priority | Score Range | Category |
|----------|------------|----------|
| 1 | 300,000 | TT move |
| 2 | 200,000 | PV move |
| 3 | 100,000+ | Good captures (SEE >= 0), sub-ordered by MVV-LVA |
| 4 | 20,000 | Killer moves |
| 5 | 15,000 | Counter-moves |
| 6 | ~[-16384, 16384] | History-ordered quiet moves |
| 7 | -20,000 and below | Bad captures (SEE < 0) |

### Step 4: Update `crates/search/src/lib.rs`

#### 4a. Add Module Declaration

Add after the existing module declarations (line 4, after `pub mod ordering;`):
```rust
pub mod see;
```

#### 4b. Bad Capture Pruning in `negamax`

The negamax move loop begins at line 349 of `lib.rs`. The current structure is:

```
line 349: for (moves_searched, mv) in ...
line 350-353: piece_kind computed from pos.piece_on(mv.from_sq())
line 354: let undo = pos.make_move(mv);
line 355: ctx.history.push(pos.hash());
line 357-360: is_tt_move, is_pv_move, is_killer, is_tactical computed
line 362-365: gives_check computed (requires position after make_move)
line 367-381: futility pruning (applies to non-tactical moves)
line 383-391: LMR conditions
```

**Modifications**: Add the SEE score computation *before* `make_move` (since SEE reads the position without mutating it), and add the bad capture pruning block *after* the `gives_check` computation and the existing futility pruning block.

The modified move loop structure:

```rust
    for (moves_searched, mv) in (0_u32..).zip(moves.into_iter()) {
        let piece_kind = pos
            .piece_on(mv.from_sq())
            .expect("piece must exist on from_sq")
            .kind;

        let is_tactical = mv.is_capture() || mv.is_promotion();

        // Pre-compute SEE for captures before make_move.
        let see_score = if mv.is_capture() && !mv.is_promotion() {
            see::see(pos, mv)
        } else {
            0
        };

        let undo = pos.make_move(mv);
        ctx.history.push(pos.hash());

        let is_tt_move = tt_move == Some(mv);
        let is_pv_move = pv_move == Some(mv);
        let is_killer = ctx.killers.is_killer(ply, mv);

        let gives_check = {
            let opp_king_sq = king_square(pos, pos.side_to_move());
            pos.is_square_attacked(opp_king_sq, pos.side_to_move().opposite())
        };

        // Futility pruning (existing, unchanged — applies to !is_tactical) ...

        // Bad capture pruning: skip captures with negative SEE
        // when the position is not critical.
        if mv.is_capture()
            && !mv.is_promotion()
            && !in_check
            && !gives_check
            && !is_tt_move
            && !is_pv_move
            && depth <= 3
            && see_score < 0
        {
            ctx.history.pop();
            pos.unmake_move(mv, undo);
            continue;
        }

        // LMR (existing, unchanged) ...
```

**Key changes from current code**:
- `is_tactical` is moved from after `make_move` (current line 360) to before `make_move`. This is safe because `mv.is_capture()` and `mv.is_promotion()` depend only on the `Move` value's flag bits, not the position state.
- `see_score` is computed before `make_move` using the position in its pre-move state, which is what the SEE function requires.
- The bad capture pruning block is inserted between the existing futility pruning block (lines 367-381) and the LMR block (lines 383+).

**Conditions explained**:
- `mv.is_capture() && !mv.is_promotion()`: Only prune plain captures, not promotion captures (which change material balance unpredictably).
- `!in_check`: Don't prune when in check — all evasions matter.
- `!gives_check`: Don't prune captures that give check — they may be tactically critical.
- `!is_tt_move && !is_pv_move`: Don't prune the TT or PV move — these are trusted ordering signals.
- `depth <= 3`: Only prune at shallow depths where the cost of a bad capture search is small relative to the risk of missing something.
- `see_score < 0`: The SEE evaluation says this capture loses material.

**Interaction with existing pruning**: Futility pruning applies to non-tactical moves (`!is_tactical`). Bad capture pruning applies to tactical captures with `!mv.is_promotion()`. These conditions are mutually exclusive, so there is no overlap or conflict.

### Step 5: Unit Tests in `crates/search/src/see.rs`

Add a `#[cfg(test)] mod tests` block at the bottom of `see.rs` with the 8 required test scenarios:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chess_board::Position;
    use chess_movegen::generate_legal_moves;

    /// Helper: find a capture move from `from_sq` to `to_sq`.
    fn find_capture(pos: &mut Position, from: Square, to: Square) -> Move {
        let moves = generate_legal_moves(pos);
        moves
            .into_iter()
            .find(|m| m.from_sq() == from && m.to_sq() == to && m.is_capture())
            .expect("expected capture move must exist")
    }

    #[test]
    fn see_pxn_winning() {
        // Pawn captures undefended knight. SEE = 320.
        let mut pos = Position::from_fen("4k3/8/4n3/3P4/8/8/8/4K3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::D5, Square::E6);
        assert_eq!(see(&pos, mv), 320);
    }

    #[test]
    fn see_nxb_defended_roughly_equal() {
        // Knight captures bishop defended by a pawn.
        // NxB (gain 330), PxN recapture (gain 320). SEE = 330 - 320 = 10.
        // Black bishop on e6, defended by black pawn on f7.
        let mut pos = Position::from_fen("4k3/5p2/4b3/4N3/8/8/8/4K3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::E5, Square::E6);
        assert_eq!(see(&pos, mv), 10);
    }

    #[test]
    fn see_bxr_winning() {
        // Bishop captures undefended rook. SEE = 500.
        // White bishop on g2, black rook on d5 (on the g2-d5 diagonal).
        let mut pos = Position::from_fen("4k3/8/8/3r4/8/8/6B1/4K3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::G2, Square::D5);
        assert_eq!(see(&pos, mv), 500);
    }

    #[test]
    fn see_rxq_winning() {
        // Rook captures undefended queen. SEE = 900.
        let mut pos = Position::from_fen("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::D1, Square::D5);
        assert_eq!(see(&pos, mv), 900);
    }

    #[test]
    fn see_pxn_with_recapture_by_knight() {
        // Pawn captures knight, second knight recaptures.
        // White pawn on d5, black knights on e6 and c7.
        // PxN on e6 (gain 320), Nc7xP on e6 (gain 100).
        // SEE = 320 - 100 = 220 (winning for initiator; see Deviation 2).
        let mut pos = Position::from_fen("4k3/2n5/4n3/3P4/8/8/8/4K3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::D5, Square::E6);
        assert_eq!(see(&pos, mv), 220);
    }

    #[test]
    fn see_undefended_piece_capture() {
        // Knight captures undefended rook — full piece value.
        // White knight on d4, black rook on e6.
        // Knight on d4 attacks e6 (d4 knight targets: c2,e2,b3,f3,b5,f5,c6,e6).
        let mut pos = Position::from_fen("4k3/8/4r3/8/3N4/8/8/4K3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::D4, Square::E6);
        assert_eq!(see(&pos, mv), 500);
    }

    #[test]
    fn see_multi_piece_exchange_losing() {
        // Multi-piece exchange: NxP on d5, defended by bishop f7 and queen a8.
        // White: knight e3, rook d1. Black: pawn d5, bishop f7, queen a8.
        // Sequence: NxP(100), BxN(320), RxB(330), QxR(500).
        // Backward propagation yields SEE = -220 (losing for initiator).
        let mut pos = Position::from_fen(
            "q3k3/5b2/8/3p4/8/4N3/8/3RK3 w - - 0 1"
        ).expect("valid fen");
        let mv = find_capture(&mut pos, Square::E3, Square::D5);
        assert!(see(&pos, mv) < 0, "multi-piece exchange should be losing for initiator");
    }

    #[test]
    fn see_capture_into_defended_pawn_chain() {
        // Knight captures pawn defended by another pawn — losing.
        // White knight on e3, black pawns on d5 and e6.
        // Black pawn on e6 defends d5. NxP(100), PxN(320). SEE = -220.
        let mut pos = Position::from_fen("4k3/8/4p3/3p4/8/4N3/8/4K3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::E3, Square::D5);
        assert!(see(&pos, mv) < 0, "capturing into defended pawn chain should lose material");
    }
}
```

Each test constructs a specific FEN with carefully placed pieces to produce the desired capture scenario. The `find_capture` helper locates the intended capture move from the legal move list. Tests assert either exact SEE values (for simple cases) or sign checks (for complex multi-piece scenarios).

## 4. Implementation Order

1. **Expose attack tables** in movegen crate (`knights.rs` line 36, `king.rs` line 36) — one keyword change each.
2. **Create `see.rs`** with the `see` function, helper functions (`pawn_attacks`, `attackers_to`, `least_valuable_attacker`), and all 8 unit tests. Run `cargo test -p chess-search see::tests` to validate.
3. **Update `ordering.rs`** — add `BAD_CAPTURE_BASE` constant, import `crate::see`, modify the capture branch in `score_move` (lines 54-55) to use SEE. Run `cargo test -p chess-search ordering::tests` to verify no regressions.
4. **Update `lib.rs`** — add `pub mod see;` (after line 4), move `is_tactical` computation before `make_move` (currently at line 360, move to before line 354), add `see_score` computation before `make_move`, add bad capture pruning block between the futility pruning block (lines 367-381) and the LMR block (lines 383+). Run `cargo test -p chess-search` to verify all tests pass.
5. **Run full validation**: `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check`.

## 5. Test Commands

```bash
# Run only the SEE tests
cargo test -p chess-search see::tests

# Run only the ordering tests (verify no regression)
cargo test -p chess-search ordering::tests

# Run all tests in the search crate
cargo test -p chess-search

# Run the full workspace tests
cargo test --workspace

# Lint checks
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --check
```

## 6. Design Deviations

**Deviation 1: Attack table visibility**
- **Parent spec requires**: SEE uses "bitboard attacker/defender sets for the target square" — implies the function computes attackers internally.
- **Issue**: The `KNIGHT_ATTACKS` and `KING_ATTACKS` precomputed tables in the movegen crate are private (`const` without `pub`). Duplicating these 64-entry tables in the search crate would be wasteful and violate DRY.
- **Alternative chosen**: Make `KNIGHT_ATTACKS` and `KING_ATTACKS` public in the movegen crate (adding `pub` to the existing `const` declarations in `crates/movegen/src/knights.rs` line 36 and `crates/movegen/src/king.rs` line 36). This is a minimal change (one keyword per file) that gives SEE efficient O(1) knight and king attack lookups without duplicating data. The `bishop_attacks` and `rook_attacks` functions in `magic.rs` are already public. Both modules are already `pub mod` in movegen's `lib.rs`.

**Deviation 2: Test scenario "PxN with NxP recapture (equal)"**
- **Parent spec says**: "PxN with NxP recapture (equal exchange)".
- **Issue**: A PxN capture where a knight recaptures the pawn yields SEE = 320 − 100 = 220 (winning for the initiator, not equal). A truly equal exchange would be, e.g., NxN or BxB. The spec labels this "equal" but the arithmetic gives 220.
- **Alternative chosen**: Implement the test as specified (PxN with recapture by a knight) and assert the mathematically correct SEE value of 220. The exchange is "equal" in the sense that both sides traded a piece, but the SEE score correctly reflects the material imbalance.

**Deviation 3: Test scenario "NxB (roughly equal)"**
- **Parent spec says**: "NxB — knight captures bishop (roughly equal, ~+10)".
- **Issue**: An undefended NxB yields SEE = 330 (full bishop value). To get ~+10, the bishop must be defended so that after NxB, the defender recaptures (capturing the knight worth 320), yielding 330 − 320 = 10.
- **Alternative chosen**: Set up the test with the bishop defended by a pawn, so the exchange is NxB, PxN, yielding SEE = 330 − 320 = 10 as the spec intends.