Now I have all the context needed. Let me produce the implementation plan.

## 1. Deliverables

- `crates/board/src/position.rs` — **modified** to add `Position::make_null_move` and `Position::unmake_null_move` methods, plus a `has_non_pawn_material` helper; unit tests for null move correctness and round-trip hash consistency
- `crates/search/src/lib.rs` — **modified** to add null move pruning logic within the `negamax` function; unit tests validating activation in normal positions and deactivation in zugzwang-prone/check/mate-score/consecutive-null-move scenarios

## 2. Dependencies

- No new crate dependencies required.
- Requires existing: `chess_board::Position` (board crate), `chess_types::{Color, Piece, PieceKind, Bitboard}` (types crate), `crate::zobrist` (within board crate).
- Requires existing search infrastructure: `negamax`, `SearchContext`, `MATE_SCORE`, `INFINITY`, `TranspositionTable`, killer table, PV table, move ordering.

## 3. Implementation Details

### `crates/board/src/position.rs` — New Methods

#### `has_non_pawn_material(&self, color: Color) -> bool`

Returns `true` if the given side has any knights, bishops, rooks, or queens. Checks the union of piece bitboards for those four piece kinds for the given color:

```rust
pub fn has_non_pawn_material(&self, color: Color) -> bool {
    let n = self.piece_bitboard(Piece::new(color, PieceKind::Knight));
    let b = self.piece_bitboard(Piece::new(color, PieceKind::Bishop));
    let r = self.piece_bitboard(Piece::new(color, PieceKind::Rook));
    let q = self.piece_bitboard(Piece::new(color, PieceKind::Queen));
    !(n | b | r | q).is_empty()
}
```

#### `make_null_move(&mut self) -> NullMoveUndo`

Performs a null move (pass): flips side to move, clears en passant square, updates Zobrist hash incrementally. Does **not** move any piece, does **not** update halfmove clock or fullmove counter (irrelevant for null move search). Returns a `NullMoveUndo` struct to restore state.

```rust
#[derive(Clone, Copy, Debug)]
pub struct NullMoveUndo {
    pub en_passant: Option<Square>,
    pub hash: u64,
}

pub fn make_null_move(&mut self) -> NullMoveUndo {
    let undo = NullMoveUndo {
        en_passant: self.en_passant,
        hash: self.hash,
    };

    // Clear en passant
    if let Some(ep_sq) = self.en_passant {
        self.hash ^= crate::zobrist::en_passant_key(ep_sq.file());
        self.en_passant = None;
    }

    // Toggle side to move
    self.side_to_move = self.side_to_move.opposite();
    self.hash ^= crate::zobrist::side_to_move_key();

    undo
}
```

#### `unmake_null_move(&mut self, undo: NullMoveUndo)`

Restores the position after a null move by toggling side to move back and restoring en passant and hash from the undo struct:

```rust
pub fn unmake_null_move(&mut self, undo: NullMoveUndo) {
    self.side_to_move = self.side_to_move.opposite();
    self.en_passant = undo.en_passant;
    self.hash = undo.hash;
}
```

### `crates/search/src/lib.rs` — Null Move Pruning in `negamax`

#### New constant

Add `MAX_PLY` constant (matching the value used in `tt.rs`):

```rust
const MAX_PLY: i32 = 128;
```

#### New parameter

Add an `allow_null: bool` parameter to `negamax` to prevent consecutive null move searches. The signature becomes:

```rust
pub fn negamax(
    pos: &mut Position,
    depth: u8,
    mut alpha: i32,
    beta: i32,
    ply: u8,
    allow_null: bool,
    ctx: &mut SearchContext,
) -> (i32, Option<Move>)
```

All existing call sites must be updated:
- The recursive call in the main move loop: pass `allow_null: true`
- The IID call: pass `allow_null: false` (IID is a reduced search, null move is unnecessary)
- The `search` function's initial call: pass `allow_null: true`

#### Null move pruning block

Insert after TT probing (after line 192) and before move generation (line 194). The block is placed after TT probing because a TT cutoff is cheaper and should be tried first:

```rust
// Null move pruning
let in_check = {
    let king_sq = king_square(pos, pos.side_to_move());
    pos.is_square_attacked(king_sq, pos.side_to_move().opposite())
};

if allow_null
    && !in_check
    && depth >= 3
    && pos.has_non_pawn_material(pos.side_to_move())
    && beta.abs() < MATE_SCORE - MAX_PLY
{
    let r: u8 = if depth <= 6 { 2 } else { 3 };
    let null_undo = pos.make_null_move();
    ctx.history.push(pos.hash());
    let (null_score, _) = negamax(
        pos,
        depth - 1 - r,
        -beta,
        -beta + 1,
        ply + 1,
        false, // no consecutive null moves
        ctx,
    );
    let null_score = -null_score;
    ctx.history.pop();
    pos.unmake_null_move(null_undo);

    if ctx.aborted {
        return (0, None);
    }

    if null_score >= beta {
        return (beta, None);
    }
}
```

The `depth >= 3` guard ensures enough remaining depth for the reduced search to be meaningful (with R=2, we'd search at depth 0 if depth were 2, which is just qsearch — not useful as a null move probe).

The `in_check` value is computed once and reused. It is also needed later for checkmate/stalemate detection (currently computed implicitly via empty move list + king attacked check), but the existing logic already handles that case separately.

## 4. API Contracts

**`Position::make_null_move(&mut self) -> NullMoveUndo`**
- Input: Mutable reference to a `Position`
- Output: `NullMoveUndo { en_passant: Option<Square>, hash: u64 }`
- Side effects: Flips `side_to_move`, clears `en_passant`, updates `hash` incrementally

**`Position::unmake_null_move(&mut self, undo: NullMoveUndo)`**
- Input: Mutable reference to a `Position`, the `NullMoveUndo` returned by `make_null_move`
- Output: None
- Side effects: Restores `side_to_move`, `en_passant`, `hash` to pre-null-move state

**`Position::has_non_pawn_material(&self, color: Color) -> bool`**
- Input: Immutable reference to a `Position`, a `Color`
- Output: `true` if the given side has any knights, bishops, rooks, or queens

**`negamax` updated signature:**
```rust
pub fn negamax(
    pos: &mut Position, depth: u8, alpha: i32, beta: i32,
    ply: u8, allow_null: bool, ctx: &mut SearchContext,
) -> (i32, Option<Move>)
```

N/A for REST/HTTP.

## 5. Test Plan

### Board crate tests (`crates/board/src/position.rs`)

| Test | What it verifies |
|------|-----------------|
| `null_move_flips_side_to_move` | `make_null_move` toggles `side_to_move` from White to Black (and vice versa) |
| `null_move_clears_en_passant` | `make_null_move` clears `en_passant` when it was set (e.g., after a double pawn push) |
| `null_move_preserves_pieces` | After `make_null_move`, all piece bitboards and occupancy are unchanged |
| `null_move_hash_consistency` | After `make_null_move`, the incremental hash matches `compute_hash` recomputation |
| `null_move_round_trip` | `unmake_null_move` restores side_to_move, en_passant, hash to exact pre-null-move values |
| `null_move_round_trip_with_en_passant` | Round-trip test when en passant was set before the null move |
| `has_non_pawn_material_startpos` | Returns `true` for both colors at startpos |
| `has_non_pawn_material_king_and_pawns_only` | Returns `false` for a side with only king and pawns |
| `has_non_pawn_material_single_knight` | Returns `true` when a side has king, pawns, and one knight |

### Search crate tests (`crates/search/src/lib.rs`)

| Test | What it verifies |
|------|-----------------|
| `null_move_pruning_activates` | At sufficient depth in a position with non-pawn material and not in check, search explores fewer nodes than without null move pruning (compare node counts by running with `allow_null: true` vs `allow_null: false`) |
| `null_move_skipped_in_check` | When the side to move is in check, null move pruning does not activate (node count matches baseline without NMP) |
| `null_move_skipped_king_pawns_only` | In a position where the side to move has only king and pawns, null move pruning does not activate |
| `null_move_no_consecutive` | The `allow_null: false` parameter prevents null move pruning from firing in the recursive call |
| `null_move_finds_correct_bestmove` | On a set of tactical positions, the engine with null move pruning finds the same best move as without it (correctness not harmed) |

## 6. Implementation Order

1. `NullMoveUndo` struct in `crates/board/src/position.rs`
2. `has_non_pawn_material` method on `Position`
3. `make_null_move` method on `Position`
4. `unmake_null_move` method on `Position`
5. Board crate unit tests for the above methods
6. `MAX_PLY` constant in `crates/search/src/lib.rs`
7. Add `allow_null: bool` parameter to `negamax` signature; update all call sites (`search`, IID, recursive call in move loop)
8. Null move pruning conditional block in `negamax` (after TT probing, before move generation)
9. Search crate unit tests for null move pruning behavior

## 7. Verification Commands

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test -p chess-board
cargo test -p chess-search
cargo test
```

## 8. Design Deviations

**Deviation: `allow_null` parameter instead of implicit tracking**

The phase spec (Design Decision 8) states: "No new modules are needed — the pruning logic is local to the search loop." Adding an `allow_null` parameter to `negamax` is consistent with this — no new modules are created. However, the task spec mentions "no consecutive null moves" as a disable condition. The cleanest way to enforce this is an explicit `allow_null: bool` parameter on `negamax`, which requires updating the function signature and all call sites. An alternative would be storing a `last_was_null` flag on `SearchContext`, but this is more error-prone (must be set/unset correctly across recursive calls and IID). The parameter approach is standard in chess engine implementations and is the simplest correct solution.

**Deviation: `depth >= 3` minimum depth guard**

The task spec does not explicitly specify a minimum depth for null move pruning, but the phase spec specifies R=2 when `depth <= 6` and R=3 when `depth > 6`. At `depth < 3` with R=2, the null move search depth would be 0 or negative, making the probe meaningless (it would just be a quiescence search). The `depth >= 3` guard ensures the null move search has at least 1 ply of real search depth, which is standard practice in chess engines.