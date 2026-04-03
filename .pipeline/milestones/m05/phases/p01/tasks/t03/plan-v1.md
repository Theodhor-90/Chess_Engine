Now I have a complete understanding of the codebase. Let me produce the implementation plan.

## 1. Deliverables

| # | File | Action | Purpose |
|---|------|--------|---------|
| 1 | `crates/search/src/tt.rs` | Modify | Add mate score adjustment helper functions (`score_to_tt` and `score_from_tt`) |
| 2 | `crates/search/src/ordering.rs` | Modify | Add a `tt_move` parameter to `order_moves` and `score_move`, giving TT moves highest priority (above PV) |
| 3 | `crates/search/src/lib.rs` | Modify | Add `TranspositionTable` to `SearchContext`; integrate TT probe at the start of `negamax` and TT store at the end; pass TT best move to `order_moves`; call `new_generation()` at the start of each iterative deepening search |

## 2. Dependencies

- **T01 / T02 (this phase)**: `TtEntry`, `BoundType`, `TranspositionTable` with `probe`, `store`, `new_generation`, `verification_key` — all already implemented in `crates/search/src/tt.rs`
- **M03 (prior milestone)**: `negamax`, `quiescence`, `SearchContext`, `search`, `MATE_SCORE`, `INFINITY` — already implemented in `crates/search/src/lib.rs`
- **M01**: `Position::hash()` returning `u64` — already available in `crates/board/src/position.rs:227`
- **chess-types crate**: `Move::from_raw(u16)`, `Move::raw() -> u16` — already available
- No new external dependencies or crate additions required

## 3. Implementation Details

### 3.1 `crates/search/src/tt.rs` — Mate Score Adjustment Helpers

**Purpose**: Provide two public helper functions for converting between root-relative and node-relative mate scores when storing/retrieving from the TT.

**Exports**:
- `pub fn score_to_tt(score: i32, ply: u8) -> i32` — Convert a root-relative score to a TT-storable (node-relative) score. For positive mate scores (score > MATE_SCORE - MAX_PLY), add ply. For negative mate scores (score < -(MATE_SCORE - MAX_PLY)), subtract ply. Non-mate scores pass through unchanged.
- `pub fn score_from_tt(score: i32, ply: u8) -> i32` — Convert a TT-stored (node-relative) score back to root-relative. For positive mate scores, subtract ply. For negative mate scores, add ply. Non-mate scores pass through unchanged.

**Key Logic**:
- Define a constant `const MAX_PLY: i32 = 128;` for the mate score threshold detection
- A score is a mate score if `score.abs() > MATE_SCORE - MAX_PLY` (where `MATE_SCORE` is imported from the parent module via `crate::MATE_SCORE`)
- Adjustment: when storing, shift mate distance away from root by adding ply for positive / subtracting ply for negative. When retrieving, reverse the operation.

### 3.2 `crates/search/src/ordering.rs` — TT Move Integration

**Purpose**: Allow the TT best move to be passed into `order_moves` and scored with the highest priority.

**Changes**:
- Add a `tt_move: Option<Move>` parameter to the `score_move` function and the `order_moves` function
- Add a new constant `const TT_SCORE: i32 = 200_000;` (higher than the existing `PV_SCORE` of 100_000)
- In `score_move`, check `tt_move` first: if `tt_move == Some(mv)`, return `TT_SCORE`. The existing PV/capture/killer logic follows unchanged
- Update all call sites of `order_moves` (in `lib.rs`) to pass the new `tt_move` parameter

**Interface change**:
```rust
pub fn order_moves(
    moves: &mut [Move],
    pos: &Position,
    killers: &KillerTable,
    ply: u8,
    pv_move: Option<Move>,
    tt_move: Option<Move>,  // NEW
)
```

### 3.3 `crates/search/src/lib.rs` — TT Integration into Search

**Purpose**: Wire the transposition table into the search lifecycle.

**Changes to `SearchContext`**:
- Add field `tt: TranspositionTable` — initialized with a default size (e.g., 64 MB) in the `search` function
- The TT persists across iterative deepening iterations within a single `search` call

**Changes to `search` function**:
- When creating `SearchContext`, initialize `tt: TranspositionTable::new(64)`
- Call `ctx.tt.new_generation()` before the iterative deepening loop begins (once per `search` call)

**Changes to `negamax` function**:

*TT Probe (at the start, after abort/time checks, before move generation)*:
1. Compute `hash = pos.hash()`
2. Call `ctx.tt.probe(hash)` — if `Some(entry)`:
   - Extract `tt_move = entry.best_move()`
   - If `ply > 0` (not root) and `entry.depth() >= depth`:
     - Let `tt_score = score_from_tt(entry.score(), ply)`
     - `Exact` bound: return `(tt_score, tt_move)` directly
     - `LowerBound`: if `tt_score >= beta`, return `(beta, tt_move)` (beta cutoff). Note: returning `tt_score` here is more standard; however, per fail-hard convention used in the existing code (returning `beta` on cutoff), we return `beta`.
     - `UpperBound`: if `tt_score <= alpha`, return `(alpha, tt_move)`. Same fail-hard reasoning: return `alpha`.
   - Even if depth is insufficient for a cutoff, preserve `tt_move` for move ordering
3. If no TT hit, `tt_move = None`

*Move Ordering*:
- Pass `tt_move` to the updated `order_moves` call: `ordering::order_moves(&mut moves, pos, &ctx.killers, ply, pv_move, tt_move)`

*TT Store (at the end, before returning)*:
1. Determine bound type:
   - If `best_move.is_none()` and no move improved alpha → `UpperBound` (all-node)
   - If `alpha >= beta` (a beta cutoff occurred) → `LowerBound`
   - Otherwise (alpha was improved but not a cutoff) → `Exact`
2. Compute `store_score = score_to_tt(alpha, ply)` for the score to store
3. Create `TtEntry::new(verification_key(hash), depth, store_score, bound_type, best_move, ctx.tt.generation())`
4. Call `ctx.tt.store(hash, entry)`

*Bound type determination detail*: Track whether alpha was improved by keeping the original alpha in a variable `original_alpha` at the top of the function. At the end:
- If `alpha <= original_alpha` → `UpperBound`
- If `alpha >= beta` → `LowerBound` 
- Otherwise → `Exact`

*Quiescence search call in `order_moves`*:
- Update the quiescence `order_moves` call to pass `None` for `tt_move` (no TT probing in quiescence)

*Root node handling*:
- At root (`ply == 0`), still probe the TT for move ordering but do NOT return early on a cutoff — the root must always complete a full search to return a valid best move

## 4. API Contracts

**`score_to_tt(score: i32, ply: u8) -> i32`**
- Input: root-relative score, current ply distance from root
- Output: node-relative score suitable for TT storage
- Example: `score_to_tt(MATE_SCORE - 5, 3)` → `MATE_SCORE - 5 + 3` = `MATE_SCORE - 2`
- Example: `score_to_tt(-(MATE_SCORE - 5), 3)` → `-(MATE_SCORE - 5) - 3` = `-(MATE_SCORE - 2)`
- Example: `score_to_tt(150, 3)` → `150` (no adjustment for non-mate)

**`score_from_tt(score: i32, ply: u8) -> i32`**
- Input: node-relative score from TT, current ply distance from root
- Output: root-relative score
- Example: `score_from_tt(MATE_SCORE - 2, 3)` → `MATE_SCORE - 2 - 3` = `MATE_SCORE - 5`
- Example: `score_from_tt(-(MATE_SCORE - 2), 3)` → `-(MATE_SCORE - 2) + 3` = `-(MATE_SCORE - 5)`
- Example: `score_from_tt(150, 3)` → `150`

**`order_moves(moves, pos, killers, ply, pv_move, tt_move)`**
- New parameter `tt_move: Option<Move>` — if provided, that move is scored at 200,000 (above PV's 100,000)
- All existing behavior unchanged when `tt_move` is `None`

## 5. Test Plan

### 5.1 Tests in `crates/search/src/tt.rs`

**`score_to_tt_positive_mate`**: Verify `score_to_tt(MATE_SCORE - 5, 3)` equals `MATE_SCORE - 2`.

**`score_to_tt_negative_mate`**: Verify `score_to_tt(-(MATE_SCORE - 5), 3)` equals `-(MATE_SCORE - 2)`.

**`score_to_tt_non_mate_unchanged`**: Verify `score_to_tt(150, 3)` equals `150`.

**`score_from_tt_positive_mate`**: Verify `score_from_tt(MATE_SCORE - 2, 3)` equals `MATE_SCORE - 5`.

**`score_from_tt_negative_mate`**: Verify `score_from_tt(-(MATE_SCORE - 2), 3)` equals `-(MATE_SCORE - 5)`.

**`score_from_tt_non_mate_unchanged`**: Verify `score_from_tt(150, 3)` equals `150`.

**`score_round_trip`**: Verify `score_from_tt(score_to_tt(score, ply), ply) == score` for several mate and non-mate scores at different plies.

### 5.2 Tests in `crates/search/src/ordering.rs`

**`tt_move_ordered_before_pv`**: Create a move list with both a PV move and a TT move set. After `order_moves`, verify the TT move is at index 0 and the PV move is at index 1.

### 5.3 Tests in `crates/search/src/lib.rs`

**`tt_reduces_node_count`**: Search the same position (e.g., `"r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2"`) to depth 4 with iterative deepening. Compare total node counts between a search with the TT active (normal code path) vs. a search where TT probing and storing are disabled (by using a 0 MB TT that effectively never hits). The TT-enabled search must visit fewer nodes.

**`tt_move_is_ordered_first`**: After searching a position to a given depth, probe the TT for that position and verify a best move is stored. Then generate legal moves, call `order_moves` with the TT move, and verify the TT move is at index 0.

**`mate_score_correct_with_tt`**: Search a known mate-in-2 position. Verify the returned score is `MATE_SCORE - 3` (mate in 2 = 3 plies from root) or equivalent for the particular position used, confirming mate scores survive TT storage/retrieval.

**Test setup**: The existing `test_ctx()` helper must be updated to include the `tt` field. Two variants may be useful: one with a normal TT (e.g., 1 MB) and one with a minimal/disabled TT for comparison tests.

## 6. Implementation Order

1. **Add mate score helpers to `tt.rs`** — Implement `score_to_tt` and `score_from_tt` with unit tests. This is a pure addition with no existing code changes.

2. **Update `ordering.rs`** — Add the `tt_move` parameter to `score_move` and `order_moves`. Add `TT_SCORE` constant. Add the `tt_move_ordered_before_pv` test. This is a signature change that will temporarily break callers.

3. **Update `lib.rs` — SearchContext** — Add the `tt: TranspositionTable` field to `SearchContext`. Update the `search` function constructor and `test_ctx()` helper to include the TT. Update all `order_moves` call sites to pass `tt_move: None` temporarily to restore compilation.

4. **Update `lib.rs` — negamax TT probe** — Add `original_alpha` tracking at the top of `negamax`. Add the TT probe logic at the start of `negamax` (after abort check, before move generation). Pass the extracted `tt_move` to `order_moves`.

5. **Update `lib.rs` — negamax TT store** — At the end of `negamax`, determine bound type from `original_alpha`/`alpha`/`beta`, create a `TtEntry`, and store it. Call `new_generation()` in the `search` function before the iterative deepening loop.

6. **Add integration tests** — Add `tt_reduces_node_count`, `tt_move_is_ordered_first`, and `mate_score_correct_with_tt` tests.

7. **Verify** — Run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test -p chess-search`, and full `cargo test`.

## 7. Verification Commands

```bash
# Format check
cargo fmt --check --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml

# Lint check
cargo clippy --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml -- -D warnings

# Run search crate tests
cargo test -p chess-search --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml

# Run full workspace tests
cargo test --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml

# Build the engine binary to verify no compilation errors
cargo build --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml
```

## 8. Design Deviations

**Deviation 1: Fail-hard vs. fail-soft TT cutoff returns**

- **Parent spec states**: "LowerBound: if stored score ≥ beta, return beta cutoff" and "UpperBound: if stored score ≤ alpha, return alpha cutoff"
- **Issue**: The existing `negamax` uses a **fail-hard** framework — it returns `beta` on beta cutoffs (see line 176: `if alpha >= beta { break; }` followed by returning `alpha` which equals `beta`). For consistency with fail-hard, TT cutoffs should also return `beta` for lower-bound cutoffs and `alpha` for upper-bound cutoffs, rather than returning the stored TT score.
- **Alternative chosen**: Return `beta` on lower-bound TT cutoffs and `alpha` on upper-bound TT cutoffs, maintaining fail-hard consistency with the existing search framework. For Exact bounds, the stored score is returned directly (as it is the true minimax value).

**Deviation 2: TT move priority vs. PV move priority**

- **Parent spec states**: "Use the TT best move as the highest-priority move for ordering" and the phase spec says the `pv_move` parameter may be "generalized or add a `tt_move` parameter"
- **Issue**: Adding the TT move through the existing `pv_move` parameter would make it impossible to distinguish between PV and TT moves in move ordering. Both have value — the PV move from the previous iteration and the TT move from prior TT storage may differ. The TT move should have strictly higher priority than the PV move.
- **Alternative chosen**: Add a separate `tt_move` parameter to `order_moves` with a score of 200,000 (above PV's 100,000). This preserves both ordering signals and ensures the TT move is always first when present.