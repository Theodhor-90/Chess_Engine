Now I have all the context I need. Let me produce the revised plan.

## 1. Deliverables

- `crates/search/src/history.rs` — `HistoryTable` struct with `update()`, `score()`, and `clear()` methods, indexed by `[side][from_square][to_square]`, implementing the gravity formula for score updates
- `crates/search/src/ordering.rs` (modified) — Updated `score_move` and `order_moves` functions to accept a `HistoryTable` reference and `Color` parameter; use history scores for quiet move ordering; `KILLER_SCORE` raised to 20,000
- `crates/search/src/lib.rs` (modified) — New `pub mod history;` declaration, `HistoryTable` field added to `SearchContext`, search loop updated to call `history.update()` on beta cutoffs (positive bonus for cutoff move, negative bonus for previously-searched quiet moves)

## 2. Dependencies

- No new crate dependencies required. The `HistoryTable` uses only `chess_types::{Color, Square}` (already imported in `lib.rs`) and standard library types.
- Depends on existing `SearchContext`, move ordering infrastructure (`ordering.rs`), and the search loop in `lib.rs` from M03/M05/M06-P01.

## 3. Implementation Details

### `crates/search/src/history.rs`

**Struct definition:**
```rust
use chess_types::{Color, Square};

pub struct HistoryTable {
    table: [[[i32; Square::COUNT]; Square::COUNT]; Color::COUNT],
}
```
- `Color::COUNT` = 2, `Square::COUNT` = 64 → 2 × 64 × 64 = 8,192 entries of `i32` → ~32 KB.
- All entries initialized to `0`.

**Methods:**
- `pub fn new() -> HistoryTable` — returns a zeroed table.
- `pub fn update(&mut self, side: Color, from: Square, to: Square, bonus: i32)` — applies the gravity formula: `entry = entry - entry * bonus.abs() / 16384 + bonus`. The caller computes `bonus = depth * depth` for cutoff moves and `bonus = -(depth * depth)` for non-cutoff quiet moves.
- `pub fn score(&self, side: Color, from: Square, to: Square) -> i32` — returns the stored score for the given `(side, from, to)` triple.
- `pub fn clear(&mut self)` — resets all entries to zero.

**Implement `Default` for `HistoryTable`** to delegate to `new()` (matching the `KillerTable` pattern in `killer.rs`).

**Unit tests (colocated `#[cfg(test)] mod tests`):**
- `gravity_formula_bounds_scores` — repeatedly call `update` with large positive bonus (e.g., `bonus = 100`) many times and verify the score stays within `[-16384, 16384]`. Do the same with large negative bonus.
- `clear_resets_all` — store several values, call `clear()`, verify all entries return 0.
- `initial_score_is_zero` — create a new table, verify `score()` returns 0 for arbitrary entries.
- `positive_bonus_increases_score` — call `update` with a positive bonus, verify `score()` returns a positive value.
- `negative_bonus_decreases_score` — call `update` with a negative bonus, verify `score()` returns a negative value.

### `crates/search/src/ordering.rs` (modifications)

**Add import:** `use crate::history::HistoryTable;` and `use chess_types::Color;`

**Raise `KILLER_SCORE`:** Change `const KILLER_SCORE: i32 = 50;` to `const KILLER_SCORE: i32 = 20_000;`. This ensures killers always rank above history-scored quiet moves (whose range is approximately `[-16384, 16384]`) while remaining well below capture scores (MVV-LVA scores start at ~100 for PxP and go up to ~89,900 for PxQ).

**Updated `score_move` signature:**
```rust
fn score_move(
    mv: Move,
    pos: &Position,
    killers: &KillerTable,
    history: &HistoryTable,
    ply: u8,
    pv_move: Option<Move>,
    tt_move: Option<Move>,
    side: Color,
) -> i32
```

**Updated scoring logic for the quiet move branch:**
```rust
} else if killers.is_killer(ply, mv) {
    KILLER_SCORE
} else {
    history.score(side, mv.from_sq(), mv.to_sq())
}
```
Previously, non-killer quiet moves scored 0. Now they use their history score.

**Updated `order_moves` signature:**
```rust
pub fn order_moves(
    moves: &mut [Move],
    pos: &Position,
    killers: &KillerTable,
    history: &HistoryTable,
    ply: u8,
    pv_move: Option<Move>,
    tt_move: Option<Move>,
    side: Color,
)
```

**Update existing tests in `ordering.rs`:**

All six existing tests call `order_moves` and/or `score_move`. Each call must be updated to pass the new `&HistoryTable` and `Color` parameters. In every test, create `let history = HistoryTable::new();` (zeroed table) and pass `&history` and `Color::White` as the additional arguments.

Impact of the `KILLER_SCORE` change from 50 to 20,000 on existing tests:
- `score_mvv_lva_pxq_higher_than_pxp` — no impact, tests `score_mvv_lva()` directly (not `score_move`), no assertions involving `KILLER_SCORE`.
- `score_mvv_lva_lower_attacker_scores_higher` — no impact, same reason.
- `score_mvv_lva_quiet_returns_zero` — no impact, same reason.
- `order_moves_captures_before_quiet` — no assertion changes needed. Captures still score above quiet moves regardless of `KILLER_SCORE` value (captures use MVV-LVA which produces scores ≥100 for any capture; `KILLER_SCORE = 20_000` is still below MVV-LVA minimums for this position where captures exist). The test only asserts relative ordering of captures vs. quiet moves with no killers active.
- `killer_moves_ordered_between_captures_and_quiet` — **assertion update required**. This test asserts `killer_idx > last_cap` ("killer should come after all captures"). With `KILLER_SCORE = 20_000`, killers score higher than some MVV-LVA scores (e.g., PxP = 100*100 - 100 = 9,900). However, in this specific test position (`"4k3/8/8/8/8/8/3q4/R3K3 w - - 0 1"`), the only capture is RxQ which scores 900*100 - 500 = 89,500, well above 20,000. So the assertion `killer_idx > last_cap` still holds. **No assertion change needed** for this test.
- `pv_move_ordered_first` — no impact. PV_SCORE (100,000) still exceeds KILLER_SCORE (20,000). Only signature update needed.
- `tt_move_ordered_before_pv` — no impact. TT_SCORE (200,000) > PV_SCORE (100,000). Only signature update needed.

**Summary**: All six existing tests require only signature updates (adding `&history` and `Color::White`). No assertion values need to change because no test directly asserts on `KILLER_SCORE`'s numeric value, and the relative ordering in each test's specific position is preserved with the new value.

### `crates/search/src/lib.rs` (modifications)

**Module declaration (after line 3):**
```rust
pub mod history;
```

**Import (after line 14):**
```rust
use history::HistoryTable;
```

**`SearchContext` field (add after `killers: KillerTable,` at line 41):**
```rust
history_table: HistoryTable,
```

Note: The existing `history: Vec<u64>` field stores Zobrist hashes for repetition detection. The new field is named `history_table` to avoid collision.

**`search()` function — initialize `HistoryTable` in `SearchContext` construction (around line 409):**
```rust
history_table: HistoryTable::new(),
```

**`test_ctx()` helper — add the new field:**
```rust
history_table: HistoryTable::new(),
```

**`order_moves` call sites — update to pass `&ctx.history_table` and side:**

1. In `negamax` (line 284):
```rust
let side = pos.side_to_move();
ordering::order_moves(&mut moves, pos, &ctx.killers, &ctx.history_table, ply, pv_move, tt_move, side);
```

2. In `quiescence` (line 130):
```rust
ordering::order_moves(&mut tactical, pos, &ctx.killers, &ctx.history_table, ply, None, None, pos.side_to_move());
```

**Track quiet moves searched before cutoff:**

Add a `Vec<Move>` before the move loop (before line 298) to collect quiet, non-promotion moves that were searched but did not cause a cutoff:
```rust
let mut searched_quiets: Vec<Move> = Vec::new();
```

**Place `searched_quiets.push(mv)` between the abort check and the score check.** Specifically, after `pos.unmake_move(mv, undo)` (line 363) and after the abort check (lines 365-367), but *before* the `if score > alpha` check (line 369):
```rust
        ctx.history.pop();
        pos.unmake_move(mv, undo);

        if ctx.aborted {
            return (0, None);
        }

        // Track quiet non-promotion moves that were searched but haven't caused a cutoff yet.
        // Moves that DO cause a cutoff are handled separately in the cutoff branch below.
        if !mv.is_capture() && !mv.is_promotion() {
            searched_quiets.push(mv);
        }

        if score > alpha {
            // ... existing alpha/beta logic
```

Note: The cutoff move itself will be in `searched_quiets` at this point, but it receives a positive bonus in the cutoff branch which overwrites the inclusion. Alternatively, to be precise, we can move the push to *only* happen when the move does NOT cause a cutoff. The cleaner approach: push *before* the `if score > alpha` check (as shown above), and in the cutoff branch, pop the last element from `searched_quiets` before applying negative bonuses (since the cutoff move was the last one pushed). The simplest correct approach is:

```rust
        if !mv.is_capture() && !mv.is_promotion() {
            searched_quiets.push(mv);
        }

        if score > alpha {
            alpha = score;
            best_move = Some(mv);
            ctx.pv_table.update(ply, mv);
            if alpha >= beta {
                if !mv.is_capture() {
                    ctx.killers.store(ply, mv);
                    let bonus = (depth as i32) * (depth as i32);
                    let side = pos.side_to_move();
                    // Positive bonus for the cutoff move
                    ctx.history_table.update(side, mv.from_sq(), mv.to_sq(), bonus);
                    // Negative bonus for all quiet moves searched before the cutoff move
                    // (excluding the cutoff move itself — it's the last element, so iterate up to len-1)
                    for i in 0..searched_quiets.len().saturating_sub(1) {
                        let prev_mv = searched_quiets[i];
                        ctx.history_table.update(side, prev_mv.from_sq(), prev_mv.to_sq(), -bonus);
                    }
                }
                break;
            }
        }
```

**Side-to-move for history indexing:** After `pos.unmake_move(mv, undo)`, the position is restored to *before* the move was made, so `pos.side_to_move()` returns the side that made the move. This is the correct side for history indexing — the side whose move caused the cutoff. No `.opposite()` call is needed.

## 4. API Contracts

The `HistoryTable` is an internal search data structure with no external API surface (not exposed via UCI or any public interface beyond the crate).

**Internal API:**
```rust
// Construction
let mut ht = HistoryTable::new();

// Update (called in search loop)
ht.update(Color::White, Square::E2, Square::E4, 9); // bonus = 3*3 at depth 3

// Query (called in move ordering)
let score: i32 = ht.score(Color::White, Square::E2, Square::E4);

// Reset
ht.clear();
```

N/A for REST/HTTP.

## 5. Test Plan

| Test | Location | What it verifies |
|------|----------|-----------------|
| `gravity_formula_bounds_scores` | `history.rs` | Repeated large-bonus updates keep scores within `[-16384, 16384]` |
| `clear_resets_all` | `history.rs` | `clear()` resets all entries to zero |
| `initial_score_is_zero` | `history.rs` | Fresh table returns 0 for all entries |
| `positive_bonus_increases_score` | `history.rs` | Single positive update yields positive score |
| `negative_bonus_decreases_score` | `history.rs` | Single negative update yields negative score |
| Existing `ordering.rs` tests (6 tests) | `ordering.rs` | Updated to pass `&HistoryTable::new()` and `Color::White` — all pass with no behavioral regression and no assertion changes |
| Existing `lib.rs` tests | `lib.rs` | `test_ctx()` includes `history_table` — all pass with no regression |
| `cargo test` (full workspace) | — | No regressions across all crates |
| `cargo clippy -- -D warnings` | — | No lint warnings |
| `cargo fmt --check` | — | Formatting correct |

## 6. Implementation Order

1. `crates/search/src/history.rs` — implement `HistoryTable` struct with `new()`, `update()`, `score()`, `clear()`, `Default` impl, and unit tests
2. `crates/search/src/ordering.rs` — add `HistoryTable` and `Color` parameters to `score_move` and `order_moves`, use history score for quiet moves, raise `KILLER_SCORE` to 20,000, update all six existing tests to pass `&HistoryTable::new()` and `Color::White`
3. `crates/search/src/lib.rs` — add `pub mod history;` and `use history::HistoryTable;`, add `history_table: HistoryTable` to `SearchContext`, update `search()` and `test_ctx()` constructors, update `order_moves` call sites in `negamax` and `quiescence`, add `searched_quiets` tracking and history update logic at beta cutoff using `pos.side_to_move()` (no `.opposite()`)

## 7. Verification Commands

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --workspace
```

## 8. Design Deviations

**Deviation 1: `KILLER_SCORE` raised from 50 to 20,000**
- The phase spec prescribes ordering: killers > history-ordered quiet moves.
- With the existing `KILLER_SCORE = 50`, history scores (range ~`[-16384, 16384]`) can exceed the killer score, breaking the prescribed ordering.
- Setting `KILLER_SCORE = 20_000` ensures killers always rank above history-scored quiet moves while remaining below captures (MVV-LVA scores start at 9,900 for PxP and go up to ~89,900 for PxQ).

**Deviation 2: Field named `history_table` instead of `history`**
- The task spec says "Add `HistoryTable` as a field on `SearchContext`".
- `SearchContext` already has a field named `history: Vec<u64>` for repetition detection (Zobrist hash history).
- The new field is named `history_table` to avoid a naming collision. All references to the history heuristic table use this name.

**Deviation 3: History table created fresh per `search()` call, not persisted across games**
- The task spec says "Clear history table on `ucinewgame` command."
- In the current architecture, `SearchContext` (including the TT, killers, and all tables) is created fresh inside the `search()` function (line 409 of `lib.rs`). No tables persist across `search()` calls.
- Making the history table persistent across searches would require refactoring `SearchContext` into a persistent engine state, which is not in scope for this task.
- Since the table is re-created each `search()` call, it is inherently cleared between games. The gravity formula ensures proper aging within a single iterative deepening search. No explicit `ucinewgame` clear integration is needed because the current architecture already achieves this effect.