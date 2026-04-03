Now I have all the information I need. Let me produce the revised plan.

## 1. Deliverables

- **Modify: `crates/search/src/lib.rs`** — Define a `TbProber` trait. Add an optional `tb_prober` parameter to `search` and `negamax`. Implement root-level tablebase filtering before the iterative deepening loop. Implement in-search WDL probing at non-root nodes, gated by piece count.

- **Modify: `crates/engine/src/syzygy/mod.rs`** — Add a `SyzygyProberAdapter` struct wrapping `SyzygyTablebase` and `probe_limit`, implementing the `TbProber` trait from the search crate.

- **Modify: `crates/engine/src/main.rs`** — Wire the `SyzygyProberAdapter` into the `Go` command handler, passing it to `chess_search::search`.

## 2. Dependencies

- **T01 (completed)**: The `SyzygyTablebase` type in `crates/engine/src/syzygy/mod.rs` with `probe_wdl`, `probe_dtz`, and `wdl_to_score` functions. `TB_WIN_SCORE = MATE_SCORE - 200 = 29800`.
- **`chess-movegen`**: Already a dependency of `chess-search`; provides `generate_legal_moves`.
- **`chess-board`**: Already a dependency of `chess-search`; provides `Position` with `occupied().pop_count()`, `make_move`, `unmake_move`.
- No new external crate dependencies are required.

## 3. Implementation Details

### `crates/search/src/lib.rs`

**New trait: `TbProber`**

Purpose: Abstracts tablebase probing so the search crate doesn't depend on the engine crate.

```rust
pub trait TbProber {
    /// Probe WDL at a non-root node. Returns converted score or None.
    fn probe_wdl(&mut self, pos: &Position) -> Option<i32>;
    /// Probe WDL + DTZ at root. Returns (wdl_score, dtz) or None.
    fn probe_root(&mut self, pos: &Position) -> Option<(i32, i32)>;
}
```

- `probe_wdl` returns an already-converted score (the implementor calls `wdl_to_score` internally), or `None` on failure/piece-count exceeded. The implementor is responsible for piece-count gating against its configured probe limit.
- `probe_root` returns `(wdl_score, dtz)` for root-level DTZ ordering, or `None` on failure. The implementor is responsible for piece-count gating.

**Modified `search` function signature**

```rust
pub fn search(
    pos: &mut Position,
    limits: SearchLimits,
    game_history: &[u64],
    on_depth: Option<DepthCallback<'_>>,
    tb_prober: Option<&mut dyn TbProber>,
) -> Option<Move>
```

**Root tablebase filtering** (inside `search`, before the iterative deepening loop):

1. If `tb_prober` is `Some`, generate legal moves.
2. For each move, make it, call `tb_prober.probe_root()` on the resulting position, unmake it.
3. Collect `(move, wdl_score, dtz)` for all moves where probing succeeds.
4. If all moves were probed successfully:
   - Negate each WDL score (the probe is from the opponent's POV after making the move).
   - Find the best (highest) negated WDL category among all moves.
   - Filter out moves whose negated WDL score is less than the best.
   - Among remaining moves (same WDL): for wins, sort by ascending absolute DTZ (fastest win); for losses, sort by descending absolute DTZ (slowest loss).
   - If exactly one move remains after filtering, return it immediately without searching.
   - If multiple moves remain, pass the DTZ-best move as the initial PV hint to the iterative deepening loop (i.e., set it as the best move from "depth 0" so it is searched first at every depth), and let the search consider all non-losing filtered moves normally. The search is not restricted to only the filtered moves — the DTZ-best move is used as a move ordering hint only.
5. If probing fails for any move, discard all partial TB results and fall through to normal search with no filtering.

**In-search WDL probing** (inside `negamax`, at non-root nodes, after the repetition detection check, before the `depth == 0` quiescence fallback):

1. If `tb_prober` is `Some` and `ply > 0`:
   - Call `tb_prober.probe_wdl(pos)`.
   - If `Some(score)`: adjust the score for ply distance by subtracting `ply as i32` from positive mate-scale scores and adding `ply as i32` to negative mate-scale scores (i.e., `if score > TB_WIN_SCORE / 2 { score - ply as i32 } else if score < -(TB_WIN_SCORE / 2) { score + ply as i32 } else { score }`). This ensures that closer TB wins sort ahead of more distant TB wins in the search and TT, matching the engine's existing mate-distance convention. Cursed wins (+1), blessed losses (-1), and draws (0) are not adjusted since they are not on the mate scale.
   - Return `(adjusted_score, None)` immediately — this is the exact tablebase result. No further evaluation or deeper search is performed.
2. If `None`, fall through to normal search.

Note: A constant `TB_WIN_THRESHOLD` (e.g., `TB_WIN_SCORE / 2 = 14900`) will be defined in the search crate to distinguish mate-scale TB scores from cursed-win/blessed-loss/draw scores for the ply adjustment logic.

**Key logic details:**

- After making a root move, the resulting position's WDL is from the opponent's perspective. Negate the score: if the opponent is in a loss, the side to move has a win.
- DTZ is used to rank among equal-WDL moves: for wins, prefer move with smallest absolute DTZ (fastest conversion); for losses, prefer move with largest absolute DTZ (slowest loss).
- The `tb_prober` parameter is threaded through `negamax` as well; the function signature gains `tb_prober: Option<&mut dyn TbProber>`.

### `crates/engine/src/syzygy/mod.rs`

**New: `SyzygyProberAdapter` and `impl TbProber`**

Since `TbProber` is defined in `chess-search`, the engine crate implements it for a wrapper:

```rust
use chess_search::TbProber;

pub struct SyzygyProberAdapter {
    pub tb: SyzygyTablebase,
    pub probe_limit: u8,
}

impl TbProber for SyzygyProberAdapter {
    fn probe_wdl(&mut self, pos: &Position) -> Option<i32> {
        let wdl = self.tb.probe_wdl(pos, self.probe_limit)?;
        Some(wdl_to_score(wdl))
    }
    fn probe_root(&mut self, pos: &Position) -> Option<(i32, i32)> {
        let wdl = self.tb.probe_wdl(pos, self.probe_limit)?;
        let dtz = self.tb.probe_dtz(pos, self.probe_limit)?;
        Some((wdl_to_score(wdl), dtz.0))
    }
}
```

Piece-count gating is handled entirely inside `SyzygyTablebase::probe_wdl` and `SyzygyTablebase::probe_dtz` (both already check `piece_count(pos) > probe_limit`). The search crate does not perform any piece-count checks — it delegates entirely to the `TbProber` trait methods.

### `crates/engine/src/main.rs`

- In the `Go` command handler, construct a `SyzygyProberAdapter` if tablebase is available, and pass `Some(&mut adapter)` to `chess_search::search`. Otherwise pass `None`.
- This requires that the engine state holds an optional `SyzygyTablebase` (for T03's UCI option work, but for now the wiring just needs to exist so a prober can be passed).

Note: Root filtering lives in the `search` function itself since that's where the iterative deepening root loop is.

## 4. API Contracts

**`TbProber` trait** (defined in `crates/search/src/lib.rs`):

```rust
pub trait TbProber {
    /// Probe WDL at a non-root node. Returns converted score or None.
    /// The implementor handles piece-count gating and score conversion.
    fn probe_wdl(&mut self, pos: &Position) -> Option<i32>;
    /// Probe WDL + DTZ at root. Returns (wdl_score, dtz) or None.
    /// The implementor handles piece-count gating and score conversion.
    fn probe_root(&mut self, pos: &Position) -> Option<(i32, i32)>;
}
```

Input: `&Position` (the current board position).

Output for `probe_wdl`: `Some(i32)` where the value is a converted score (`TB_WIN_SCORE = 29800` for wins, `0` for draws, `-TB_WIN_SCORE = -29800` for losses, `1` for cursed wins, `-1` for blessed losses) — or `None` if probing failed or piece count exceeds limit.

Output for `probe_root`: `Some((wdl_score, dtz))` where `wdl_score` is as above and `dtz` is the distance-to-zeroing value — or `None` on failure.

**Ply-distance adjustment**: Performed by the `negamax` caller after receiving the score from `probe_wdl`. For mate-scale TB scores (absolute value > `TB_WIN_THRESHOLD`), adjust: wins become `score - ply`, losses become `score + ply`. Non-mate-scale scores (cursed win/blessed loss/draw) are not adjusted.

**Modified `search` function**:

```rust
pub fn search(
    pos: &mut Position,
    limits: SearchLimits,
    game_history: &[u64],
    on_depth: Option<DepthCallback<'_>>,
    tb_prober: Option<&mut dyn TbProber>,
) -> Option<Move>
```

**Modified `negamax` function** (internal):

```rust
pub fn negamax(
    pos: &mut Position,
    depth: u8,
    alpha: i32,
    beta: i32,
    ply: u8,
    allow_null: bool,
    ctx: &mut SearchContext,
    prev_move: Option<(PieceKind, Move)>,
    excluded_move: Option<Move>,
    tb_prober: Option<&mut dyn TbProber>,
) -> (i32, Option<Move>)
```

## 5. Test Plan

All tests in `crates/search/src/lib.rs` `#[cfg(test)] mod tests`.

**Test 1: `tb_probe_returns_exact_score_in_search`**
- Create a mock `TbProber` that returns `Some(TB_WIN_SCORE)` for `probe_wdl`.
- Run `negamax` at depth 1 on a simple position with low piece count, at `ply = 1`.
- Assert the returned score equals `TB_WIN_SCORE - 1` (ply-adjusted) and the search short-circuits (node count is minimal).

**Test 2: `tb_probe_none_falls_through`**
- Create a mock `TbProber` that returns `None` for all probes.
- Run `negamax` and confirm normal search behavior (non-zero node count, reasonable score).

**Test 3: `root_tb_filters_losing_moves`**
- Create a mock `TbProber` where `probe_root` returns loss for one move and win for another.
- Run `search` and confirm the returned move is the winning one.

**Test 4: `root_tb_prefers_shortest_win`**
- Create a mock `TbProber` where `probe_root` returns win with DTZ=5 for one move and win with DTZ=12 for another.
- Run `search` and confirm the returned move is the one with DTZ=5.

**Test 5: `no_prober_normal_search`**
- Run `search` with `tb_prober: None`.
- Confirm normal search works exactly as before (backward compatibility).

**Test 6: `in_search_probe_skipped_at_root`**
- Create a mock `TbProber` that tracks calls via `Cell<u32>` counter and verify `probe_wdl` is not called at ply 0 (root), only at ply > 0.

**Test 7: `piece_count_gating_delegated_to_prober`**
- Create a mock `TbProber` that tracks whether `probe_wdl` was called (via `Cell<bool>`) and always returns `None`.
- Run `negamax` at `ply = 1` on a position with many pieces (e.g., the starting position with 32 pieces).
- Assert that `probe_wdl` was called exactly once (confirming the search crate always calls through to the prober), and that since the prober returned `None`, normal search proceeded.
- This verifies that piece-count gating is the prober's responsibility (not the search crate's), satisfying verification criterion 4. The actual piece-count gating correctness is covered by T01's `SyzygyTablebase::probe_wdl` tests which check `piece_count(pos) > probe_limit`. The search crate trusts the prober to handle gating.

## 6. Implementation Order

1. **Define `TbProber` trait** in `crates/search/src/lib.rs` — add the trait definition with `probe_wdl` and `probe_root` methods. Define `TB_WIN_THRESHOLD` constant for ply-adjustment logic.

2. **Add `tb_prober` parameter to `negamax`** — extend the function signature. Update all recursive `negamax` calls within `negamax` to pass the prober through (using `tb_prober.as_deref_mut()` pattern or reborrowing). Update `search` to pass it to `negamax`.

3. **Add in-search WDL probing to `negamax`** — after the repetition detection check and before the `depth == 0` quiescence fallback, add the WDL probe logic for `ply > 0`. Apply ply-distance adjustment to mate-scale scores before returning.

4. **Add root tablebase filtering to `search`** — before the iterative deepening loop, generate legal moves, probe each resulting position with `probe_root`, filter losing moves, sort by DTZ, and return immediately if only one move remains. If multiple moves survive, set the DTZ-best as the initial PV hint for the iterative deepening loop.

5. **Update `search` function signature** — add `tb_prober: Option<&mut dyn TbProber>` parameter.

6. **Implement `SyzygyProberAdapter`** in `crates/engine/src/syzygy/mod.rs` — struct wrapping `SyzygyTablebase` and `probe_limit`, implementing `TbProber`.

7. **Update `crates/engine/src/main.rs`** — wire up the `SyzygyProberAdapter` in the `Go` command handler, passing it to `chess_search::search`.

8. **Update existing tests** — fix all existing test call sites to pass `None` for the new `tb_prober` parameter.

9. **Add new tests** — implement all 7 tests specified in the Test Plan.

## 7. Verification Commands

```bash
# Format check
cargo fmt --check --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml

# Lint check
cargo clippy --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml -- -D warnings

# Run all tests (unit tests in search and engine crates)
cargo test --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/Cargo.toml

# Run search crate tests specifically
cargo test --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/crates/search/Cargo.toml

# Run engine crate tests specifically
cargo test --manifest-path /Users/theodhor/Desktop/Portfolio/Chess_Engine/crates/engine/Cargo.toml
```

## 8. Design Deviations

**Deviation 1: Trait-based prober instead of direct dependency**

- **Parent spec requires**: "Modify: `crates/search/` — integrate tablebase probes into the root search loop and in-search logic"
- **Why direct coupling is problematic**: The search crate (`chess-search`) currently does not depend on the engine crate (`chess-engine`), and adding such a dependency would create a circular dependency since the engine crate already depends on the search crate. The `SyzygyTablebase` type lives in the engine crate.
- **Alternative chosen**: Define a `TbProber` trait in the search crate. The engine crate implements this trait for a `SyzygyProberAdapter` wrapper around `SyzygyTablebase`. The search function accepts `Option<&mut dyn TbProber>`. This preserves the existing crate dependency graph and allows clean integration without circular dependencies.

**Deviation 2: `probe_root` combines WDL + DTZ in one method**

- **Parent spec requires**: Separate WDL filtering and DTZ ordering at root.
- **Why separate calls are problematic**: Each root move requires both WDL and DTZ. Making two separate trait method calls per move doubles the virtual dispatch overhead and complicates the interface (the caller must correlate WDL and DTZ results). The underlying `SyzygyTablebase` already provides separate `probe_wdl` and `probe_dtz` methods, but the adapter combines them into a single `probe_root` call for the search layer's convenience.
- **Alternative chosen**: A single `probe_root` method returning `(wdl_score, dtz)` tuple. The search code filters by WDL first, then sorts by DTZ, all from the single result per move.

**Deviation 3: `tb_prober` as parameter rather than `SearchContext` field**

- **Parent spec does not prescribe** where the prober lives, but storing a `dyn TbProber` in `SearchContext` would require either boxing (heap allocation) or lifetime annotations that complicate the struct.
- **Alternative chosen**: Pass `tb_prober: Option<&mut dyn TbProber>` as a parameter through `search` → `negamax`. This avoids lifetime complexity on `SearchContext` and keeps the prober's lifetime cleanly tied to the `search` call scope. The prober is not needed in `quiescence` (piece counts at that point are too high for TB probing to be relevant, and the task spec specifies probing at "low piece counts" in the main search only).