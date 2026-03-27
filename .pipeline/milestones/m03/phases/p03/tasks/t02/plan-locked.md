## 1. Deliverables

| File | Action | Purpose |
|------|--------|---------|
| `crates/search/src/killer.rs` | Create | Killer move table implementation: 2 slots per ply, storage, retrieval, slot rotation, and clearing. |
| `crates/search/src/ordering.rs` | Modify | Integrate killer move scoring into the `order_moves` function so killers are prioritized below captures but above quiet moves. |
| `crates/search/src/lib.rs` | Modify | Declare `killer` module, extend `SearchContext` with a `KillerTable` field, record killers on beta cutoff of quiet moves in `negamax`, pass killer table and ply to `order_moves`, clear killer table in `search`. |

## 2. Dependencies

No new external dependencies. All required crates are already in `crates/search/Cargo.toml`:
- `chess-types` — `Move`, `MoveFlag`
- `chess-board` — `Position`
- `chess-movegen` — `generate_legal_moves`
- `chess-eval` — `piece_value` (used by existing MVV-LVA)

## 3. Implementation Details

### `crates/search/src/killer.rs`

**Purpose**: Encapsulate the killer move table — a fixed-size array of 2 killer move slots per ply.

**Constants**:
- `MAX_PLY: usize = 128` — maximum search depth supported (standard chess engine convention).
- `NUM_KILLERS: usize = 2` — two slots per ply.

**Struct**: `KillerTable`
```rust
pub struct KillerTable {
    table: [[Option<Move>; NUM_KILLERS]; MAX_PLY],
}
```

**Exports/Methods**:
- `KillerTable::new() -> KillerTable` — returns a table with all slots set to `None`.
- `KillerTable::store(&mut self, ply: u8, mv: Move)` — stores a killer move at the given ply. Skips if `mv` is already the first killer. Otherwise, shifts the first killer to the second slot and places `mv` in the first slot. Only quiet (non-capture) moves should be stored — the caller is responsible for this check.
- `KillerTable::is_killer(&self, ply: u8, mv: Move) -> bool` — returns `true` if `mv` matches either killer slot at the given ply.
- `KillerTable::clear(&mut self)` — resets all slots to `None`.

**Key logic for `store`**:
1. If `mv == self.table[ply][0]`, return early (already stored as first killer).
2. Shift: `self.table[ply][1] = self.table[ply][0]`.
3. Insert: `self.table[ply][0] = Some(mv)`.

**Key logic for `is_killer`**:
1. Check `self.table[ply][0] == Some(mv) || self.table[ply][1] == Some(mv)`.

**Pseudo-legality validation**: The task spec requires validating killers as pseudo-legal before use. Since killer moves originate from sibling nodes (same ply in the search tree), they share the same side to move. Validation is done by checking that the move appears in the current legal move list. Since `order_moves` operates on the already-generated legal move list, a killer is validated implicitly — `is_killer` is called on moves already in the legal move list, so any move that matches a killer slot is guaranteed to be legal in the current position. This is the standard approach: rather than pre-filtering killers, we boost the score of legal moves that match a killer.

### `crates/search/src/ordering.rs`

**Modifications**: Extend `order_moves` to accept a `KillerTable` reference and current ply, so that killer moves receive a score between captures and quiet moves.

**Updated signature**:
```rust
pub fn order_moves(moves: &mut [Move], pos: &Position, killers: &KillerTable, ply: u8)
```

**Scoring constants**:
- `KILLER_SCORE: i32 = 50` — killers score above quiet moves (0) but well below even the lowest-value capture. The minimum MVV-LVA capture score is PxP = `100 * 100 - 100 = 9900`, so any positive constant below that works. Using 50 provides clear separation.

**Updated scoring logic** (in the sort comparator or a unified `score_move` function):
1. If `mv.is_capture()`, return `score_mvv_lva(mv, pos)` (unchanged, produces values ≥ 9900).
2. Else if `killers.is_killer(ply, mv)`, return `KILLER_SCORE` (50).
3. Else return `0` (quiet move).

**Refactoring approach**: Replace the closure-based sort in `order_moves` with a call to a new private `score_move` helper that encapsulates the full scoring logic (MVV-LVA + killer), then sort by descending score. The `score_mvv_lva` public function remains unchanged for backward compatibility and direct use in quiescence.

### `crates/search/src/lib.rs`

**Modifications**:

1. **Module declaration**: Add `pub mod killer;` alongside existing `pub mod ordering;`.

2. **SearchContext extension**: Add a `killers: killer::KillerTable` field to the `SearchContext` struct.

3. **SearchContext initialization**: In the `search` function, initialize `killers: killer::KillerTable::new()` in the `SearchContext` constructor. The same killer table accumulates useful data across iterative deepening iterations (standard practice), so it is not cleared between iterations.

4. **negamax — record killers on beta cutoff**: In the `negamax` function, when `alpha >= beta` (beta cutoff), if the move is **not** a capture (`!mv.is_capture()`), call `ctx.killers.store(ply, mv)`.

5. **negamax — pass killers to ordering**: Change the call `ordering::order_moves(&mut moves, pos)` to `ordering::order_moves(&mut moves, pos, &ctx.killers, ply)`.

6. **quiescence — no change to ordering call**: Quiescence search only searches captures/promotions, so killer moves (which are quiet) do not apply. However, the `order_moves` signature changed, so the call in `quiescence` needs to be updated. Pass `&ctx.killers` and `ply` through — the killer check will simply never match on capture moves, which is harmless.

7. **test_ctx helper**: Update `test_ctx()` in tests to include the `killers` field initialized via `KillerTable::new()`.

## 4. API Contracts

**`KillerTable::new() -> KillerTable`**
- Input: none
- Output: `KillerTable` with all 128×2 slots set to `None`

**`KillerTable::store(&mut self, ply: u8, mv: Move)`**
- Input: `ply` (0–127), `mv` (a quiet move that caused beta cutoff)
- Output: none (mutates table in place)
- Example: After `store(3, Move::new(Square::E2, Square::E4, MoveFlag::QUIET))`, `is_killer(3, that_move)` returns `true`.

**`KillerTable::is_killer(&self, ply: u8, mv: Move) -> bool`**
- Input: `ply` (0–127), `mv` (any move)
- Output: `true` if `mv` matches either killer slot at that ply, `false` otherwise

**`KillerTable::clear(&mut self)`**
- Input: none
- Output: none (resets all slots to `None`)

**`order_moves(moves: &mut [Move], pos: &Position, killers: &KillerTable, ply: u8)`**
- Input: mutable slice of legal moves, position reference, killer table reference, current ply
- Output: none (sorts `moves` in place by descending score: captures by MVV-LVA first, then killers, then quiet moves)

## 5. Test Plan

### `crates/search/src/killer.rs` — unit tests

**Test 1: `store_and_retrieve`**
- Setup: Create `KillerTable::new()`. Create two distinct quiet moves.
- Action: Store move A at ply 5. Check `is_killer(5, A)` returns `true`. Store move B at ply 5. Check both `is_killer(5, A)` and `is_killer(5, B)` return `true`.
- Verifies: Storage and retrieval work; slot rotation preserves both killers.

**Test 2: `slot_rotation_evicts_oldest`**
- Setup: Create `KillerTable::new()`. Create three distinct quiet moves A, B, C.
- Action: Store A at ply 3, then B at ply 3, then C at ply 3.
- Assert: `is_killer(3, C)` is `true`, `is_killer(3, B)` is `true`, `is_killer(3, A)` is `false`.
- Verifies: Third store evicts the oldest (A was shifted to slot 1, then B replaced it when C was stored).

**Test 3: `duplicate_not_shifted`**
- Setup: Create `KillerTable::new()`. Create two moves A and B.
- Action: Store A at ply 4, store B at ply 4, store A again at ply 4.
- Assert: `is_killer(4, A)` is `true` and `is_killer(4, B)` is `true` (B was not evicted because A was already in slot 0 and the store was a no-op).
- Verifies: Storing a duplicate of the first killer does not shift.

**Test 4: `killers_per_ply_independent`**
- Setup: Store move A at ply 0, move B at ply 1.
- Assert: `is_killer(0, A)` is `true`, `is_killer(1, A)` is `false`, `is_killer(1, B)` is `true`, `is_killer(0, B)` is `false`.
- Verifies: Killer storage is per-ply.

**Test 5: `clear_resets_all`**
- Setup: Store move A at ply 0, move B at ply 5, move C at ply 10. Call `clear()`.
- Assert: `is_killer(0, A)` is `false`, `is_killer(5, B)` is `false`, `is_killer(10, C)` is `false`.
- Verifies: Clear resets all slots across all plies.

### `crates/search/src/ordering.rs` — unit tests

**Test 6: `killer_moves_ordered_between_captures_and_quiet`**
- Setup: A position with both captures and quiet moves available. Store one of the quiet moves as a killer for ply 0.
- Action: Call `order_moves` with the killer table and ply 0.
- Assert: All captures appear before the killer move, and the killer move appears before other quiet moves.
- Verifies: Exit criterion #3 — killers are prioritized below captures but above quiet moves.

### `crates/search/src/lib.rs` — existing tests

All existing tests must continue to pass with the updated `SearchContext` struct and `order_moves` signature. No new integration tests are added beyond confirming existing tests compile and pass.

## 6. Implementation Order

1. **Create `crates/search/src/killer.rs`** — implement `KillerTable` struct with `new`, `store`, `is_killer`, `clear` methods and unit tests.
2. **Modify `crates/search/src/ordering.rs`** — update `order_moves` signature to accept `&KillerTable` and `ply`, add killer scoring logic, add the `killer_moves_ordered_between_captures_and_quiet` test.
3. **Modify `crates/search/src/lib.rs`** — declare `pub mod killer`, add `killers` field to `SearchContext`, update `negamax` to record killers on quiet beta cutoffs, update all `order_moves` call sites (in both `negamax` and `quiescence`), update `test_ctx` helper and any tests that construct `SearchContext`.

## 7. Verification Commands

```bash
# Format check
cargo fmt --check

# Lint check
cargo clippy -- -D warnings

# Run all tests (includes search crate unit tests)
cargo test

# Run only search crate tests for focused verification
cargo test -p chess-search

# Run specific killer table tests
cargo test -p chess-search killer

# Run specific ordering tests
cargo test -p chess-search ordering
```

## 8. Design Deviations

**Pseudo-legality validation approach:**

- **Parent spec requires**: "Validate killers as pseudo-legal in the current position before promoting them."
- **Why direct validation is unnecessary**: The `order_moves` function operates on the already-generated legal move list from `generate_legal_moves`. Killer scoring is applied by checking each legal move against the killer table via `is_killer`. A legal move that matches a killer gets a boosted score; moves that don't match get no boost. Since we only score moves already in the legal move list, pseudo-legality is guaranteed — we never inject killer moves into the move list, we only boost existing legal moves that happen to match a stored killer. This is the standard implementation used by virtually all chess engines and is functionally equivalent to pre-validating killers, but simpler and more efficient.
- **Alternative chosen**: Check `is_killer` against each move in the legal move list during scoring rather than pre-filtering the killer table against the position. The effect is identical — only valid killers get prioritized — but without needing a separate pseudo-legality check function that doesn't exist in the current codebase.