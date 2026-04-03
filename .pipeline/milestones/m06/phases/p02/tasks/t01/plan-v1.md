I now see the architecture clearly. `SearchContext` is created fresh per `search()` call (line 409), and it's not persisted. The `HistoryTable` will need to follow this same pattern within the current architecture. The "clear on `ucinewgame`" requirement from the task spec is about the eventual design where these tables persist — but that would require refactoring `SearchContext` to be persistent, which is not in scope for T01. Since the table is created fresh per search call, it's inherently cleared.

However, I should note: for the history heuristic to be truly effective, it should persist across iterative deepening iterations within a single `search()` call — and it does, since `SearchContext` lives for the entire `search()` call. The gravity formula handles aging. This is correct for the current architecture.

Now I have everything needed. Let me write the complete plan.

## 1. Deliverables

- `crates/search/src/history.rs` — `HistoryTable` struct with `update()`, `score()`, and `clear()` methods, indexed by `[side][from_square][to_square]`, implementing the gravity formula for score updates
- `crates/search/src/ordering.rs` (modified) — Updated `score_move` and `order_moves` functions to accept a `HistoryTable` reference and use history scores for quiet move ordering
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

**Add import:** `use crate::history::HistoryTable;`

**Add constant:** `const HISTORY_BASE: i32 = 0;` — not needed since history scores are used directly. Instead, quiet moves scored by history will use the raw history score, which ranges approximately `[-16384, 16384]`. Since `KILLER_SCORE` = 50 and captures scored by MVV-LVA produce values like `100 * victim_value - attacker_value` (e.g., PxQ = 90000 - 100 = 89900), history scores naturally slot below killers for most values. However, since history scores can theoretically exceed 50, and the phase spec's ordering priority places killers above history-ordered quiet moves, we must ensure that killer moves still score above history moves. The existing `KILLER_SCORE = 50` is too low relative to history score range. The fix is to raise `KILLER_SCORE` to a value above the history score range, e.g., `KILLER_SCORE = 20_000`.

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

**Update existing tests** in `ordering.rs` to pass the new `HistoryTable` and `Color` parameters. Create a default `HistoryTable::new()` and use `Color::White` (or appropriate side) in existing test calls to `order_moves` and `score_move`.

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

**Beta cutoff in the main move loop — update history table (around line 373-377):**

After the existing `if alpha >= beta` block, when a quiet move causes a beta cutoff:
```rust
if alpha >= beta {
    if !mv.is_capture() {
        ctx.killers.store(ply, mv);
        // Update history: positive bonus for cutoff move
        let bonus = (depth as i32) * (depth as i32);
        let side = pos.side_to_move().opposite(); // side that just moved
        ctx.history_table.update(side, mv.from_sq(), mv.to_sq(), bonus);
        // Negative bonus for all quiet moves searched before the cutoff move
        for &prev_mv in &searched_quiets {
            ctx.history_table.update(side, prev_mv.from_sq(), prev_mv.to_sq(), -bonus);
        }
    }
    break;
}
```

**Track quiet moves searched before cutoff:** Add a `Vec<Move>` (or a stack-allocated small vec) before the move loop to collect quiet (non-capture, non-promotion) moves that were searched but did not cause a cutoff:
```rust
let mut searched_quiets: Vec<Move> = Vec::new();
```

Inside the move loop, after unmake and before the score check, if the move is quiet and does not cause a cutoff, push it to `searched_quiets`:
```rust
// After the score comparison, inside the `if score > alpha` block but before break:
// If this quiet move improved alpha but didn't cause cutoff, it's fine (it won't be in searched_quiets).
// Outside the `if score > alpha` block, if the move is quiet:
if !mv.is_capture() && !mv.is_promotion() {
    searched_quiets.push(mv);
}
```

The `searched_quiets.push(mv)` call goes just before the `if score > alpha` check (since if it causes a cutoff, it's handled in the cutoff branch, not added to `searched_quiets`).

**Important side note:** `pos.side_to_move()` after `unmake_move` returns the side that made the move. But actually, after `pos.unmake_move(mv, undo)`, the position is restored to before the move was made, so `pos.side_to_move()` is the side that made the move. This is the correct side for history indexing — the side whose move caused the cutoff.

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
| Existing `ordering.rs` tests | `ordering.rs` | Updated to pass `HistoryTable` and `Color` — all pass with no behavioral regression |
| Existing `lib.rs` tests | `lib.rs` | `test_ctx()` includes `history_table` — all pass with no regression |
| `cargo test` (full workspace) | — | No regressions across all crates |
| `cargo clippy -- -D warnings` | — | No lint warnings |
| `cargo fmt --check` | — | Formatting correct |

## 6. Implementation Order

1. `crates/search/src/history.rs` — implement `HistoryTable` struct with `new()`, `update()`, `score()`, `clear()`, `Default` impl, and unit tests
2. `crates/search/src/ordering.rs` — add `HistoryTable` and `Color` parameters to `score_move` and `order_moves`, use history score for quiet moves, raise `KILLER_SCORE` to 20,000, update all existing tests
3. `crates/search/src/lib.rs` — add `pub mod history;` and `use history::HistoryTable;`, add `history_table: HistoryTable` to `SearchContext`, update `search()` and `test_ctx()` constructors, update `order_moves` call sites in `negamax` and `quiescence`, add `searched_quiets` tracking and history update logic at beta cutoff

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
- Setting `KILLER_SCORE = 20_000` ensures killers always rank above history-scored quiet moves while remaining below captures (MVV-LVA scores start at ~100 for PxP and go up to ~89,900 for PxQ).

**Deviation 2: Field named `history_table` instead of `history`**
- The task spec says "Add `HistoryTable` as a field on `SearchContext`".
- `SearchContext` already has a field named `history: Vec<u64>` for repetition detection (Zobrist hash history).
- The new field is named `history_table` to avoid a naming collision. All references to the history heuristic table use this name.

**Deviation 3: History table created fresh per `search()` call, not persisted across games**
- The task spec says "Clear history table on `ucinewgame` command."
- In the current architecture, `SearchContext` (including the TT, killers, and all tables) is created fresh inside the `search()` function (line 409 of `lib.rs`). No tables persist across `search()` calls.
- Making the history table persistent across searches would require refactoring `SearchContext` into a persistent engine state, which is not in scope for this task.
- Since the table is re-created each `search()` call, it is inherently cleared between games. The gravity formula ensures proper aging within a single iterative deepening search. No explicit `ucinewgame` clear integration is needed because the current architecture already achieves this effect.