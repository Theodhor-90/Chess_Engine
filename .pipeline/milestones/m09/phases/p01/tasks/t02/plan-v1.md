I have all the context I need. Let me produce the implementation plan.

## 1. Deliverables

- **Modify: `crates/engine/src/book/mod.rs`** — Add `BookMode` enum with `BestMove` and `Weighted` variants. Add `select_book_move` function that accepts a `BookMode` and a slice of `PolyglotEntry` references, returning `Option<Move>` by delegating to `bestmove` or `weighted` strategy. Add internal `select_best` and `select_weighted` helper functions. Add unit tests for selection logic.
- **Modify: `crates/engine/Cargo.toml`** — Add `rand` crate dependency for weighted random selection.

## 2. Dependencies

- **`rand` crate** — Required for weighted random selection in `weighted` mode. Add to `crates/engine/Cargo.toml` under `[dependencies]`. No other crates in the workspace currently use `rand`, so this is a new dependency.
- **T01 (completed)** — `PolyglotEntry` struct (with `raw_move`, `weight` fields) and `polyglot_move_to_engine_move` function are already implemented in `crates/engine/src/book/mod.rs`.

## 3. Implementation Details

### `BookMode` enum

- **Purpose**: Represent the two selection strategies.
- **Location**: `crates/engine/src/book/mod.rs`
- **Definition**:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub enum BookMode {
      BestMove,
      Weighted,
  }
  ```
- **Exports**: Public enum, used by the UCI integration in T03.

### `select_book_move` function

- **Purpose**: The selection interface that accepts a mode parameter and delegates.
- **Signature**:
  ```rust
  pub fn select_book_move(
      pos: &mut Position,
      entries: &[PolyglotEntry],
      mode: BookMode,
  ) -> Option<Move>
  ```
- **Logic**:
  1. If `entries` is empty, return `None`.
  2. Based on `mode`:
     - `BestMove`: Find the entry with the highest `weight` (first encountered on tie). Call `polyglot_move_to_engine_move` on the selected entry's `raw_move`.
     - `Weighted`: Use `rand` to select an entry with probability proportional to `weight`. Compute the total weight sum, generate a random number in `[0, total_weight)`, iterate entries accumulating weight until the threshold is crossed. Entries with weight 0 naturally get zero probability. If total weight is 0 (all entries have weight 0), return `None`. Call `polyglot_move_to_engine_move` on the selected entry's `raw_move`.
  3. Return the resulting `Option<Move>`.

- **Why `pos` is `&mut Position`**: The function calls `polyglot_move_to_engine_move`, which takes `&mut Position` (because it calls `generate_legal_moves` which requires `&mut Position`). This matches the existing T01 signature.

### `select_weighted` internal logic detail

- Use `rand::thread_rng()` and `rand::Rng::gen_range` (or `Rng::random_range`) to generate a random `u32` in `[0, total_weight)` where `total_weight` is the `u32` sum of all `weight` fields (cast from `u16`). Iterate entries, accumulating weight. When the accumulator exceeds the random threshold, select that entry.
- `u32` sum avoids overflow since max possible sum is 65535 * N entries, which fits comfortably in `u32`.

## 4. API Contracts

### `BookMode` enum
```rust
pub enum BookMode {
    BestMove,
    Weighted,
}
```

### `select_book_move`
```rust
pub fn select_book_move(
    pos: &mut Position,
    entries: &[PolyglotEntry],
    mode: BookMode,
) -> Option<Move>
```

**Inputs:**
- `pos`: Mutable reference to the current `Position` (needed for move translation via legal move generation).
- `entries`: Slice of `PolyglotEntry` records for the current position (as returned by `PolyglotBook::probe`).
- `mode`: `BookMode::BestMove` or `BookMode::Weighted`.

**Output:**
- `Some(Move)` — the selected engine move.
- `None` — if `entries` is empty, all weights are 0 (in weighted mode), or the move translation fails.

**Examples:**
```rust
// BestMove: returns the move with weight 200
let entries = [
    PolyglotEntry { key: 0, raw_move: 0x0A0B, weight: 100, learn: 0 },
    PolyglotEntry { key: 0, raw_move: 0x0C0D, weight: 200, learn: 0 },
];
let mv = select_book_move(&mut pos, &entries, BookMode::BestMove);
// mv corresponds to the entry with weight 200

// Empty entries
let mv = select_book_move(&mut pos, &[], BookMode::BestMove);
assert!(mv.is_none());
```

## 5. Test Plan

All tests are colocated in `crates/engine/src/book/mod.rs` within the existing `#[cfg(test)] mod tests` block.

### Test 1: `bestmove_returns_highest_weight`
- **Setup**: Construct a `Vec<PolyglotEntry>` with 3 entries having distinct weights. Use a position (e.g., startpos) and valid Polyglot raw_move values that correspond to legal moves (e.g., e2→e4, d2→d4, g1→f3).
- **Assertion**: `select_book_move` with `BookMode::BestMove` returns the move corresponding to the entry with the highest weight.

### Test 2: `bestmove_deterministic_tie_breaking`
- **Setup**: Construct entries with two equal-highest weights. Use valid moves from startpos.
- **Assertion**: `select_book_move` with `BookMode::BestMove` returns the move of the first entry with the highest weight (deterministic, first-encountered).

### Test 3: `empty_entries_returns_none`
- **Setup**: Empty slice of entries.
- **Assertion**: `select_book_move` returns `None` for both `BookMode::BestMove` and `BookMode::Weighted`.

### Test 4: `weighted_distribution_consistent`
- **Setup**: Construct entries with known weights (e.g., weight 900 and weight 100). Run `select_book_move` with `BookMode::Weighted` many times (e.g., 10,000 iterations) from a startpos.
- **Assertion**: The move corresponding to the high-weight entry is selected significantly more often than the low-weight entry (e.g., the high-weight move is selected >80% of the time). This is a statistical test with very wide margins to avoid flakiness.

### Test 5: `single_entry_always_selected`
- **Setup**: One entry with non-zero weight.
- **Assertion**: `select_book_move` with `BookMode::Weighted` returns the corresponding move.

## 6. Implementation Order

1. **Add `rand` dependency** to `crates/engine/Cargo.toml`.
2. **Define `BookMode` enum** in `crates/engine/src/book/mod.rs`.
3. **Implement `select_book_move` function** in `crates/engine/src/book/mod.rs`, with the internal best/weighted logic.
4. **Write unit tests** in the existing `#[cfg(test)] mod tests` block in `crates/engine/src/book/mod.rs`.
5. **Run verification commands** to confirm correctness.

## 7. Verification Commands

```bash
# Format check
cargo fmt --check

# Lint check
cargo clippy -- -D warnings

# Run all tests (including new book selection tests)
cargo test -p engine

# Run only book-related tests
cargo test -p engine book
```

## 8. Design Deviations

**Deviation: `select_book_move` takes `&mut Position` instead of returning raw moves**

- **Parent spec requires**: "A selection interface that accepts the mode as a parameter and delegates to the `bestmove` or `weighted` strategy based on the provided mode parameter."
- **Why pure entry-only selection is insufficient**: The task spec states "The move returned should be the engine's internal `Move` type (using the translation from T01)." The T01 translation function `polyglot_move_to_engine_move` requires `&mut Position` because it calls `generate_legal_moves` to find the matching legal move. Therefore, the selection function must accept `&mut Position` to perform the translation and return `Option<Move>` rather than `Option<PolyglotEntry>`.
- **Alternative chosen**: `select_book_move(pos: &mut Position, entries: &[PolyglotEntry], mode: BookMode) -> Option<Move>` encapsulates both selection and translation in a single call, which is the natural interface for T03 (UCI integration) to consume.