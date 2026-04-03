Good — no existing extension logic. Now I have everything I need. Here is the task plan:

## 1. Deliverables

- `crates/search/src/lib.rs` — modified to add check extension logic at the top of `negamax`, before any pruning decisions. When the side to move is in check, the search depth is incremented by one ply. Also adds a `check_extension_enabled` flag to `SearchContext` for testability (following the existing pattern of `lmr_enabled` and `futility_enabled`).
- Unit tests within `crates/search/src/lib.rs` (in the existing `#[cfg(test)] mod tests` block) — tests confirming check extension activates in check positions and a WAC solve-rate comparison test.

## 2. Dependencies

- No new crate dependencies required.
- Depends on existing `is_square_attacked` from `chess_board::Position` (M01/M02) for in-check detection (already used at line 250 of `lib.rs`).
- Depends on existing pruning guards from M06/P01 (null move pruning, LMR, futility pruning) already checking `in_check` — these are in place.
- Depends on existing move ordering from M06/P02 (history, counter-move, SEE) — already in place.

## 3. Implementation Details

### `SearchContext` — new field

Add `pub(crate) check_extension_enabled: bool` to the `SearchContext` struct (line 41), following the existing pattern of `lmr_enabled` (line 55) and `futility_enabled` (line 56). Default to `true` in the `search()` function (line 546) and in `test_ctx()` (line 600).

### `negamax` — check extension logic

Insert check extension logic **after** the `depth == 0` quiescence fallback (line 208–210) and **before** the TT probe section (line 216). The logic:

1. The `in_check` computation currently at line 248–251 needs to be moved earlier — to just after the `depth == 0` check and before the TT probe. This is because check extension must modify `depth` before TT probe depth comparison occurs.

2. After computing `in_check`, if `ctx.check_extension_enabled && in_check`, increment `depth` by 1 (capped to avoid overflow: `depth = depth.saturating_add(1)`). This extends the search by one ply for positions where the side to move is in check.

3. Remove the duplicate `in_check` computation at line 248–251 since it's now computed earlier.

The existing pruning guards already reference `in_check`:
- Null move pruning (line 253): `!in_check` — already correct.
- Reverse futility pruning (line 292): `!in_check` — already correct.
- Futility pruning (line 376–378): `!in_check` — already correct.
- LMR (line 406–414): `!in_check` — already correct.

These guards ensure that nodes where the side to move is in check skip all pruning. No changes needed to these guards.

### Unit tests

**Test 1: `check_extension_activates_in_check`**

Use a FEN position where the side to move is in check. Run `negamax` with `check_extension_enabled = true` and compare the node count against a run with `check_extension_enabled = false` at the same nominal depth. The extension-enabled run should search more nodes (since it extends the depth by 1 ply at check positions, exploring deeper). Use a position like `"rnbqkbnr/pppp1ppp/8/4p3/7q/5P2/PPPPP1PP/RNBQKBNR w KQkq - 0 2"` (White is in check from Qh4) at depth 4.

**Test 2: `check_extension_no_effect_when_not_in_check`**

Use a position where the side to move is NOT in check (e.g., startpos). Run `negamax` with `check_extension_enabled = true` and `false`. Node counts should be identical, confirming the extension only activates when in check.

**Test 3: `check_extension_improves_wac_solve_rate`**

WAC solve-rate comparison test. Use the same WAC positions array already defined in the existing `futility_preserves_wac_solve_rate` test (4 tactical positions). Run iterative deepening to depth 6 with check extension enabled vs disabled. Assert that the number of correctly solved positions with check extension enabled is greater than or equal to the number solved without it. This follows the exact pattern of the existing `futility_preserves_wac_solve_rate` test.

## 4. API Contracts

N/A — no new public API surface. The only change is internal to the `negamax` function and the `SearchContext` struct (which is `pub` but only used internally by `search()`). The `check_extension_enabled` field is `pub(crate)` for test access, matching the existing pattern.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| `check_extension_activates_in_check` | When the side to move is in check, check extension causes more nodes to be searched (depth is extended by 1 ply) |
| `check_extension_no_effect_when_not_in_check` | When the side to move is NOT in check, enabling/disabling check extension produces identical node counts |
| `check_extension_improves_wac_solve_rate` | Check extension does not reduce (and ideally improves) solve rate on WAC tactical positions at fixed depth compared to baseline without the extension |

All tests are colocated in the existing `#[cfg(test)] mod tests` block in `crates/search/src/lib.rs`, following project conventions. Tests use the existing `test_ctx()` helper and `SearchContext` construction pattern seen throughout the file.

## 6. Implementation Order

1. Add `pub(crate) check_extension_enabled: bool` field to `SearchContext` struct
2. Set `check_extension_enabled: true` in `search()` function and `test_ctx()` helper
3. Move `in_check` computation earlier in `negamax` (after `depth == 0` check, before TT probe)
4. Add check extension depth increment: `if ctx.check_extension_enabled && in_check { depth = depth.saturating_add(1); }`
5. Remove the now-duplicate `in_check` computation
6. Add test `check_extension_activates_in_check`
7. Add test `check_extension_no_effect_when_not_in_check`
8. Add test `check_extension_improves_wac_solve_rate`

## 7. Verification Commands

```bash
cargo test -p chess-search
cargo clippy -p chess-search -- -D warnings
cargo fmt --check -p chess-search
cargo test
```

## 8. Design Deviations

**Deviation: `in_check` computation moved before TT probe**

- **What the task spec requires**: "Check extension is applied at the top of the `negamax` function, before any pruning decisions."
- **Why the current placement requires adjustment**: In the existing code, `in_check` is computed at line 248 (after the TT probe at line 219). Check extension modifies `depth`, and since the TT probe at line 221 compares `entry.depth() >= depth`, the extension must happen before the TT probe to ensure the extended depth is used in the TT depth comparison. If check extension happened after the TT probe, a TT entry stored at the original depth could incorrectly satisfy the depth requirement and return a cached score that was computed without the extension.
- **Alternative chosen**: Move the `in_check` computation to just after the `depth == 0` quiescence fallback and before the TT probe. Apply the depth extension immediately. This ensures the TT probe sees the extended depth, which is correct behavior — a TT entry must have been searched at least as deep as the extended depth to be usable.