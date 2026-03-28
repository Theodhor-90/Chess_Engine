Now I have all the context. Let me produce the definitive plan.

## 1. Deliverables

- **`crates/search/src/lib.rs`** — modified to add:
  - `SINGULAR_MARGIN` constant (64 centipawns)
  - `SINGULAR_MIN_DEPTH` constant (6)
  - `singular_extension_enabled` flag in `SearchContext` (following existing pattern of `lmr_enabled`, `futility_enabled`, `check_extension_enabled`)
  - `excluded_move` parameter added to `negamax` function signature to support excluding the TT move during the verification search
  - Singular extension logic within `negamax`: after TT probing and before move generation, when conditions are met (depth ≥ 6, TT move exists with sufficient depth, TT entry has a lower-bound or exact score), perform a verification search excluding the TT move at reduced depth (`depth / 2`) with zero-window `(tt_score - margin - 1, tt_score - margin)`. If all alternatives score below the threshold, extend the TT move's search depth by one ply.
  - Unit tests for activation, non-activation, and depth threshold conditions

## 2. Dependencies

No new external dependencies required. The implementation uses only existing crate-internal types and functions:
- `TtEntry` (from `tt` module) — `depth()`, `score()`, `bound()`, `best_move()`
- `score_from_tt` — to convert TT score to search score
- `BoundType` — to check if the TT entry has a lower-bound or exact bound
- `negamax` — recursive call for the verification search
- `chess_movegen::generate_legal_moves` — already used; the excluded move is simply filtered from the resulting move list

## 3. Implementation Details

### Constant: `SINGULAR_MARGIN`
- Value: `64` (centipawns)
- Placed alongside existing constants `FUTILITY_MARGINS`, `REVERSE_FUTILITY_MARGINS`, and `IID_MIN_DEPTH` near the top of `lib.rs`

### Constant: `SINGULAR_MIN_DEPTH`
- Value: `6`
- Minimum remaining depth at which singular extension is attempted

### `SearchContext` modification
- Add field `pub(crate) singular_extension_enabled: bool` following the pattern of the existing `check_extension_enabled` field
- Set to `true` in the `search()` function's `SearchContext` initialization
- Set to `true` in the `test_ctx()` helper in the test module
- All existing test `SearchContext` initializations must include this field (set to `true`)

### `negamax` signature change
- Add parameter `excluded_move: Option<Move>` after `prev_move`
- All existing call sites pass `None` for `excluded_move` (normal search behavior)
- Only the singular extension verification search passes `Some(tt_move)` to exclude the TT move

### Singular extension logic placement in `negamax`
The singular extension logic is inserted **after** TT probing and the TT move validation, but **before** move generation and IID. The logic:

1. **Guard conditions**: `singular_extension_enabled && depth >= SINGULAR_MIN_DEPTH && excluded_move.is_none() && !in_check`
2. **TT entry check**: A TT entry must exist with `bound` of `LowerBound` or `Exact`, the TT entry's `depth()` must be `>= depth - 3` (sufficient depth to trust the score), and `tt_move` must be `Some`
3. **Verification search**: Call `negamax` with:
   - `depth: depth / 2` (reduced depth)
   - `alpha: tt_score - SINGULAR_MARGIN - 1`
   - `beta: tt_score - SINGULAR_MARGIN`
   - `excluded_move: Some(tt_move)` (exclude the TT move)
   - `allow_null: false` (to avoid interfering with null move pruning in the verification)
4. **Extension decision**: If the verification search returns a score `< tt_score - SINGULAR_MARGIN` (i.e., all alternatives are significantly worse), the TT move's depth is extended by one ply: `depth = depth.saturating_add(1)`

### Move exclusion in `negamax`
When `excluded_move` is `Some(mv)`, after generating legal moves, filter out the excluded move from the move list using `.retain(|&m| Some(m) != excluded_move)`. This is applied right after `generate_legal_moves` and before move ordering. The TT move validation section also skips setting `tt_move` if it matches the excluded move.

### Variable tracking for per-move extension
The singular extension applies to the TT move specifically. The `depth` variable is incremented by 1 when singular extension triggers, so the entire node (including the TT move) searches one ply deeper. This is correct because the TT move will be searched first (highest ordering priority), and the extension depth applies to the full node.

## 4. API Contracts

**Modified function signature:**
```rust
pub fn negamax(
    pos: &mut Position,
    mut depth: u8,
    mut alpha: i32,
    beta: i32,
    ply: u8,
    allow_null: bool,
    ctx: &mut SearchContext,
    prev_move: Option<(PieceKind, Move)>,
    excluded_move: Option<Move>,  // NEW
) -> (i32, Option<Move>)
```

- `excluded_move: None` — normal search, no move exclusion
- `excluded_move: Some(mv)` — verification search for singular extension; the specified move is removed from the move list before searching

All call sites (recursive calls within `negamax`, the `search()` function, IID call, null move search, and all test calls) pass `None` for `excluded_move` except the singular extension verification search.

## 5. Test Plan

### Test 1: `singular_extension_triggers_when_tt_move_dominates`
- **Setup**: Use a position where one move clearly dominates (e.g., a position with a forced tactic like `"2rr3k/pp3pp1/1nnqbN1p/3pN3/2pP4/2P3Q1/PPB4P/R4RK1 w - - 0 1"`). Pre-warm the TT by running iterative deepening up to depth 6+ so the TT contains an entry with sufficient depth and a best move. Then run negamax at depth 8 with `singular_extension_enabled: true` and compare nodes against the same search with `singular_extension_enabled: false`.
- **Assertion**: With singular extension enabled, the node count differs (more nodes due to deeper search on the extended move), confirming the extension triggered.

### Test 2: `singular_extension_does_not_trigger_when_alternatives_comparable`
- **Setup**: Use a quiet position (e.g., starting position) where no single first move dominates by the singular margin. Run iterative deepening to depth 8. Compare nodes with `singular_extension_enabled: true` vs `false`.
- **Assertion**: Node counts should be very similar (or identical at lower depths), because no TT move dominates by the margin — the verification search finds alternatives scoring within range, so the extension does not trigger.

### Test 3: `singular_extension_not_attempted_below_depth_6`
- **Setup**: Run negamax at depth 5 on any position with `singular_extension_enabled: true` and `singular_extension_enabled: false`.
- **Assertion**: Node counts are identical, confirming the depth threshold prevents activation.

### Test 4: `singular_and_check_extensions_improve_wac_solve_rate`
- **Setup**: Run the WAC tactical test suite positions at fixed depth (6) with both check and singular extensions enabled vs both disabled. Use the same WAC positions and pattern as the existing `check_extension_improves_wac_solve_rate` test.
- **Assertion**: `correct_on >= correct_off` — extensions should not reduce (and ideally improve) the solve rate.

## 6. Implementation Order

1. **Add constants**: Add `SINGULAR_MARGIN` and `SINGULAR_MIN_DEPTH` constants near the top of `lib.rs`, alongside existing constants `MATE_SCORE`, `INFINITY`, `IID_MIN_DEPTH`, `MAX_PLY`, `FUTILITY_MARGINS`, and `REVERSE_FUTILITY_MARGINS`.

2. **Add `singular_extension_enabled` to `SearchContext`**: Add the new field, update `search()` initialization, and update `test_ctx()`.

3. **Add `excluded_move` parameter to `negamax`**: Modify the function signature to add `excluded_move: Option<Move>`. Update all call sites (recursive calls in negamax, IID call, null move call, `search()` call, and all test calls) to pass `None`.

4. **Implement move exclusion**: After `generate_legal_moves`, if `excluded_move` is `Some`, filter it out of the move list. Also ensure the TT move validation discards the TT move if it matches the excluded move.

5. **Implement singular extension logic**: After TT probing and TT move validation, add the singular extension check: guard conditions, verification search call with excluded move and reduced depth/window, and conditional depth increment.

6. **Add unit tests**: Add the four tests described in the test plan, following the existing test patterns (using `SearchContext` struct initialization with all flags, comparing node counts or solve rates).

7. **Run verification**: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`.

## 7. Verification Commands

```bash
cd /Users/theodhor/Desktop/Portfolio/Chess_Engine && cargo fmt --check
cd /Users/theodhor/Desktop/Portfolio/Chess_Engine && cargo clippy -- -D warnings
cd /Users/theodhor/Desktop/Portfolio/Chess_Engine && cargo test -p chess-search
cd /Users/theodhor/Desktop/Portfolio/Chess_Engine && cargo test
```

## 8. Design Deviations

**Deviation 1: `excluded_move` parameter on `negamax` instead of filtering within the function body only**

- **Parent spec requires**: "perform a reduced-depth verification search excluding the TT move from the move list"
- **Why an `excluded_move` parameter is needed**: The verification search is a recursive call to `negamax` at a reduced depth. To exclude the TT move from that search, the information about which move to exclude must flow into the recursive call. Simply filtering the move list at the call site is insufficient because the exclusion must apply at the root of the verification search's move generation. Adding an `excluded_move: Option<Move>` parameter to `negamax` is the standard approach used by all major chess engines (Stockfish, Ethereal) and is the only clean way to propagate this constraint.
- **Alternative chosen**: Add `excluded_move: Option<Move>` as the final parameter of `negamax`. This adds one `Option<Move>` (4 bytes) to the signature. All existing call sites pass `None`, preserving existing behavior. Only the singular extension verification search passes `Some(tt_move)`.