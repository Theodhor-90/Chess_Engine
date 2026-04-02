I now have everything I need. Let me produce the implementation plan.

## 1. Deliverables

- **Modify: `crates/search/src/lib.rs`** — Add a `SyzygyProber` trait and an optional tablebase prober field to `SearchContext`. Integrate WDL probing into `negamax` at non-root nodes (piece count gated). Implement root-level tablebase filtering in a new `search_root_tb` helper called from the `search` function. Modify the `search` function signature to accept an optional tablebase prober.

- **Modify: `crates/search/Cargo.toml`** — Add dependency on `chess-engine` (or restructure) — **Not needed.** The search crate should remain independent. Instead, use a trait-based approach: define a `SyzygyProber` trait in the search crate so that the engine crate can implement it and pass it in. This avoids circular dependencies.

- **Modify: `crates/engine/src/main.rs`** — Pass the `SyzygyTablebase` (wrapped in the `SyzygyProber` trait) to the `search` function when tablebase data is available.

## 2. Dependencies

- **T01 (completed)**: The `SyzygyTablebase` type in `crates/engine/src/syzygy/mod.rs` with `probe_wdl`, `probe_dtz`, and `wdl_to_score` functions.
- **`chess-movegen`**: Already a dependency of `chess-search`; provides `generate_legal_moves`.
- **`chess-board`**: Already a dependency of `chess-search`; provides `Position` with `occupied().pop_count()`, `make_move`, `unmake_move`.
- No new external crate dependencies are required.

## 3. Implementation Details

### `crates/search/src/lib.rs`

**New trait: `TbProber`**

Purpose: Abstracts tablebase probing so the search crate doesn't depend on the engine crate.

```rust
pub trait TbProber {
    /// Probe WDL for a position. Returns None if piece count exceeds limit or probe fails.
    fn probe_wdl(&mut self, pos: &Position) -> Option<i32>;
    /// Probe WDL for each root move. Returns (wdl_score, dtz) or None.
    fn probe_root(&mut self, pos: &Position) -> Option<(i32, i32)>;
}
```

- `probe_wdl` returns an already-converted score (the implementor calls `wdl_to_score` internally), or `None` on failure/piece-count exceeded.
- `probe_root` returns `(wdl_score, dtz)` for root-level DTZ ordering, or `None` on failure.

**Modified `SearchContext`**

No changes to `SearchContext` itself. The prober is passed as a parameter.

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

1. Generate legal moves.
2. For each move, make it, call `tb_prober.probe_root()` on the resulting position, unmake it.
3. Collect `(move, wdl_score, dtz)` for all moves where probing succeeds.
4. If all moves were probed:
   - Find the best WDL category among all moves (from the side-to-move's perspective: negate scores since probe is from opponent's POV after making the move).
   - Filter out moves whose negated WDL score is less than the best.
   - Among remaining moves (same WDL), prefer lowest DTZ for wins (fastest win), highest DTZ for losses (longest to lose).
   - If exactly one move remains, return it immediately. Otherwise, continue search with only the filtered moves as a hint (or just set the best as root move).
5. If probing fails for any move, fall through to normal search.

**In-search WDL probing** (inside `negamax`, at non-root nodes after repetition check, before depth==0 qsearch):

1. If `tb_prober` is `Some` and `ply > 0`:
   - Call `tb_prober.probe_wdl(pos)`.
   - If `Some(score)`, return `(score, None)` immediately — this is the exact tablebase result.
2. If `None`, fall through to normal search.

**Key logic details:**

- After making a root move, the resulting position's WDL is from the opponent's perspective. Negate the score: if the opponent is in a loss, the side to move has a win.
- DTZ is used to rank among equal-WDL moves: for wins, prefer move with smallest absolute DTZ (fastest conversion); for losses, prefer move with largest absolute DTZ (slowest loss).
- The `tb_prober` parameter is threaded through `negamax` as well; the function signature gains `tb_prober: Option<&mut dyn TbProber>`.

### `crates/engine/src/syzygy/mod.rs`

**New: `impl TbProber for SyzygyTablebase`** (or a wrapper struct)

Since `TbProber` is defined in `chess-search`, the engine crate implements it for `SyzygyTablebase`:

```rust
use chess_search::TbProber;

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

A `SyzygyProberAdapter` struct wraps `SyzygyTablebase` and the configured `probe_limit: u8`.

### `crates/engine/src/main.rs`

- In the `Go` command handler, construct a `SyzygyProberAdapter` if tablebase is available, and pass `Some(&mut adapter)` to `chess_search::search`. Otherwise pass `None`.
- This requires that the engine state holds an optional `SyzygyTablebase` (for T03's UCI option work, but for now the wiring just needs to exist so a prober can be passed).

Note: The task spec says "Possibly modify: Root move filtering logic in the search or engine crate." The root filtering will live in the `search` function itself since that's where the iterative deepening root loop is.

## 4. API Contracts

**`TbProber` trait** (defined in `crates/search/src/lib.rs`):

```rust
pub trait TbProber {
    /// Probe WDL at a non-root node. Returns converted score or None.
    fn probe_wdl(&mut self, pos: &Position) -> Option<i32>;
    /// Probe WDL + DTZ at root. Returns (wdl_score, dtz) or None.
    fn probe_root(&mut self, pos: &Position) -> Option<(i32, i32)>;
}
```

Input: `&Position` (the current board position).

Output for `probe_wdl`: `Some(i32)` where the value is a converted score (`TB_WIN_SCORE`, `0`, `-TB_WIN_SCORE`, `1`, `-1`) — or `None` if probing failed or piece count exceeds limit.

Output for `probe_root`: `Some((wdl_score, dtz))` — or `None` on failure.

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

**Modified `negamax` function** (internal, not pub-exported for trait usage):

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
- Run `negamax` at depth 1 on a simple position with low piece count.
- Assert the returned score equals `TB_WIN_SCORE` and the search short-circuits (node count is minimal).

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
- Create a mock `TbProber` and verify `probe_wdl` is not called at ply 0 (root), only at ply > 0.

## 6. Implementation Order

1. **Define `TbProber` trait** in `crates/search/src/lib.rs` — add the trait definition with `probe_wdl` and `probe_root` methods.

2. **Add `tb_prober` parameter to `negamax`** — extend the function signature. Update all recursive `negamax` calls within `negamax` to pass the prober through (using `tb_prober.as_deref_mut()` pattern or reborrowing). Update `search` to pass it to `negamax`.

3. **Add in-search WDL probing to `negamax`** — after the repetition detection check and before the `depth == 0` quiescence fallback, add the WDL probe logic for `ply > 0`.

4. **Add root tablebase filtering to `search`** — before the iterative deepening loop, generate legal moves, probe each resulting position with `probe_root`, filter losing moves, sort by DTZ, and return immediately if the best move is unambiguous, or pass the TB-best move to the search as a hint.

5. **Update `search` function signature** — add `tb_prober: Option<&mut dyn TbProber>` parameter.

6. **Implement `SyzygyProberAdapter`** in `crates/engine/src/syzygy/mod.rs` — struct wrapping `SyzygyTablebase` and `probe_limit`, implementing `TbProber`.

7. **Update `crates/engine/src/main.rs`** — wire up the `SyzygyProberAdapter` in the `Go` command handler, passing it to `chess_search::search`.

8. **Update existing tests** — fix all existing test call sites to pass `None` for the new `tb_prober` parameter.

9. **Add new tests** — implement the tests specified in the Test Plan.

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