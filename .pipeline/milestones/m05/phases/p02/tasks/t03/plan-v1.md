Now I have all the context I need. Let me produce the implementation plan.

## 1. Deliverables

| File | Action | Purpose |
|------|--------|---------|
| `crates/search/src/lib.rs` | Modify | Add `history: Vec<u64>` field to `SearchContext`; add repetition detection (threefold) and fifty-move rule check at the start of `negamax` before the TT probe; update `search` function signature to accept `game_history: &[u64]`; push/pop hashes around `make_move`/`unmake_move` in both `negamax` and `quiescence`; add tests for repetition detection, fifty-move rule, and search behavior around repetitions |
| `crates/engine/src/main.rs` | Modify | Build a `Vec<u64>` of Zobrist hashes from the UCI `position` command's move list and pass it to `search` |

## 2. Dependencies

No new external crates or packages are required. This task depends on:

- `chess_board::Position` — `hash()` method returns `u64`, `halfmove_clock()` returns `u8`, `make_move` returns `UndoInfo`, `unmake_move` restores state
- `chess_movegen::generate_legal_moves` — used by search
- `chess_eval::evaluate` — used by quiescence search
- Sibling tasks T01 (TT move legality validation) and T02 (IID) are already implemented in `crates/search/src/lib.rs`

## 3. Implementation Details

### `crates/search/src/lib.rs`

**SearchContext changes:**

Add a `history: Vec<u64>` field to the `SearchContext` struct. This stores Zobrist hashes of all positions reached — both from the game history (pre-search moves from UCI) and from positions reached during search.

```rust
pub struct SearchContext {
    // ... existing fields ...
    history: Vec<u64>,
}
```

**`search` function signature change:**

Update `search` to accept `game_history: &[u64]`:

```rust
pub fn search(
    pos: &mut Position,
    limits: SearchLimits,
    game_history: &[u64],
    on_depth: Option<DepthCallback<'_>>,
) -> Option<Move>
```

Inside `search`, initialize `ctx.history` from `game_history`:

```rust
let mut ctx = SearchContext {
    // ... existing fields ...
    history: game_history.to_vec(),
};
```

**Repetition detection in `negamax`:**

At the start of `negamax`, after incrementing the node counter and checking for abort, but **before** the TT probe and before the `depth == 0` check, add:

```rust
// Repetition and fifty-move rule detection (skip at root ply 0)
if ply > 0 {
    // Fifty-move rule
    if pos.halfmove_clock() >= 100 {
        return (0, None);
    }

    // Threefold repetition: scan backward through history
    let current_hash = pos.hash();
    let halfmove = pos.halfmove_clock() as usize;
    let mut repetitions = 0;
    // Only check positions since last irreversible move
    let start = if ctx.history.len() > halfmove {
        ctx.history.len() - halfmove
    } else {
        0
    };
    for i in (start..ctx.history.len()).rev().skip(1).step_by(2) {
        if ctx.history[i] == current_hash {
            repetitions += 1;
            if repetitions >= 2 {
                return (0, None);
            }
        }
    }
}
```

Key details:
- The `step_by(2)` optimization: positions alternate side-to-move, so only every other position can match (same side to move means same Zobrist side-to-move XOR).
- `skip(1)` skips the most recent entry (which is the current position itself, already pushed before the recursive call).
- We scan backward only up to `halfmove_clock` positions, since a capture or pawn move resets the clock and makes earlier repetitions impossible.
- `repetitions >= 2` means the current position has appeared at least 2 times before (i.e., this is the third occurrence = threefold repetition). In practice for search, a single repetition (`>= 1`) is often used (twofold detection), which is standard engine practice to avoid entering drawn lines. However, the spec says "threefold repetition" so we check for `>= 2` prior occurrences.

**Note on twofold vs threefold:** The spec says "Threefold repetition: if the current hash appears 2+ times previously in the history, it's a threefold repetition." This means we need 2 prior occurrences (current = 3rd occurrence). However, standard chess engine practice is to use twofold detection during search (1 prior occurrence = return draw) to avoid entering repetition lines. The spec explicitly states "2+ times previously" so we use `>= 2`. But if a test requires the engine to *avoid* repeating when winning, twofold detection is needed. We will use **twofold** detection during search (`repetitions >= 1` means we've seen it once before, so playing into it again would allow threefold — engine should avoid this). This matches standard engine behavior and the test criteria (c) and (d).

**Actually, re-reading the spec:** "Threefold repetition: if the current hash appears 2+ times previously in the history, it's a threefold repetition." This is the strict definition. But test criteria (c) says "engine avoids repeating positions when winning" and (d) says "engine seeks repetition when losing" — this behavior requires the engine to detect twofold (a single prior occurrence) during search to proactively avoid/seek draws. Standard practice: use `repetitions >= 1` in search to return draw score when a position has been seen before. This is how engines handle it — they treat twofold as a draw during search to prevent entering lines that lead to threefold. We will use `>= 1`.

**Push/pop hashes in `negamax`:**

Around each `make_move`/`unmake_move` call in the move loop:

```rust
ctx.history.push(pos.hash());  // push AFTER make_move (new position hash)
let (score, _) = negamax(pos, depth - 1, -beta, -alpha, ply + 1, ctx);
let score = -score;
ctx.history.pop();  // pop before unmake_move
pos.unmake_move(mv, undo);
```

Wait — we need to push the hash of the position *after* making the move, since that's the position being searched. The current position's hash is already in the history (pushed by the caller). So:

```rust
let undo = pos.make_move(mv);
ctx.history.push(pos.hash());
let (score, _) = negamax(pos, depth - 1, -beta, -alpha, ply + 1, ctx);
let score = -score;
ctx.history.pop();
pos.unmake_move(mv, undo);
```

**Push/pop hashes in `quiescence`:**

Same pattern around each `make_move`/`unmake_move` in quiescence search:

```rust
let undo = pos.make_move(mv);
ctx.history.push(pos.hash());
let score = -quiescence(pos, -beta, -alpha, ply + 1, ctx);
ctx.history.pop();
pos.unmake_move(mv, undo);
```

Also add a repetition/fifty-move check at the start of `quiescence` (positions can repeat during qsearch too, though rare):

Actually, quiescence only searches captures and promotions — these are irreversible moves (captures reset halfmove clock), so repetition cannot occur in quiescence. No changes needed to quiescence for repetition detection.

**Push initial position hash in `search`:**

Before the iterative deepening loop, push the current position's hash:

```rust
ctx.history.push(pos.hash());
```

This ensures the starting search position is in the history stack.

**IID recursive call also needs push/pop:**

The IID call in `negamax` does NOT make a move — it searches the same position at reduced depth. The current position's hash is already in `ctx.history` (pushed by the caller). No additional push/pop is needed for the IID call since it operates on the same position.

**Update `test_ctx` helper and all test call sites:**

The `test_ctx()` helper needs the new `history` field:

```rust
fn test_ctx() -> SearchContext {
    SearchContext {
        // ... existing fields ...
        history: Vec::new(),
    }
}
```

All existing tests that construct `SearchContext` manually need the `history` field added. All calls to `search()` need an empty `&[]` game history argument added.

### `crates/engine/src/main.rs`

**Build game history in `Position` command handler:**

When processing `UciCommand::Position`, build a `Vec<u64>` of Zobrist hashes as moves are applied:

```rust
chess_uci::UciCommand::Position { fen, moves } => {
    let mut pos = if let Some(fen_str) = fen {
        match Position::from_fen(&fen_str) {
            Ok(p) => p,
            Err(_) => continue,
        }
    } else {
        Position::startpos()
    };
    let mut game_history: Vec<u64> = vec![pos.hash()];
    for move_str in &moves {
        if let Some(mv) = parse_uci_move(&mut pos, move_str) {
            let _ = pos.make_move(mv);
            game_history.push(pos.hash());
        }
    }
    state.position = pos;
    state.game_history = game_history;
}
```

**Add `game_history` field to `EngineState`:**

```rust
struct EngineState {
    position: Position,
    game_history: Vec<u64>,
    // ... other existing fields ...
}
```

Initialize in `main()`:

```rust
let mut state = EngineState {
    position: Position::startpos(),
    game_history: vec![Position::startpos().hash()],
    // ... other existing fields ...
};
```

**Pass game history to `search`:**

In the `Go` command handler, clone the game history and pass it to the search thread:

```rust
let game_history = state.game_history.clone();
state.search_handle = Some(std::thread::spawn(move || {
    let result = chess_search::search(
        &mut search_pos,
        limits,
        &game_history,
        Some(&|depth, score, nodes, elapsed, pv| { /* ... */ }),
    );
    // ...
}));
```

## 4. API Contracts

**`search` function (updated signature):**

```rust
pub fn search(
    pos: &mut Position,
    limits: SearchLimits,
    game_history: &[u64],
    on_depth: Option<DepthCallback<'_>>,
) -> Option<Move>
```

- `game_history`: Slice of Zobrist hashes from the game's move history, starting from the initial position. The current position's hash should be the last element. If empty (e.g., in tests), no pre-search history is considered.
- Return value and all other parameters remain unchanged.

## 5. Test Plan

All tests are added to the existing `#[cfg(test)] mod tests` in `crates/search/src/lib.rs`.

### Test (a): `threefold_repetition_returns_draw`

**Setup:** Construct a position and a game history where the same position hash appears twice already. Push these into the `SearchContext.history`. Then search at depth 1+.

**Approach:** Use a simple position (e.g., kings + a piece). Play moves that create a repeated position: Ke1→Ke2→Ke1→Ke2→... Build the history manually with the hashes. When search encounters the position a third time, it should return score 0.

```
FEN: "8/8/8/8/8/8/8/4K2k w - - 0 1" (or similar)
```

Play: Ke1-e2, Kh1-g1, Ke2-e1, Kg1-h1 (back to start). Do this twice so the starting position appears 3 times. The search should return 0 for the repeated position.

**Concrete approach:** Use `search()` with a game history containing two prior occurrences of the current position hash. Verify the search returns a draw-like score or that `negamax` directly returns 0 for a repeated position.

**Implementation:** Call `negamax` with a `ctx.history` pre-loaded with the current position's hash appearing once before. The check `repetitions >= 1` will trigger and return `(0, None)` at `ply > 0`. For a root-level test, set up iterative deepening where a child position repeats.

### Test (b): `fifty_move_rule_returns_draw`

**Setup:** Construct a position with `halfmove_clock = 100` (via FEN). Search should return draw score 0.

```
FEN: "8/8/8/8/8/8/8/4K2k w - - 100 51"
```

Call `negamax` at depth 1 with `ply > 0` — it should detect halfmove clock >= 100 and return 0. Alternatively, test via `search()` — the root position has halfmove clock 100, but detection is at `ply > 0`, so the children inherit the clock value. Actually, on `make_move`, the halfmove clock increments from 100 to 101 (or resets on capture/pawn move). So set the FEN halfmove clock to 99, then any quiet move will make it 100, and the child node will detect it.

Better approach: Set halfmove clock to 100 in the FEN. At the root (ply=0), detection is skipped. The children will have halfmove clock = 101 (incremented by quiet move) which is >= 100, so they return draw. This means the root sees all children returning 0.

### Test (c): `engine_avoids_repetition_when_winning`

**Setup:** Position where White has a large material advantage (e.g., an extra queen). The game history contains one prior occurrence of the current position. The engine should prefer a non-repeating move to avoid a draw.

Verify that the search returns a move and a score > 0 (not draw).

### Test (d): `engine_seeks_repetition_when_losing`

**Setup:** Position where the side to move is significantly behind in material, and one legal move leads to a position that has appeared before in the history (drawing by repetition). The engine should prefer the repeating move (score = 0) over continuing in a losing position.

Verify that the search returns a score of 0 (draw) or near 0, rather than a deeply negative score.

### Test setup:

All tests use the existing `test_ctx()` helper (updated with `history` field) or construct `SearchContext` directly. The `search()` tests pass `game_history` as `&[u64]`.

### Existing test updates:

All existing tests that call `search()` must pass an empty `&[]` as `game_history`. All existing tests that construct `SearchContext` manually must add `history: Vec::new()`. These are mechanical changes.

## 6. Implementation Order

1. **Add `history: Vec<u64>` field to `SearchContext`** — update the struct definition, `test_ctx()`, and all manual `SearchContext` constructions in tests to include `history: Vec::new()`.

2. **Update `search` function signature** — add `game_history: &[u64]` parameter, initialize `ctx.history` from it, push the current position's hash before iterative deepening. Update all existing test calls to `search()` to pass `&[]`.

3. **Add repetition and fifty-move detection in `negamax`** — insert the check at the start of `negamax`, after node counting and abort check, before `depth == 0` and TT probe. Check `ply > 0` to skip at root.

4. **Add push/pop of hashes in `negamax` move loop** — push `pos.hash()` after `make_move`, pop before `unmake_move`.

5. **Add push/pop of hashes in `quiescence` move loop** — not strictly necessary (captures reset halfmove clock, making repetition impossible), but included for correctness in edge cases. Actually, skip this — captures are irreversible, so no repetition in qsearch.

6. **Update `crates/engine/src/main.rs`** — add `game_history: Vec<u64>` to `EngineState`, build it in `Position` handler, pass to `search` in `Go` handler. Update `UciNewGame` handler to reset game history.

7. **Add tests** — `threefold_repetition_returns_draw`, `fifty_move_rule_returns_draw`, `engine_avoids_repetition_when_winning`, `engine_seeks_repetition_when_losing`.

8. **Run `cargo test` and `cargo clippy`** — verify all tests pass and no warnings.

## 7. Verification Commands

```bash
# Run all tests in the search crate
cargo test -p chess-search

# Run the specific new tests
cargo test -p chess-search threefold_repetition
cargo test -p chess-search fifty_move_rule
cargo test -p chess-search engine_avoids_repetition
cargo test -p chess-search engine_seeks_repetition

# Run the full workspace tests
cargo test --workspace

# Check for lint warnings
cargo clippy --workspace -- -D warnings

# Check formatting
cargo fmt --check
```

## 8. Design Deviations

**Deviation 1: Twofold detection instead of strict threefold in search**

- **Parent spec requires:** "Threefold repetition: if the current hash appears 2+ times previously in the history, it's a threefold repetition" (i.e., 3 total occurrences).
- **Why strict threefold is problematic for search:** The exit criteria (c) and (d) require the engine to *avoid* repeating positions when winning and *seek* repetition when losing. With strict threefold detection (requiring 2 prior occurrences), the engine would need to enter a position twice before recognizing the draw, meaning it would willingly enter repeated positions. Standard chess engines universally use twofold detection in search (1 prior occurrence → draw score) to proactively steer toward or away from repetitions.
- **Alternative chosen:** Use `repetitions >= 1` (twofold) — if the current position has appeared once before in the history, return draw score 0. This satisfies exit criteria (a) threefold is detected (a position appearing 3 times always triggers, since on the 3rd occurrence there are 2 prior), (c) engine avoids repeating when winning, and (d) engine seeks repetition when losing. Twofold is a superset of threefold detection — it catches all threefold cases plus prevents entering them in the first place.

**Deviation 2: No repetition check in quiescence search**

- **Parent spec says:** "During search, `make_move` pushes the new hash and `unmake_move` pops it" (implies all search nodes).
- **Why adding push/pop to quiescence is unnecessary:** Quiescence search only examines captures and promotions. Both are irreversible moves (captures reset the halfmove clock). A position cannot repeat through a sequence of captures. Adding push/pop overhead to every quiescence node would slow search with zero benefit.
- **Alternative chosen:** Push/pop hashes only in `negamax`. Quiescence search does not modify `ctx.history`. The repetition check only exists in `negamax`, which is never reached from quiescence (quiescence does not call `negamax`).