I now have all the information needed to produce the task plan. Here is the complete plan:

## 1. Deliverables

- `crates/nnue/src/accumulator.rs` (modify) — Add `AccumulatorState` struct with dirty flags and feature delta tracking; add `AccumulatorStack` for managing accumulator state across the search tree with push/pop semantics; modify `Accumulator` to support lazy incremental recomputation.
- `crates/nnue/src/lib.rs` (modify) — Re-export new public types (`AccumulatorState`, `AccumulatorStack`).
- `crates/search/src/lib.rs` (modify) — Replace `Accumulator` field in `SearchContext` with `AccumulatorStack`; update `eval_position` to use lazy evaluation from the stack; insert accumulator push/pop calls around `make_move`/`unmake_move` in `negamax` and `quiescence`.

## 2. Dependencies

- No new crate dependencies required.
- Depends on existing `crates/nnue` types: `Accumulator`, `Network`, `feature_index`, `forward`, `L1_SIZE`.
- Depends on existing `crates/board` types: `Position`, `UndoInfo`, `Piece`, `PieceKind`, `Square`.
- Depends on existing `crates/types`: `Color`, `Move`, `MoveFlag`.

## 3. Implementation Details

### `crates/nnue/src/accumulator.rs`

**`FeatureDelta` struct** (new, internal to module):
```rust
struct FeatureDelta {
    adds: ArrayVec<(Color, usize), 4>,    // (perspective, feature_index) pairs to add
    removes: ArrayVec<(Color, usize), 4>, // (perspective, feature_index) pairs to remove
}
```
Use fixed-capacity inline arrays (max 4 adds and 4 removes covers the worst case: castling moves two pieces per perspective, each with an add and a remove). Since `arrayvec` is an external dependency and the project minimizes dependencies, implement as `[(Color, usize); 4]` with a `len: u8` counter instead.

**`AccumulatorState` struct** (new, public):
```rust
pub struct AccumulatorState {
    pub acc: Accumulator,
    dirty: [bool; 2],  // dirty[0] = white perspective, dirty[1] = black perspective
    king_moved: bool,   // if king moved, require full refresh for that perspective
}
```
- `dirty` flags track whether each perspective needs recomputation.
- `king_moved` indicates a king move occurred, requiring full refresh (since king square changes affect all HalfKP feature indices for that perspective).
- Methods:
  - `new() -> Self` — creates clean state with zeroed accumulator, not dirty.
  - `from_accumulator(acc: Accumulator) -> Self` — wraps an already-computed accumulator.
  - `mark_dirty(&mut self, perspective: Color)` — sets dirty flag for a perspective.
  - `mark_king_moved(&mut self)` — sets king_moved flag.
  - `is_dirty(&self, perspective: Color) -> bool` — checks dirty flag.
  - `needs_refresh(&self) -> bool` — returns true if king_moved is set.

**`AccumulatorStack` struct** (new, public):
```rust
pub struct AccumulatorStack {
    stack: Vec<AccumulatorState>,
}
```
Manages a stack of accumulator states corresponding to the search tree's make/unmake operations.

Methods:
- `new() -> Self` — creates stack with one initial entry.
- `current(&self) -> &AccumulatorState` — returns the top of the stack (current position's state).
- `current_mut(&mut self) -> &mut AccumulatorState` — mutable access to top.
- `push_move(&mut self, pos: &Position, mv: Move, network: &Network)` — called after `pos.make_move(mv)`. Clones the current top, then computes the feature deltas for the move and applies them to the new top. For king moves, marks `king_moved` and defers full refresh. For non-king moves, computes add/remove deltas based on the move type and applies them incrementally.
- `pop(&mut self)` — called after `pos.unmake_move(mv, undo)`. Pops the top entry, restoring the previous accumulator state. This is exact restoration (not recomputation).
- `refresh_if_needed(&mut self, pos: &Position, network: &Network)` — if the current top needs a full refresh (king moved or not yet initialized), performs `acc.refresh(pos, network)` and clears dirty/king_moved flags.
- `evaluate(&mut self, pos: &Position, network: &Network, side_to_move: Color) -> i32` — ensures the accumulator is up-to-date (calls `refresh_if_needed` if king moved, otherwise accumulator is already correct from incremental updates), then calls `forward(&self.current().acc, network, side_to_move)`.

**Feature delta computation** (private helper in `accumulator.rs`):

`fn compute_and_apply_deltas(state: &mut AccumulatorState, pos_before: &Position, pos_after: &Position, mv: Move, captured: Option<Piece>, network: &Network)`

This function determines what features changed due to the move:

1. **Moving piece**: The piece on `mv.from_sq()` moves to `mv.to_sq()`.
   - For each perspective where king did NOT move: remove feature for (king_sq, from_sq, kind, color), add feature for (king_sq, to_sq, kind, color).

2. **Captured piece** (regular capture): Remove feature for (king_sq, to_sq, captured_kind, captured_color) for each perspective.

3. **En passant**: The captured pawn is on a different square than `to_sq`. Remove feature for the captured pawn's actual square.

4. **Promotion**: Remove pawn feature at from_sq, add promoted piece feature at to_sq (instead of adding pawn at to_sq).

5. **Castling**: King moves (triggers `king_moved`; full refresh deferred) + rook moves from its home square to its destination. Both perspectives need the rook delta applied (but since king moved, both will get full refresh anyway).

6. **King move** (including castling): Sets `king_moved = true`. Since all HalfKP features are king-relative, the entire accumulator for that perspective must be recomputed. Mark dirty and defer to `refresh_if_needed`.

Key insight: When the moving side's king moves, ALL feature indices for that perspective change (since king_sq is part of every index). A full refresh is unavoidable for that perspective. However, the opponent's perspective only needs deltas for the pieces that moved/were captured. When the king does NOT move, both perspectives get cheap incremental updates.

**Modifications to existing `Accumulator`**: None. The existing `Accumulator` struct and its `add_feature`, `remove_feature`, `refresh` methods remain unchanged. The new `AccumulatorState` and `AccumulatorStack` wrap `Accumulator` and add the delta/dirty logic on top.

### `crates/nnue/src/lib.rs`

Add re-exports:
```rust
pub use accumulator::{AccumulatorStack, AccumulatorState};
```

### `crates/search/src/lib.rs`

**`SearchContext` modification**: Replace `accumulator: Accumulator` with `acc_stack: AccumulatorStack`.

**`eval_position` modification**:
```rust
fn eval_position(pos: &Position, ctx: &mut SearchContext) -> i32 {
    if ctx.eval_mode == EvalMode::Nnue {
        if let Some(ref network) = ctx.network {
            return ctx.acc_stack.evaluate(pos, network, pos.side_to_move());
        }
    }
    chess_eval::evaluate(pos, &mut ctx.pawn_table)
}
```

**`negamax` and `quiescence` modification**: At every `pos.make_move(mv)` call, add `ctx.acc_stack.push_move(pos, mv, network)` immediately after (while the captured piece info is still available from `UndoInfo`). At every `pos.unmake_move(mv, undo)`, add `ctx.acc_stack.pop()` immediately after.

The captured piece is available from `undo.captured`. The push_move call needs the position state *after* the move, plus the move and captured piece. Sequence:
```rust
let undo = pos.make_move(mv);
if let Some(ref network) = ctx.network {
    ctx.acc_stack.push_move(pos, mv, undo.captured, network);
}
// ... recursive search ...
if let Some(ref _network) = ctx.network {
    ctx.acc_stack.pop();
}
pos.unmake_move(mv, undo);
```

Note: `push_move` takes `undo.captured` as a parameter to know what piece was captured, since the position after `make_move` no longer has the captured piece on the board.

**Initialization**: When search begins, initialize the `AccumulatorStack` with a single entry refreshed from the root position:
```rust
ctx.acc_stack = AccumulatorStack::new();
ctx.acc_stack.refresh_if_needed(&pos, &network);
```

## 4. API Contracts

```rust
// AccumulatorState — wraps Accumulator with dirty tracking
let state = AccumulatorState::new();
let state = AccumulatorState::from_accumulator(acc);
state.mark_dirty(Color::White);
state.is_dirty(Color::White); // true

// AccumulatorStack — manages accumulator across search tree
let mut stack = AccumulatorStack::new();
stack.refresh_if_needed(&pos, &network);  // full refresh at root
let score = stack.evaluate(&pos, &network, Color::White);  // returns i32

let undo = pos.make_move(mv);
stack.push_move(&pos, mv, undo.captured, &network);
// ... recursive search ...
stack.pop();
pos.unmake_move(mv, undo);
```

N/A for REST/HTTP.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| `incremental_matches_full_refresh_quiet_move` | After a quiet move, incrementally updated accumulator produces identical values to full refresh on the resulting position |
| `incremental_matches_full_refresh_capture` | After a capture, incrementally updated accumulator matches full refresh |
| `incremental_matches_full_refresh_en_passant` | After an en passant capture, incrementally updated accumulator matches full refresh |
| `incremental_matches_full_refresh_castling` | After castling (both king-side and queen-side), accumulator matches full refresh (via deferred king-moved refresh) |
| `incremental_matches_full_refresh_promotion` | After a promotion (with and without capture), accumulator matches full refresh |
| `stack_push_pop_restores_state` | After push_move + pop, the accumulator state is identical to before the push (verifying exact unmake restoration) |
| `incremental_chain_matches_refresh` | Apply a sequence of 10+ make_move/push_move calls, then compare final accumulator to a fresh full refresh on the same position — must be identical |
| `dirty_flag_tracks_correctly` | Dirty flags set/clear correctly on make/unmake for king and non-king moves |
| `existing_nnue_tests_pass` | All existing tests in `crates/nnue` continue to pass unchanged |

All tests are colocated in `crates/nnue/src/accumulator.rs` via `#[cfg(test)] mod tests`, consistent with project convention. The chain test uses positions from the starting position with a known move sequence. Tests use `Network::new_zeroed()` with deterministic non-zero weights (same pattern as existing `accumulator_refresh_startpos` test).

## 6. Implementation Order

1. **`crates/nnue/src/accumulator.rs`** — Define `AccumulatorState` struct with dirty flags and `king_moved` flag. Implement constructors and flag methods.
2. **`crates/nnue/src/accumulator.rs`** — Define `AccumulatorStack` struct with `push_move`, `pop`, `refresh_if_needed`, and `evaluate` methods. Implement feature delta computation for all move types (quiet, capture, en passant, promotion, castling, king moves).
3. **`crates/nnue/src/lib.rs`** — Add re-exports for `AccumulatorStack` and `AccumulatorState`.
4. **`crates/nnue/src/accumulator.rs`** — Write all tests verifying incremental matches full refresh for each move type, push/pop restoration, and multi-move chain correctness.
5. **`crates/search/src/lib.rs`** — Replace `accumulator: Accumulator` with `acc_stack: AccumulatorStack` in `SearchContext`. Update initialization.
6. **`crates/search/src/lib.rs`** — Update `eval_position` to use `acc_stack.evaluate()`.
7. **`crates/search/src/lib.rs`** — Insert `push_move`/`pop` calls around all `make_move`/`unmake_move` sites in `negamax` and `quiescence`.
8. **Verify** — Run all tests, clippy, and fmt.

## 7. Verification Commands

```bash
cargo test -p chess-nnue
cargo test -p chess-search
cargo clippy --workspace -- -D warnings
cargo fmt --check
cargo test --workspace
```

## 8. Design Deviations

**Deviation 1: King moves trigger full refresh instead of incremental delta**

- **Parent spec requires**: "only recompute affected features on make/unmake" and "applying the minimal delta (feature add/subtract) rather than a full recomputation."
- **Why pure incremental is problematic for king moves**: In HalfKP, every feature index includes the king square as a dimension (`king_sq * 640 + ...`). When the king moves, ALL feature indices for that perspective change — every active feature must be removed at the old king square and re-added at the new king square. This is equivalent to a full refresh (iterate all pieces, compute new indices) and is not a "minimal delta" — it touches every active feature. Attempting to track all old indices and all new indices doubles the work compared to a single `refresh()` call.
- **Alternative chosen**: For king moves, set `king_moved = true` and defer to `refresh_if_needed()`, which performs a full `Accumulator::refresh()` for the affected perspective only. Non-king moves use true incremental deltas. This is the standard approach used by Stockfish and all major NNUE implementations. The opponent's perspective still gets cheap incremental updates even when the moving side's king moves.

**Deviation 2: `push_move` signature takes `captured: Option<Piece>` as a parameter**

- **Parent spec states**: "On `make_move`, mark the affected side's accumulator dirty and record the feature additions and removals."
- **Why the spec's hook model is impractical**: The task spec suggests integration points with `crates/board` (make/unmake hooks). However, `crates/board` has no dependency on `crates/nnue` (and adding one would create a circular dependency since `crates/nnue` depends on `crates/board`). The board crate is intentionally NNUE-agnostic.
- **Alternative chosen**: The accumulator stack is managed at the search layer (`crates/search`), which already depends on both `crates/board` and `crates/nnue`. The `push_move` call is placed immediately after `pos.make_move(mv)` in the search code, receiving the captured piece from `UndoInfo.captured`. This preserves the clean crate dependency graph while achieving the same incremental update behavior.