Good. Now I have all the context I need. The `magic` module is public from the movegen crate, so `chess_movegen::magic::bishop_attacks()` and `chess_movegen::magic::rook_attacks()` are accessible. Knight and king attacks are private. Let me now write the complete revised plan.

# T03 — Static Exchange Evaluation (SEE): Implementation Plan

## 1. Overview

This task implements Static Exchange Evaluation (SEE) for the chess engine's search crate. SEE evaluates whether a capture sequence on a given square is winning, equal, or losing by simulating alternating recaptures from least-valuable to most-valuable attacker. The result is integrated into two places: (1) move ordering in `ordering.rs`, where good captures (SEE >= 0) are scored above killers and bad captures (SEE < 0) below quiet moves; and (2) the search loop in `lib.rs`, where captures with negative SEE are pruned in non-critical nodes.

## 2. Deliverables

| # | File | Action | Description |
|---|------|--------|-------------|
| 1 | `crates/movegen/src/knights.rs` | Modify | Change `const KNIGHT_ATTACKS` to `pub const KNIGHT_ATTACKS` |
| 2 | `crates/movegen/src/king.rs` | Modify | Change `const KING_ATTACKS` to `pub const KING_ATTACKS` |
| 3 | `crates/search/src/see.rs` | Create | New module containing the `see` function, attack helper functions, and unit tests |
| 4 | `crates/search/src/ordering.rs` | Modify | Update `score_move` to use SEE for capture scoring; add `BAD_CAPTURE_BASE` constant |
| 5 | `crates/search/src/lib.rs` | Modify | Add `pub mod see;`, add bad capture pruning in `negamax`, add SEE-based delta pruning in `quiescence` |

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

**Rationale**: SEE needs O(1) knight and king attack lookups. The `bishop_attacks` and `rook_attacks` functions in `magic.rs` are already public. Duplicating the 64-entry const tables in the search crate would violate DRY. Adding `pub` is a one-keyword change per file.

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

This mirrors the shift logic in `crates/movegen/src/pawns.rs` (lines 62-84, 160-181). White pawns attack northwest (`<<7`) and northeast (`<<9`); black pawns attack southeast (`>>7`) and southwest (`>>9`). File masks prevent wrapping.

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

Note the pawn attack symmetry: to find white pawns that attack square `sq`, we compute where a black pawn on `sq` would attack (the reverse direction) and intersect with white pawns. This is the standard technique.

The key property for X-ray handling: `bishop_attacks` and `rook_attacks` take an `occupied` bitboard parameter. When we remove a piece from `occupied` during the swap loop, re-calling `attackers_to` with the updated occupancy will automatically reveal any sliding piece that was hidden behind the removed piece (X-ray attack).

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
    // The gain includes promotion value minus pawn value.
    if mv.is_promotion() {
        if let Some(promo_kind) = mv.promotion_piece() {
            attacker_value = SEE_VALUES[promo_kind as usize];
            // Initial gain includes the promotion bonus.
            // gain[0] = victim + (promo_value - pawn_value)
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

        // Stand-pat pruning: if the side to move can't improve their
        // position even by capturing, they won't continue.
        // This is: max(-gain[depth-1], gain[depth]) optimization.
        // If gain[depth] < 0 and -gain[depth-1] < 0, both options lose.
        // Prune if max(gain[depth], -gain[depth-1]) < 0 — but we handle
        // this in the backward propagation instead for simplicity.

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

        // If the attacker is a pawn, bishop, or queen, re-check diagonal X-rays.
        // If the attacker is a rook or queen, re-check straight X-rays.
        // We do this by recomputing all attackers with updated occupancy.
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
            // The side that just moved king cannot legally capture because
            // the opponent still attacks the square. Undo this depth.
            depth -= 1;
            break;
        }
    }

    // Backward propagation: negamax-style unwind.
    // gain[i] = max of (not capturing = -gain[i-1]) or (capturing = gain[i])
    // from the perspective of the side making capture i.
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

Current code (lines 54-55):
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

Add bad capture pruning in the move loop, after the existing futility pruning block (after line 381) and before the LMR block (line 383). The pruning goes after `make_move` because we already have `is_tactical` computed and need `gives_check`:

```rust
        // Bad capture pruning: skip captures with negative SEE
        // in non-critical situations.
        let do_bad_capture_prune = is_tactical
            && !mv.is_promotion()
            && !in_check
            && !gives_check
            && !is_tt_move
            && !is_pv_move
            && depth <= 3
            && see::see(pos_before_move, mv) < 0;

        if do_bad_capture_prune {
            ctx.history.pop();
            pos.unmake_move(mv, undo);
            continue;
        }
```

Wait — there is a complication. The current code structure computes `gives_check` *after* `make_move` (lines 354-365: the move is made at line 354, then `gives_check` is checked on the resulting position). The SEE function takes the position *before* the move. So we need the SEE call to use the position state before `make_move`.

**Revised approach**: Compute the SEE score for captures *before* `make_move`, storing it in a variable. Then use it in the pruning condition after `gives_check` is computed. The SEE function reads the position without mutating it, so this is safe.

The actual code placement in the move loop (around lines 349-381):

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

        // Futility pruning (existing, unchanged) ...

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

**Conditions explained**:
- `mv.is_capture() && !mv.is_promotion()`: Only prune plain captures, not promotion captures (which change material balance unpredictably).
- `!in_check`: Don't prune when in check — all evasions matter.
- `!gives_check`: Don't prune captures that give check — they may be tactically critical.
- `!is_tt_move && !is_pv_move`: Don't prune the TT or PV move — these are trusted ordering signals.
- `depth <= 3`: Only prune at shallow depths where the cost of a bad capture search is small relative to the risk of missing something. At deeper depths, the bad capture may lead to important positions.
- `see_score < 0`: The SEE evaluation says this capture loses material.

**Interaction with existing pruning**: Futility pruning applies to non-tactical moves (`!is_tactical`). Bad capture pruning applies to tactical captures. These conditions are mutually exclusive, so there is no overlap or conflict.

Note: `is_tactical` was previously computed at line 360 *after* `make_move`. We move it to *before* `make_move` since it only reads the move flags, not the position. This is purely a reordering — `mv.is_capture()` and `mv.is_promotion()` depend only on the `Move` value.

#### 4c. SEE-Based Delta Pruning in `quiescence`

In the `quiescence` function (lines 108-167), add SEE-based pruning to skip bad captures. After ordering the tactical moves (line 148) and before the loop (line 149):

In the existing loop body, add a check before `make_move`:

```rust
    for mv in tactical {
        // Delta pruning with SEE: skip captures that lose material
        // and cannot raise alpha even with the victim's value.
        if see::see(pos, mv) < 0 {
            continue;
        }

        let undo = pos.make_move(mv);
        // ... rest unchanged
```

This filters out losing captures in quiescence search, which is safe because:
1. Quiescence already has a stand-pat cutoff — the side can choose not to capture.
2. A losing capture (negative SEE) will almost certainly not improve the position above stand-pat.
3. This significantly reduces the number of nodes searched in quiescence.

### Step 5: Unit Tests in `crates/search/src/see.rs`

Add a `#[cfg(test)] mod tests` block at the bottom of `see.rs` with the 8 required test scenarios plus edge cases:

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
        // Test 1: PxN — pawn captures undefended knight. SEE = 320.
        let mut pos = Position::from_fen("4k3/8/4n3/3P4/8/8/8/4K3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::D5, Square::E6);
        assert_eq!(see(&pos, mv), 320);
    }

    #[test]
    fn see_nxb_defended_roughly_equal() {
        // Test 2: NxB where bishop is defended by a pawn.
        // NxB (gain 330), PxN (recapture, gain 320).
        // SEE = 330 - 320 = 10.
        let mut pos = Position::from_fen("4k3/8/3pb3/4N3/8/8/8/4K3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::E5, Square::E6);
        // Note: d6 pawn defends e6 (the bishop's square -- wait, d6 pawn 
        // for black attacks e5 not e6. Let's fix the position.)
        // Black pawn on d7 defends e6: (d7 pawn attacks e6 for black).
        // Actually: a black pawn on d7 attacks c6 and e6 (>>7 and >>9).
        // Wait — black pawns attack downward. d7 = index 51.
        // d7 >> 7 = index 44 = e5? No. Let me reconsider.
        // Black pawn on f7 attacks e6 and g6.
        // So: black bishop on e6, black pawn on f7 defending it.
        drop(pos);
        let mut pos = Position::from_fen("4k3/5p2/4b3/4N3/8/8/8/4K3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::E5, Square::E6);
        assert_eq!(see(&pos, mv), 10); // 330 - 320 = 10
    }

    #[test]
    fn see_bxr_winning() {
        // Test 3: BxR — bishop captures undefended rook. SEE = 500.
        let mut pos = Position::from_fen("4k3/8/4r3/8/8/8/6B1/4K3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::G2, Square::E4);
        // Actually bishop on g2 can't reach e6 directly. Let me use 
        // a proper diagonal. g2 bishop attacks along a8-h1 and a8-h1 
        // diagonals. g2 -> f3 -> e4 -> d5 -> c6... or h3.
        // Let's put the rook on d5.
        drop(pos);
        let mut pos = Position::from_fen("4k3/8/8/3r4/8/8/6B1/4K3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::G2, Square::D5);
        assert_eq!(see(&pos, mv), 500);
    }

    #[test]
    fn see_rxq_winning() {
        // Test 4: RxQ — rook captures undefended queen. SEE = 900.
        let mut pos = Position::from_fen("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::D1, Square::D5);
        assert_eq!(see(&pos, mv), 900);
    }

    #[test]
    fn see_pxn_with_recapture_by_knight() {
        // Test 5: PxN with NxP recapture.
        // White pawn on d5 captures black knight on e6.
        // Black knight on g7 recaptures on e6 (wait, needs another 
        // knight that can reach e6).
        // Black knight on c7 can reach e6? No: c7 knight goes to 
        // a6, a8, b5, d5, e6, e8. Yes — c7 knight attacks e6.
        // So: PxN (gain 320), NxP (gain 100). SEE = 320 - 100 = 220.
        // The spec says "equal exchange" but the arithmetic gives 220 
        // (see Deviation 2).
        let mut pos = Position::from_fen("4k3/2n5/4n3/3P4/8/8/8/4K3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::D5, Square::E6);
        assert_eq!(see(&pos, mv), 220);
    }

    #[test]
    fn see_undefended_piece_capture() {
        // Test 6: Capturing an undefended piece — full piece value.
        // White knight captures undefended black rook.
        let mut pos = Position::from_fen("4k3/8/8/4r3/3N4/8/8/4K3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::D4, Square::E6);
        // Wait, d4 knight attacks e6? Knight on d4: attacks are
        // c2, e2, b3, f3, b5, f5, c6, e6. Yes.
        assert_eq!(see(&pos, mv), 500);
    }

    #[test]
    fn see_multi_piece_exchange_losing() {
        // Test 7: Multi-piece exchange on a single square.
        // Sequence: PxN, BxP, RxB, QxR — net losing for initiator.
        // White: pawn on d5, rook on d1.
        // Black: knight on e6, bishop on c8 (defends e6 via diagonal),
        //        queen on a6 (defends e6 via diagonal? No.)
        // Let me construct carefully:
        // Target square: e6. 
        // White pawn d5 captures black knight e6.
        // Black bishop recaptures on e6 (needs a bishop that attacks e6).
        // White rook recaptures on e6 (needs rook on e-file or 6th rank).
        // Black queen recaptures on e6.
        // Gains: [320, 100, 330, 500]
        // gain[0] = 320 (PxN)
        // gain[1] = 100 - 320 = -220 (BxP: capture pawn worth 100, minus what white gained)
        // Wait, the gain array works differently. Let me trace:
        // gain[0] = 320 (knight value)
        // attacker_value = 100 (pawn, now on e6)
        // depth=1: gain[1] = 100 - 320 = -220, attacker_value = 330 (bishop)
        // depth=2: gain[2] = 330 - (-220) = 550, attacker_value = 500 (rook)
        // depth=3: gain[3] = 500 - 550 = -50, no more attackers
        // Backward: gain[2] = -max(-550, -50) = -max(-550, -50) = -(-50) = 50
        // gain[1] = -max(-(-220), 50) = -max(220, 50) = -220
        // gain[0] = -max(-320, -220) = -max(-320, -220) = -(-220) = 220
        // Hmm, that gives 220. Let me reconsider the scenario.
        //
        // Better scenario: White pawn captures well-defended piece.
        // Target: d5. Black knight on d5.
        // White: pawn e4, nothing else nearby.
        // Black: bishop on b7 (attacks d5), rook on d8 (attacks d5), 
        //        queen somewhere.
        // PxN: gain 320. Now white pawn (100) on d5.
        // BxP: gain 100. Now black bishop (330) on d5.
        // No more white attackers → stop.
        // gain[0]=320, gain[1]=100-320=-220
        // Back: gain[0] = -max(-320, -220) = 220. Still winning.
        //
        // For a truly losing exchange, the initiator must lose more than
        // they gain. E.g., NxP where the pawn is very well defended.
        // White knight captures black pawn on d5.
        // Black has: bishop on c6 (attacks d5? bishop on c6 attacks d5 
        // diagonally... c6 bishop attacks b5, a4, d7, e8, b7, a8, d5 — 
        // yes if on the c6-d5 diagonal... c6 to d5 is one step diag, 
        // but is it? c6=(2,5), d5=(3,4). Diff=(1,-1). Yes, diagonal.)
        // Sequence: NxP (gain 100), BxN (gain 320).
        // gain[0]=100, gain[1]=320-100=220. 
        // Back: gain[0] = -max(-100, 220) = -220. 
        // So SEE = -220 for the initiator. That's losing!
        //
        // Let's add more pieces for a longer chain:
        // White: knight on e3, rook on d1.
        // Black: pawn on d5, bishop on f7 (attacks d5? f7->e6->d5: 
        // two steps, but bishop slides. f7 bishop attacks d5 if no 
        // pieces between. f7=(5,6), d5=(3,4). Diff=(2,2). Diagonal. 
        // Yes.), queen on a8 (attacks d5 via diagonal: a8->d5? 
        // a8=(0,7), d5=(3,4). Diff=(3,-3). Yes, diagonal.)
        //
        // NxP (100), BxN (320), RxB (330), QxR (500).
        // gain[0]=100, attacker_val=320 (knight, now on d5)
        // depth=1: gain[1]=320-100=220, attacker_val=330 (bishop)
        // depth=2: gain[2]=330-220=110, attacker_val=500 (rook)
        // depth=3: gain[3]=500-110=390, no more white attackers, 
        //          but black queen on a8. Switch to black:
        // depth=3: after white rook captures, side=black, 
        //          find black queen, gain[3]=500-110=390, 
        //          attacker_val=900 (queen)
        // depth=4: side=white, no white attackers. Stop.
        // Back: gain[3]=390
        // gain[2] = -max(-110, 390) = -390
        // gain[1] = -max(-220, -390) = 220
        // gain[0] = -max(-100, 220) = -220
        // SEE = -220. Losing for white (the initiator).
        
        let mut pos = Position::from_fen(
            "q3k3/5b2/8/3p4/8/4N3/8/3RK3 w - - 0 1"
        ).expect("valid fen");
        let mv = find_capture(&mut pos, Square::E3, Square::D5);
        assert!(see(&pos, mv) < 0, "multi-piece exchange should be losing for initiator");
    }

    #[test]
    fn see_capture_into_defended_pawn_chain() {
        // Test 8: Capture into a defended pawn chain — losing.
        // White knight captures black pawn on d5, but d5 is defended 
        // by black pawn on e6.
        // NxP: gain 100. Pawn (320 knight now on d5) recaptured by pawn.
        // PxN: gain 320. 
        // gain[0]=100, gain[1]=320-100=220.
        // Back: gain[0] = -max(-100, 220) = -220.
        let mut pos = Position::from_fen("4k3/8/4p3/3p4/8/4N3/8/4K3 w - - 0 1")
            .expect("valid fen");
        let mv = find_capture(&mut pos, Square::E3, Square::D5);
        assert!(see(&pos, mv) < 0, "capturing into defended pawn chain should lose material");
    }
}
```

**Note on test positions**: Each test constructs a specific FEN with carefully placed pieces to produce the desired capture scenario. The `find_capture` helper locates the intended capture move from the legal move list. Tests assert either exact SEE values (for simple cases) or sign checks (for complex multi-piece scenarios where exact values depend on position details).

## 4. Implementation Order

1. **Expose attack tables** in movegen crate (`knights.rs`, `king.rs`) — one keyword change each.
2. **Create `see.rs`** with the `see` function, helper functions (`pawn_attacks`, `attackers_to`, `least_valuable_attacker`), and all 8 unit tests. Run `cargo test -p chess-search see::tests` to validate.
3. **Update `ordering.rs`** — add `BAD_CAPTURE_BASE` constant, import `crate::see`, modify the capture branch in `score_move` to use SEE. Run `cargo test -p chess-search ordering::tests` to verify no regressions.
4. **Update `lib.rs`** — add `pub mod see;`, add bad capture pruning in `negamax` (with SEE computed before `make_move`), add SEE filtering in `quiescence`. Run `cargo test -p chess-search` to verify all tests pass.
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
- **Alternative chosen**: Make `KNIGHT_ATTACKS` and `KING_ATTACKS` public in the movegen crate (adding `pub` to the existing `const` declarations in `crates/movegen/src/knights.rs` line 36 and `crates/movegen/src/king.rs` line 36). This is a minimal change (one keyword per file) that gives SEE efficient O(1) knight and king attack lookups without duplicating data. The `bishop_attacks` and `rook_attacks` functions in `magic.rs` are already public.

**Deviation 2: Test scenario "PxN with NxP recapture (equal)"**
- **Parent spec says**: "PxN with NxP recapture (equal exchange)".
- **Issue**: A PxN capture where a knight recaptures the pawn yields SEE = 320 − 100 = 220 (winning for the initiator, not equal). A truly equal exchange would be, e.g., NxN or BxB. The spec labels this "equal" but the arithmetic gives 220.
- **Alternative chosen**: Implement the test as specified (PxN with recapture by a knight) and assert the mathematically correct SEE value of 220. The exchange is "equal" in the sense that both sides traded a piece, but the SEE score correctly reflects the material imbalance.

**Deviation 3: Test scenario "NxB (roughly equal)"**
- **Parent spec says**: "NxB — knight captures bishop (roughly equal, ~+10)".
- **Issue**: An undefended NxB yields SEE = 330 (full bishop value). To get ~+10, the bishop must be defended so that after NxB, the defender recaptures (capturing the knight worth 320), yielding 330 − 320 = 10.
- **Alternative chosen**: Set up the test with the bishop defended by a pawn, so the exchange is NxB, PxN, yielding SEE = 330 − 320 = 10 as the spec intends.