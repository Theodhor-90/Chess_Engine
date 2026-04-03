Now I have a comprehensive understanding of the codebase. Let me produce the implementation plan.

## 1. Deliverables

| File | Action | Purpose |
|------|--------|---------|
| `crates/search/src/lib.rs` | **Modify** | Add LMR reduction table (pre-computed 2D array), integrate LMR logic into the `negamax` move loop, add unit tests for LMR correctness and node reduction benchmarks |

## 2. Dependencies

No new crate dependencies. LMR uses only `f64::ln()` and `f64::floor()` from the standard library, which are already available. The existing dependencies in `crates/search/Cargo.toml` (`chess-types`, `chess-board`, `chess-movegen`, `chess-eval`) are sufficient.

## 3. Implementation Details

### `crates/search/src/lib.rs`

#### LMR Reduction Table

- Define constants `const LMR_MAX_DEPTH: usize = 64;` and `const LMR_MAX_MOVES: usize = 64;`.
- Define a static 2D array `LMR_TABLE: [[u8; LMR_MAX_MOVES]; LMR_MAX_DEPTH]` initialized at compile time or via a `once_cell`/`std::sync::OnceLock` lazy init.
  - Since `f64::ln()` is not `const fn` in stable Rust, use `std::sync::OnceLock<[[u8; LMR_MAX_MOVES]; LMR_MAX_DEPTH]>` with a helper function `lmr_table() -> &'static [[u8; LMR_MAX_MOVES]; LMR_MAX_DEPTH]` that initializes the table on first access.
  - Formula per cell: `table[d][m] = floor(ln(d as f64) * ln(m as f64) / 1.75) as u8`.
  - For `d == 0` or `m == 0`, set reduction to 0 (ln(0) is undefined; these indices represent depth 0 or move index 0 which are not reduced).

#### LMR Logic in `negamax`

Insert LMR logic inside the existing `for mv in moves` loop at line 262, modifying the search call for each move. The integration point is between the `pos.make_move(mv)` and the `negamax(...)` recursive call.

**Move loop changes** (pseudocode structure):

```rust
let mut moves_searched: u32 = 0;

for mv in moves {
    let undo = pos.make_move(mv);
    ctx.history.push(pos.hash());

    let mut score;

    // Determine if this move qualifies for LMR
    let is_tt_move = tt_move == Some(mv);
    let is_pv_move = pv_move == Some(mv);
    let is_killer = ctx.killers.is_killer(ply, mv);
    let is_tactical = mv.is_capture() || mv.is_promotion();

    // Check if the move gives check (needed to exclude check-giving moves from LMR)
    let gives_check = {
        let opp_king_sq = king_square(pos, pos.side_to_move());
        pos.is_square_attacked(opp_king_sq, pos.side_to_move().opposite())
    };

    let do_lmr = depth >= 3
        && moves_searched >= 3
        && !is_tt_move
        && !is_pv_move
        && !is_killer
        && !is_tactical
        && !in_check      // side to move was not in check (already computed above null move)
        && !gives_check;  // move does not give check

    if do_lmr {
        let table = lmr_table();
        let d = (depth as usize).min(LMR_MAX_DEPTH - 1);
        let m = (moves_searched as usize).min(LMR_MAX_MOVES - 1);
        let reduction = table[d][m].max(1); // at least 1 ply reduction

        // Reduced-depth search with null window
        let reduced_depth = if depth as i32 - 1 - reduction as i32 > 0 {
            (depth - 1 - reduction) as u8
        } else {
            0
        };
        let (s, _) = negamax(pos, reduced_depth, -alpha - 1, -alpha, ply + 1, true, ctx);
        score = -s;

        // Re-search at full depth if the reduced search failed high (score > alpha)
        if score > alpha {
            let (s, _) = negamax(pos, depth - 1, -beta, -alpha, ply + 1, true, ctx);
            score = -s;
        }
    } else {
        let (s, _) = negamax(pos, depth - 1, -beta, -alpha, ply + 1, true, ctx);
        score = -s;
    }

    ctx.history.pop();
    pos.unmake_move(mv, undo);

    // ... rest of alpha/beta update logic unchanged ...

    moves_searched += 1;
}
```

**Key design points:**

1. **`moves_searched` counter**: Tracks how many moves have been searched at this node. LMR only applies when `moves_searched >= 3` (i.e., the first 3 moves are searched at full depth).

2. **`gives_check` detection**: After `make_move`, the position has the opponent to move. Check if the opponent's king is attacked by the side that just moved: `pos.is_square_attacked(opp_king_sq, pos.side_to_move().opposite())`. Note that after make_move, `pos.side_to_move()` is the opponent, and `pos.side_to_move().opposite()` is the side that just played.

3. **`in_check`**: Already computed earlier in `negamax` for the null move pruning section (line 197–200). Reuse this variable.

4. **Minimum reduction**: The LMR table may produce 0 for small depth/move_index values. Use `.max(1)` to ensure at least 1 ply of reduction when LMR is applied.

5. **Reduced depth floor**: If `depth - 1 - reduction` would be negative, clamp to 0 (goes directly to quiescence).

6. **Re-search window**: The reduced search uses a null window `(-alpha - 1, -alpha)`. If it fails high (score > alpha), a full-window re-search at full depth `(depth - 1)` is performed with the `(-beta, -alpha)` window.

## 4. API Contracts

N/A — LMR is an internal search optimization. No new public API surface is added. The `negamax` function signature remains unchanged. The `lmr_table()` function is module-private.

## 5. Test Plan

### Test 1: `lmr_table_values_correct`

**Purpose**: Verify the pre-computed LMR reduction table produces expected values for known (depth, move_index) pairs.

**Setup**: Call `lmr_table()` and check specific entries.

**Specification**:
- `table[0][*]` should be 0 for all move indices (depth 0 has no reductions).
- `table[*][0]` should be 0 for all depths (move index 0 has no reduction).
- `table[6][4]` = `floor(ln(6) * ln(4) / 1.75)` = `floor(1.7918 * 1.3863 / 1.75)` = `floor(1.419)` = 1.
- `table[10][10]` = `floor(ln(10) * ln(10) / 1.75)` = `floor(2.3026 * 2.3026 / 1.75)` = `floor(3.031)` = 3.

### Test 2: `lmr_reduces_node_count`

**Purpose**: Verify LMR achieves >=30% node count reduction at depth 10 on standard benchmark positions compared to baseline without LMR.

**Setup**: Run iterative deepening searches to depth 10 on standard positions (startpos, Kiwipete, additional perft positions) with LMR enabled (normal code path) vs LMR disabled. To disable LMR, use a direct search call where the `moves_searched` counter effectively never triggers LMR (or use a mechanism such as setting depth limit to prevent LMR conditions). The simplest approach: since LMR is gated by `depth >= 3 && moves_searched >= 3`, we can compare against a baseline that existed before LMR was added. A practical approach is to search with `max_depth = 10` using `search()` and compare node counts.

**Implementation strategy for disabling LMR**: Add an `lmr_enabled: bool` field to `SearchContext` (default `true`). The LMR condition adds `&& ctx.lmr_enabled`. In the test, set it to `false` for the baseline run. This is a minimal addition needed for testability.

**Specification**: 
- For each position, compute `reduction_pct = 1.0 - (nodes_with_lmr / nodes_without_lmr)`.
- Assert `reduction_pct >= 0.30` for each position.

### Test 3: `lmr_skips_tt_moves`

**Purpose**: Verify LMR does not reduce TT moves, PV moves, killers, captures, or promotions.

**Setup**: This is validated structurally by the condition checks in the code. A practical test: search a position at depth >=3 where the first few moves include a TT move and killers; verify search finds the same best move as without LMR (no regression).

**Specification**: Search tactical positions (Scholar's mate Qxf7#, back-rank mate Qd8#) with and without LMR. Best move must be identical.

### Test 4: `lmr_re_search_on_fail_high`

**Purpose**: Verify that when a reduced search returns a score above alpha, a full-depth re-search is performed, preserving correctness.

**Setup**: Compare search results (best move and score) with LMR enabled vs disabled on tactical positions at depth 6.

**Specification**: Best move must match on tactical positions. Score must be within a small tolerance (re-search ensures correctness, so exact match is expected for forced mate positions).

## 6. Implementation Order

1. **Add `lmr_enabled` field to `SearchContext`** — Add `lmr_enabled: bool` to the struct definition, set to `true` in the `search()` function constructor, set to `true` in `test_ctx()`. This allows tests to toggle LMR.

2. **Implement `lmr_table()` function** — Create the lazy-initialized 2D reduction table using `std::sync::OnceLock`. Place it near the top of `lib.rs` alongside other constants.

3. **Integrate LMR into the `negamax` move loop** — Add the `moves_searched` counter, the `gives_check` computation inside the move loop, the LMR condition check, the reduced-depth search, and the re-search on fail-high. Modify the existing single search call into the conditional LMR/full-depth branching structure.

4. **Add unit tests** — Add `lmr_table_values_correct`, `lmr_reduces_node_count`, `lmr_skips_tt_moves` / correctness on tactical positions, and `lmr_re_search_on_fail_high` tests in the existing `#[cfg(test)] mod tests` block.

## 7. Verification Commands

```bash
# Format check
cargo fmt --check --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml

# Lint check
cargo clippy --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml -- -D warnings

# Run all tests (including new LMR tests)
cargo test --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml

# Run only search crate tests for faster iteration
cargo test --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml -p chess-search

# Run specific LMR tests
cargo test --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml -p chess-search lmr

# Run the node reduction benchmark test (may be slow at depth 10)
cargo test --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml -p chess-search lmr_reduces_node_count -- --nocapture
```

## 8. Design Deviations

**Deviation 1: `lmr_enabled` field on `SearchContext`**

- **Parent spec requires**: LMR logic in `negamax` with a benchmark test confirming >=30% node reduction at depth 10.
- **Why deviation is needed**: To measure node reduction with/without LMR, we need a way to disable LMR while keeping everything else identical. Without a toggle, the only alternative is comparing against a pre-LMR git revision, which is impractical in a unit test.
- **Alternative chosen**: Add a single `lmr_enabled: bool` field to `SearchContext`, defaulting to `true`. The LMR condition adds `&& ctx.lmr_enabled`. This is a minimal, non-invasive change that enables the required benchmark test. The field is `pub(crate)` and only used in tests.

**Deviation 2: `gives_check` detection approach**

- **Parent spec requires**: Do not reduce moves that give check, but does not specify how to detect check.
- **Why this matters**: There is no existing `gives_check(mv)` function in the codebase. Computing it requires making the move and then checking if the opponent's king is attacked.
- **Alternative chosen**: After `pos.make_move(mv)`, compute `gives_check` by calling `pos.is_square_attacked(opp_king_sq, pos.side_to_move().opposite())`. This is done inside the move loop after `make_move` but before the search call, since the move has already been applied. This reuses existing infrastructure without adding new functions.