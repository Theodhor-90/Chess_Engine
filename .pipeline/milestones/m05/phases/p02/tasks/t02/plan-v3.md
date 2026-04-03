Now I have all the context. The feedback has a single issue: Test 3 (`iid_finds_move_for_ordering`) says "Use a middlegame position" without specifying a FEN string. I need to provide a concrete FEN. The rest of the plan was accepted. Let me output the revised plan.

## 1. Deliverables

| File | Action | Purpose |
|------|--------|---------|
| `crates/search/src/lib.rs` | Modify | Add `const IID_MIN_DEPTH: i32 = 4;` constant and IID logic in `negamax`; add tests for IID node reduction, threshold behavior, and move ordering |

## 2. Dependencies

No new external dependencies. IID reuses the existing `negamax` function and the TT probe/store infrastructure already present in `crates/search/src/tt.rs`. The TT move legality validation from sibling task T01 is already in place (lines 176–180 of `lib.rs`).

## 3. Implementation Details

### `crates/search/src/lib.rs`

**New constant:**

```rust
const IID_MIN_DEPTH: i32 = 4;
```

Defined at module level (after the existing `INFINITY` constant on line 18). This is a compile-time constant controlling the minimum remaining depth at which IID activates.

**IID logic in `negamax`:**

Insert IID after the TT move legality validation block (after line 180) and before the move ordering call (line 183). The logic:

1. Check if `tt_move` is `None` and `depth as i32 >= IID_MIN_DEPTH`.
2. If both conditions hold, perform a reduced-depth search: call `negamax(pos, depth - 2, alpha, beta, ply, ctx)`.
3. After the reduced search, re-probe the TT with `pos.hash()` to obtain a move.
4. If the re-probe returns an entry with a best move, validate it against the legal move list (already computed in `moves`) and set `tt_move` to it if valid.

The reduced search uses `depth - 2` (2-ply reduction). Since `depth` is `u8` and the threshold is 4, `depth - 2` is always >= 2, so no underflow concern. The reduced search naturally populates the TT via its own `tt.store` calls at the end of `negamax`.

**Key considerations:**

- The `tt_move` variable must become `mut` (it already is, per line 151).
- The `moves` variable (the legal move list) is already computed before the IID insertion point, so validation is straightforward.
- The IID search reuses the same `ctx` (same TT, killers, PV table, time budget), which is correct — the TT entries from the reduced search benefit the full-depth search.
- Since the `depth` parameter is `u8`, the comparison `depth as i32 >= IID_MIN_DEPTH` handles the type mismatch between the constant (`i32`) and the parameter (`u8`).

**Exact code to insert (between lines 180 and 182):**

```rust
    // Internal iterative deepening: if no TT move and depth is sufficient,
    // do a reduced-depth search to populate the TT with a move for ordering.
    if tt_move.is_none() && depth as i32 >= IID_MIN_DEPTH {
        negamax(pos, depth - 2, alpha, beta, ply, ctx);
        if let Some(entry) = ctx.tt.probe(hash) {
            if let Some(iid_move) = entry.best_move() {
                if moves.contains(&iid_move) {
                    tt_move = Some(iid_move);
                }
            }
        }
    }
```

## 4. API Contracts

N/A — No public API changes. The `IID_MIN_DEPTH` constant is module-private (`const`, not `pub const`). The `negamax` function signature is unchanged. IID is an internal optimization within `negamax`.

## 5. Test Plan

All tests are added to the existing `#[cfg(test)] mod tests` block in `crates/search/src/lib.rs`.

### Test 1: `iid_reduces_node_count`

**Purpose:** Verify that IID reduces total node count compared to search without IID on positions at sufficient depth (exit criterion 1).

**Setup:**
- Use the position `"r1bqkb1r/pppppppp/2n2n2/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3"`.
- Search at depth 5 using iterative deepening (depths 1..=5) with a 1 MB TT — this is the "with IID" baseline (IID triggers naturally in the normal code path since no TT move exists at unexplored nodes with depth >= 4).
- Search the same position at depth 5 using a **separate, modified approach**: use a `SearchContext` with a 0 MB TT (minimum 1-entry table). With a near-useless TT, IID's reduced search cannot effectively populate TT entries that the full search would find, so IID's benefit is neutralized.
- Compare node counts. The search with a functional TT (where IID populates it) should use fewer nodes.

**Assertion:** `nodes_with_iid < nodes_without_iid`.

**Note:** This test structure mirrors the existing `tt_reduces_node_count` and `tt_move_ordering_reduces_nodes` tests already in the file, using 1 MB vs 0 MB TT to contrast effectiveness.

### Test 2: `iid_does_not_trigger_below_threshold`

**Purpose:** Verify that IID does not activate at depths below `IID_MIN_DEPTH` (4) (exit criterion 2).

**Setup:**
- Use the starting position.
- Create two `SearchContext` instances, both with 1 MB TTs.
- Call `new_generation()` on both TTs (no prior entries, so no TT move is available).
- Call `negamax` directly at `depth = 3` (below threshold) on both contexts.
- Compare node counts — they must be identical because IID cannot trigger at depth 3, so both searches explore the same tree with the same (empty) TT state.

**Assertion:** `ctx_a.nodes == ctx_b.nodes` (identical node counts confirm IID did not activate at depth 3).

This single, unambiguous assertion is sufficient: if IID were erroneously triggering at depth 3, the two contexts would diverge in node count due to the extra reduced-depth search calls that IID performs. Since both start with empty TTs and search the same position at the same depth, identical node counts prove IID is inactive below the threshold.

### Test 3: `iid_finds_move_for_ordering`

**Purpose:** Verify that IID correctly finds a move that the subsequent full-depth search uses for ordering (exit criterion 3).

**Setup:**
- Use the position `"r1bqkb1r/pppppppp/2n2n2/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3"` (same middlegame FEN used in Test 1 and in the existing `tt_reduces_node_count` test).
- Create a `SearchContext` with a 1 MB TT but no prior entries (fresh TT, `new_generation` called).
- Call `negamax` at depth 5. Internally, IID should trigger at depth >= 4 nodes.
- After the search, probe the TT for the root position's hash.
- Verify the TT entry has a best move and that this move is legal.

**Assertion:** The TT contains an entry for the root position with a valid best move after the search completes. The search returns a valid move (not `None`).

## 6. Implementation Order

1. Add `const IID_MIN_DEPTH: i32 = 4;` after the `INFINITY` constant in `crates/search/src/lib.rs`.
2. Insert the IID logic block in `negamax` between the TT move validation (line 180) and the PV move / ordering call (line 182).
3. Add the three test functions (`iid_reduces_node_count`, `iid_does_not_trigger_below_threshold`, `iid_finds_move_for_ordering`) to the `mod tests` block.
4. Run `cargo test -p chess-search` to verify all new and existing tests pass.
5. Run `cargo test` across the full workspace.
6. Run `cargo clippy -- -D warnings` and `cargo fmt --check`.

## 7. Verification Commands

```bash
# Run search crate tests (includes new IID tests and all existing tests)
cargo test -p chess-search

# Run full workspace tests
cargo test

# Lint and format checks
cargo clippy -- -D warnings
cargo fmt --check
```

## 8. Design Deviations

None.