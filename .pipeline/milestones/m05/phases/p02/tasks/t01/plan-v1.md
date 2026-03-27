I now have all the context needed. Here is the task plan:

## 1. Deliverables

- `crates/search/src/lib.rs` — Modified: add TT move legality validation before passing to `order_moves` in `negamax`; add tests verifying TT move ordering correctness, invalid TT move discard, and node reduction from TT move ordering

## 2. Dependencies

- No new dependencies required.
- **Existing dependencies used**: `chess-movegen` (for `generate_legal_moves` to validate TT moves), `chess-board` (`Position`, `hash()`), `chess-types` (`Move`), `chess-search` internal modules (`tt`, `ordering`, `killer`, `pv_table`)

## 3. Implementation Details

### `crates/search/src/lib.rs` — TT Move Legality Validation

**Location**: Inside `negamax`, between the TT probe block (lines 153–171) and the call to `order_moves` (line 175).

**Current code** (lines 151–175):
```rust
let mut tt_move: Option<Move> = None;

if let Some(entry) = ctx.tt.probe(hash) {
    tt_move = entry.best_move();
    // ... TT cutoff logic ...
}

let mut moves = chess_movegen::generate_legal_moves(pos);
let pv_move = ctx.pv_move_at(ply);
ordering::order_moves(&mut moves, pos, &ctx.killers, ply, pv_move, tt_move);
```

**Change**: After the TT probe block assigns `tt_move`, and after the legal move list is generated, validate the TT move by checking if it appears in the `moves` list. If the TT move is not present in the legal moves, set `tt_move = None` to discard it before passing to `order_moves`.

```rust
let mut moves = chess_movegen::generate_legal_moves(pos);

// Validate TT move: discard if not legal (hash collision may produce invalid move)
if let Some(tm) = tt_move {
    if !moves.contains(&tm) {
        tt_move = None;
    }
}

let pv_move = ctx.pv_move_at(ply);
ordering::order_moves(&mut moves, pos, &ctx.killers, ply, pv_move, tt_move);
```

The validation uses `Vec::contains` which compares `Move` by `PartialEq` (derived on the `Move(u16)` newtype, so exact 16-bit match). This correctly handles all move types — quiet, captures, promotions, castling, en passant — because the TT stores the full `Move(u16)` and the legal move generator produces the same encoding for the same move.

### `crates/search/src/lib.rs` — Tests

Three new tests in the existing `#[cfg(test)] mod tests` block:

**Test (a): `tt_move_ordering_across_positions`** — Verify the TT move is ordered first across varied positions (captures, quiet moves, promotions). Set up 3 positions: one where the TT best move is a capture, one where it is a quiet move, one where it involves a promotion. For each, run a search to populate the TT, then retrieve the TT entry, generate legal moves, order them with the TT move, and assert `moves[0] == tt_move`.

**Test (b): `invalid_tt_move_discarded`** — Verify an invalid TT move from a hash collision is discarded. Manually store a TT entry for a position's hash with a `best_move` that is not legal in that position (simulating a hash collision). Then run `negamax` on the position and confirm it does not crash, returns a valid move, and the invalid TT move is not the first move searched. Specifically: create a position, fabricate a `Move` from two squares that is not in the legal move list (e.g., a move from a square with no piece), store it as a TT entry with the position's hash, then call `negamax` and assert a valid result is returned.

**Test (c): `tt_move_ordering_reduces_nodes`** — Verify TT move ordering produces fewer nodes than ordering without TT move. Use a test position at depth 5 with iterative deepening. Compare two contexts: one with a 1 MB TT (normal ordering uses TT move) and one with a 0 MB TT (minimal table, TT move ordering is ineffective). Assert the TT-enabled search uses fewer nodes. This test is structurally similar to the existing `tt_reduces_node_count` test but specifically attributes the reduction to TT move ordering by using the same position with a meaningful depth.

## 4. API Contracts

N/A — no public API changes. The `negamax` function signature is unchanged. The TT move validation is an internal refinement within `negamax`.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| `tt_move_ordering_across_positions` | TT move is ordered first across varied positions (captures, quiet moves, promotions) |
| `invalid_tt_move_discarded` | Invalid TT move from hash collision is discarded, search still returns a valid result |
| `tt_move_ordering_reduces_nodes` | TT move ordering produces fewer nodes searched than ordering without the TT move |

All three tests are added to the existing `#[cfg(test)] mod tests` block in `crates/search/src/lib.rs`. They use the existing `test_ctx()` helper and the same patterns established by existing tests (e.g., `tt_reduces_node_count`, `tt_move_is_ordered_first`).

## 6. Implementation Order

1. Add TT move legality validation in `negamax` (between move generation and `order_moves` call)
2. Add test `tt_move_ordering_across_positions`
3. Add test `invalid_tt_move_discarded`
4. Add test `tt_move_ordering_reduces_nodes`
5. Run `cargo test -p chess-search` to verify all tests pass
6. Run `cargo test` across the full workspace

## 7. Verification Commands

```bash
cargo test -p chess-search
cargo test
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

## 8. Design Deviations

None.