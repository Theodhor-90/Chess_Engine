Now I have all the context. Let me compute the actual MVV-LVA scores and design the correct approach:

- PxP = 100*100 - 100 = 9,900
- NxP = 100*100 - 320 = 9,680
- BxP = 100*100 - 330 = 9,670
- RxP = 100*100 - 500 = 9,500
- QxP = 100*100 - 900 = 9,100
- PxN = 320*100 - 100 = 31,900
- NxN = 320*100 - 320 = 31,680
- ...

So the minimum MVV-LVA score for any capture is QxP = 9,100. History scores range up to ~16,384. There is no single `KILLER_SCORE` value that sits above 16,384 (history max) and below 9,100 (minimum capture). The feedback is right — this needs a different approach.

The cleanest fix: add a `CAPTURE_BASE` offset to all capture scores so they always sort above killers and history moves. This way the phase spec ordering is preserved: captures > killers > history.

## 1. Deliverables

- `crates/search/src/history.rs` — `HistoryTable` struct with `update()`, `score()`, and `clear()` methods, indexed by `[side][from_square][to_square]`, implementing the gravity formula for score updates
- `crates/search/src/ordering.rs` (modified) — Updated `score_move` and `order_moves` functions to accept a `HistoryTable` reference and `Color` parameter; use history scores for quiet move ordering; `KILLER_SCORE` raised to 20,000; `CAPTURE_BASE` offset of 100,000 added to all MVV-LVA capture scores so captures always sort above killers and history-scored quiet moves
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

**Add `CAPTURE_BASE` constant:**
```rust
const CAPTURE_BASE: i32 = 100_000;
```
This offset is added to all MVV-LVA capture scores so that every capture sorts above killers and history-scored quiet moves.

Rationale: Without this offset, MVV-LVA scores for low-value captures (e.g., QxP = 9,100) fall below the history score range (~[-16,384, 16,384]) and the KILLER_SCORE (20,000). The phase spec ordering hierarchy requires all captures to sort above killers and quiet moves. Adding `CAPTURE_BASE = 100,000` ensures the minimum capture score (QxP) becomes 109,100, well above KILLER_SCORE (20,000) and the maximum history score (~16,384). This constant is below `PV_SCORE` (100,000 → becomes effectively second tier since PV_SCORE is already at 100,000). We raise `PV_SCORE` to 200,000 and `TT_SCORE` to 300,000 to maintain the full ordering hierarchy.

**Updated constants:**
```rust
const TT_SCORE: i32 = 300_000;
const PV_SCORE: i32 = 200_000;
const CAPTURE_BASE: i32 = 100_000;
const KILLER_SCORE: i32 = 20_000;
```

This guarantees the ordering hierarchy:
- TT move (300,000) > PV move (200,000) > captures (100,000 + MVV-LVA, range 109,100–189,900) > killers (20,000) > history-ordered quiet moves (range ~-16,384 to ~16,384) > plain quiet moves (0, when history is zero)

The minimum capture score is QxP = 100,000 + 9,100 = 109,100. The maximum killer score is 20,000. This satisfies the phase spec requirement that all captures rank above killers.

**Raise `KILLER_SCORE`:** Change `const KILLER_SCORE: i32 = 50;` to `const KILLER_SCORE: i32 = 20_000;`. This ensures killers always rank above history-scored quiet moves (whose range is approximately `[-16384, 16384]`).

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

**Updated capture scoring:**
```rust
} else if mv.is_capture() {
    CAPTURE_BASE + score_mvv_lva(mv, pos)
}
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

Impact analysis of the constant changes on existing tests:

- `score_mvv_lva_pxq_higher_than_pxp` — no impact, tests `score_mvv_lva()` directly (not `score_move`), no assertions involving any constants.
- `score_mvv_lva_lower_attacker_scores_higher` — no impact, same reason.
- `score_mvv_lva_quiet_returns_zero` — no impact, same reason.
- `order_moves_captures_before_quiet` — no assertion changes needed. With `CAPTURE_BASE`, captures score even higher relative to quiet moves (minimum capture = 109,100 vs. quiet = 0). The test only asserts relative ordering of captures vs. quiet moves. Still passes.
- `killer_moves_ordered_between_captures_and_quiet` — **still passes with no assertion changes**. The test asserts: (1) `killer_idx > last_cap` — killers (20,000) sort after captures (now 109,100+ with CAPTURE_BASE). The only capture in the test position is RxQ = 100,000 + 89,500 = 189,500, well above 20,000. ✓ (2) `killer_idx < first_quiet` — killers (20,000) sort before plain quiet moves (0 history). ✓ Both assertions hold.
- `pv_move_ordered_first` — PV_SCORE is now 200,000 (was 100,000). PV move still sorts first when no TT move is present. The test passes `None` for `tt_move`, so PV (200,000) is the highest. ✓ No assertion change.
- `tt_move_ordered_before_pv` — TT_SCORE is now 300,000 (was 200,000), PV_SCORE is 200,000 (was 100,000). TT (300,000) > PV (200,000). The test asserts `moves[0] == tt_mv` and `moves[1] == pv_mv`. ✓ No assertion change.

**Summary**: All six existing tests require only signature updates (adding `&history` and `Color::White`). No assertion values need to change because the relative ordering in each test's specific position is preserved with the new constants.

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

**Place `searched_quiets.push(mv)` after unmake but before the score check.** Specifically, after `pos.unmake_move(mv, undo)` and after the abort check, but *before* the `if score > alpha` check:
```rust
        ctx.history.pop();
        pos.unmake_move(mv, undo);

        if ctx.aborted {
            return (0, None);
        }

        // Track quiet non-promotion moves that were searched but haven't caused a cutoff yet.
        if !mv.is_capture() && !mv.is_promotion() {
            searched_quiets.push(mv);
        }

        if score > alpha {
            // ... existing alpha/beta logic
```

**History updates at beta cutoff:**
```rust
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
2. `crates/search/src/ordering.rs` — add `HistoryTable` and `Color` parameters to `score_move` and `order_moves`; add `CAPTURE_BASE = 100_000` offset to all capture scores; raise `PV_SCORE` to 200,000 and `TT_SCORE` to 300,000 to maintain hierarchy; raise `KILLER_SCORE` to 20,000; use history score for quiet moves; update all six existing tests to pass `&HistoryTable::new()` and `Color::White`
3. `crates/search/src/lib.rs` — add `pub mod history;` and `use history::HistoryTable;`, add `history_table: HistoryTable` to `SearchContext`, update `search()` and `test_ctx()` constructors, update `order_moves` call sites in `negamax` and `quiescence`, add `searched_quiets` tracking and history update logic at beta cutoff using `pos.side_to_move()` (no `.opposite()`)

## 7. Verification Commands

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --workspace
```

## 8. Design Deviations

**Deviation 1: `CAPTURE_BASE` offset added to all capture scores; `PV_SCORE` and `TT_SCORE` raised**
- The phase spec prescribes ordering: TT move > PV move > good captures (positive SEE) > killer moves > history-ordered quiet moves > bad captures (negative SEE).
- With the current MVV-LVA scoring, low-value captures (e.g., QxP = 9,100) score below the history heuristic range (~[-16,384, 16,384]). Adding `KILLER_SCORE = 20,000` alone would cause killers to sort above these captures, violating the spec hierarchy.
- Fix: Add `CAPTURE_BASE = 100,000` to all MVV-LVA scores, ensuring the minimum capture (QxP = 109,100) exceeds KILLER_SCORE (20,000). `PV_SCORE` raised to 200,000 and `TT_SCORE` raised to 300,000 to maintain the full TT > PV > capture > killer > history ordering.
- Note: The phase spec distinguishes "good captures (positive SEE)" from "bad captures (negative SEE)". In this task (T01), SEE is not yet implemented (it arrives in T03). Until T03, all captures receive the `CAPTURE_BASE` offset uniformly, meaning losing captures also sort above killers. This is a known temporary ordering approximation. T03 will split captures into good (positive SEE, scored with `CAPTURE_BASE +` MVV-LVA) and bad (negative SEE, scored below quiet moves) to match the spec ordering exactly.

**Deviation 2: Field named `history_table` instead of `history`**
- The task spec says "Add `HistoryTable` as a field on `SearchContext`".
- `SearchContext` already has a field named `history: Vec<u64>` for repetition detection (Zobrist hash history).
- The new field is named `history_table` to avoid a naming collision. All references to the history heuristic table use this name.

**Deviation 3: History table created fresh per `search()` call, not persisted across games**
- The task spec says "Clear history table on `ucinewgame` command."
- In the current architecture, `SearchContext` (including the TT, killers, and all tables) is created fresh inside the `search()` function (line 409 of `lib.rs`). No tables persist across `search()` calls.
- Making the history table persistent across searches would require refactoring `SearchContext` into a persistent engine state, which is not in scope for this task.
- Since the table is re-created each `search()` call, it is inherently cleared between games. The gravity formula ensures proper aging within a single iterative deepening search. No explicit `ucinewgame` clear integration is needed because the current architecture already achieves this effect.