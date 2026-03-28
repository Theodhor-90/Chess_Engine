I now have a complete understanding of the codebase. Let me produce the implementation plan.

## 1. Deliverables

| File | Action | Purpose |
|------|--------|---------|
| `crates/search/src/lib.rs` | **Modify** | Add reverse futility pruning (node-level, before move loop), futility pruning (per-move, within move loop), margin constants, `futility_enabled` flag on `SearchContext`, static eval caching at top of `negamax`, and unit tests for futility correctness and WAC solve-rate validation |

## 2. Dependencies

No new external dependencies. The implementation uses the existing `chess_eval::evaluate` function from `crates/eval/` (already a dependency of `chess-search` in `Cargo.toml`).

## 3. Implementation Details

### `crates/search/src/lib.rs`

#### Constants

Add margin constants near the top of the file alongside existing constants:

```rust
const FUTILITY_MARGINS: [i32; 4] = [0, 200, 400, 600];
const REVERSE_FUTILITY_MARGINS: [i32; 4] = [0, 100, 300, 500];
```

Index 0 is unused (depth 0 goes to quiescence). Indices 1–3 correspond to depths 1–3.

#### `SearchContext` field

Add a `pub(crate) futility_enabled: bool` field to `SearchContext`, mirroring the existing `lmr_enabled` pattern. Default to `true` in both `search()` and `test_ctx()`.

#### Static eval caching in `negamax`

After the `in_check` computation and null move pruning block, but before the move generation, compute the static eval once:

```rust
let static_eval = if !in_check { chess_eval::evaluate(pos) } else { 0 };
```

When in check, static eval is unreliable and both futility techniques are disabled, so the value doesn't matter.

#### Reverse futility pruning (node-level)

Insert after static eval computation, before IID and move generation. Conditions:
- `ctx.futility_enabled`
- `!in_check`
- `depth <= 3`
- `depth >= 1` (always true since depth 0 goes to quiescence)
- `beta.abs() < MATE_SCORE - MAX_PLY` (don't prune near mate scores)

Logic:
```rust
if ctx.futility_enabled
    && !in_check
    && depth <= 3
    && beta.abs() < MATE_SCORE - MAX_PLY
{
    let margin = REVERSE_FUTILITY_MARGINS[depth as usize];
    if static_eval - margin >= beta {
        return (static_eval, None);
    }
}
```

This prunes the entire node before generating or searching any moves.

#### Futility pruning (per-move, within the move loop)

Inside the `for (moves_searched, mv)` loop, after computing `gives_check` and before the LMR/search decision, add a futility pruning check. This determines whether to skip a quiet move:

```rust
let do_futility = ctx.futility_enabled
    && depth <= 3
    && !in_check
    && !gives_check
    && !is_tactical
    && alpha.abs() < MATE_SCORE - MAX_PLY;

if do_futility {
    let margin = FUTILITY_MARGINS[depth as usize];
    if static_eval + margin <= alpha {
        ctx.history.pop();
        pos.unmake_move(mv, undo);
        continue;
    }
}
```

Key details:
- The move has already been made at this point (because `gives_check` detection requires it), so we must `unmake_move` before `continue`.
- Only skip quiet moves (not captures or promotions — `!is_tactical`).
- Only skip when not in check and the move doesn't give check.
- Do not apply when alpha is near mate scores.
- `static_eval` is the eval of the position *before* the move (computed once at node top).

**Important consideration**: The current loop structure makes the move *before* checking `gives_check`. The futility skip must happen after `make_move` and `gives_check`, so the unmake is needed. This matches the existing structure where LMR also evaluates `gives_check` post-make_move.

#### Ensuring `best_move` is not `None` after futility skips

If futility pruning skips all moves at a node, `best_move` would remain `None`. However, this cannot happen in practice because:
1. Futility pruning only applies at `depth <= 3` and only to quiet, non-check moves.
2. If a position has only quiet moves and all are futility-pruned, we need a fallback. To handle this edge case, track whether any move was actually searched. If all moves were skipped, return `static_eval` (which is the best estimate at a futility-pruned node).

Add a `let mut any_searched = false;` before the loop, set it to `true` when a move is actually searched, and after the loop:

```rust
if best_move.is_none() && any_searched == false && !moves_were_empty {
    // All quiet moves were futility-pruned; return static_eval
    return (alpha, None);
}
```

Actually, re-examining: since `moves.is_empty()` is already handled above (returning mate/stalemate score), and captures/promotions are never futility-pruned, this edge case only arises if the position has *only* quiet non-check moves and all are pruned. In that case `alpha` was never updated, so returning `(alpha, None)` is correct — the existing code path after the loop already handles this correctly because `alpha` remains `original_alpha` and the TT store uses `BoundType::UpperBound`.

No additional fallback code is needed. The existing post-loop logic handles this naturally.

## 4. API Contracts

N/A — no new public API is introduced. The `futility_enabled` field is `pub(crate)` following the `lmr_enabled` convention and is only used for testing.

## 5. Test Plan

All tests are added in the existing `#[cfg(test)] mod tests` block in `crates/search/src/lib.rs`.

### Test 1: `futility_margins_match_spec`

Verify the constant arrays match the specified values:
- `FUTILITY_MARGINS[1] == 200`, `[2] == 400`, `[3] == 600`
- `REVERSE_FUTILITY_MARGINS[1] == 100`, `[2] == 300`, `[3] == 500`

### Test 2: `reverse_futility_prunes_node`

Set up a position where White has a massive material advantage (e.g., White has queen + pieces, Black has only king and pawns). Search at depth 2 with a narrow beta window. Compare node counts with `futility_enabled = true` vs `futility_enabled = false`. The futility-enabled search should produce fewer nodes, confirming reverse futility pruning is activating.

### Test 3: `futility_skips_quiet_moves`

Similar to Test 2 but focused on per-move futility pruning. Use a position where the static eval is far below alpha. Search at depth 1 with a high alpha. With `futility_enabled = true`, the search should skip quiet moves and produce fewer nodes than with `futility_enabled = false`.

### Test 4: `futility_does_not_prune_in_check`

Set up a position where the side to move is in check. Compare node counts with `futility_enabled = true` vs `false`. They must be equal, confirming futility pruning is disabled in check.

### Test 5: `futility_does_not_prune_captures`

Set up a position at shallow depth where captures exist. Verify that the best move found with futility enabled matches the best move found without it on tactical positions (e.g., Scholar's mate, back-rank mate). This confirms captures and promotions are never pruned.

### Test 6: `futility_preserves_wac_solve_rate`

Run a set of WAC (Win At Chess) tactical positions at a fixed depth (e.g., depth 8) with futility enabled and compare results to futility disabled. Verify the solve rate (number of correct best moves found) is not reduced. Use at least 5–10 well-known WAC positions with known best moves. The positions and expected target squares:

- `2rr3k/pp3pp1/1nnqbN1p/3pN3/2pP4/2P3Q1/PPB4P/R4RK1 w - -` (Qg6 or similar tactical blow)
- `r1b1k2r/ppppnppp/2n2q2/2b5/3NP3/2P1B3/PP3PPP/RN1QKB1R w KQkq -` (standard development/tactic)
- Additional known WAC positions

For each position, assert that `search(futility_enabled=true)` returns the same best move as `search(futility_enabled=false)`.

### Test 7: `futility_interacts_correctly_with_nmp_and_lmr`

Search a standard position (startpos) at depth 6 with all three pruning techniques enabled. Verify the search completes without panic and returns a legal move. Then compare with each technique individually disabled to ensure the combined path doesn't crash or produce invalid moves.

## 6. Implementation Order

1. Add `FUTILITY_MARGINS` and `REVERSE_FUTILITY_MARGINS` constant arrays
2. Add `futility_enabled: bool` field to `SearchContext`
3. Update `search()` function to set `futility_enabled: true` in `SearchContext` initialization
4. Update `test_ctx()` to set `futility_enabled: true`
5. Add static eval computation (`let static_eval = ...`) after the `in_check` and null move pruning block in `negamax`
6. Add reverse futility pruning block after static eval, before IID/move generation
7. Add per-move futility pruning inside the move loop, after `gives_check` computation, before LMR logic
8. Add unit tests (Tests 1–7)
9. Run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` to verify

## 7. Verification Commands

```bash
# Format check
cargo fmt --check --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml

# Lint check
cargo clippy --workspace --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml -- -D warnings

# Run all tests
cargo test --workspace --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml

# Run only search crate tests
cargo test --package chess-search --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml

# Run specific futility tests
cargo test --package chess-search --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml futility

# Run specific test with output for debugging
cargo test --package chess-search --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml futility -- --nocapture
```

## 8. Design Deviations

**Deviation 1: Futility pruning applied at depth ≤ 3 (not depth ≤ 1)**

- **Parent spec states**: "At frontier nodes (depth ≤ 1), if the static eval plus the futility margin (depth 1: 200 cp) is below alpha, skip quiet moves (futility pruning)."
- **However, the same spec also states**: The margin table includes entries for depths 1, 2, and 3 (200/400/600 cp), and says "Apply futility pruning only to quiet moves at depth ≤ 3."
- **Resolution**: The spec is internally inconsistent — the text says "depth ≤ 1" but the margin table and later text both say "depth ≤ 3". The margin table with three entries (depths 1–3) is the more detailed specification. I apply per-move futility pruning at depth ≤ 3 using all three margin values, which is the standard implementation in chess engines and matches the margin table provided.

**Deviation 2: Per-move futility check happens after `make_move`**

- **Parent spec states**: Futility pruning skips quiet moves before searching them.
- **Why adjusted**: The current `negamax` structure computes `gives_check` by making the move first and then checking if the opponent's king is attacked. Since the spec requires that moves giving check must not be pruned, the futility check must happen after `make_move` (to know `gives_check`), and then `unmake_move` + `continue` if the move qualifies for pruning. This is semantically equivalent to "skip the move" but mechanistically requires make/unmake for the check detection. This is the standard approach in chess engines.