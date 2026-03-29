Now I have a complete picture of the codebase. Let me produce the implementation plan.

## 1. Deliverables

- **Modify**: `crates/eval/src/lib.rs` — Add a diagnostic helper function (used only in tests) to decompose evaluation into individual term contributions (pawn structure, king safety, mobility, piece bonuses, king centralization) for the weight balance verification. Adjust weight constants in imported modules as needed to satisfy the 50% dominance constraint and king safety < material constraint. Add integration tests for tapered interpolation across game phases and weight balance verification.

## 2. Dependencies

No new external dependencies required. All existing crate dependencies (`chess-types`, `chess-board`, `chess-movegen`) are sufficient.

## 3. Implementation Details

### `crates/eval/src/lib.rs`

**Purpose**: This is the only file that needs modification. T03 is a verification and balancing pass — it confirms that all M07 evaluation terms integrate correctly into the tapered evaluation framework, then adds tests to verify the constraints, and adjusts weight constants if any constraints are violated.

**Key Logic**:

1. **Diagnostic decomposition function** (test-only, `#[cfg(test)]`): A function `eval_term_contributions(pos: &Position, pawn_table: &mut PawnHashTable) -> TermBreakdown` that computes the final tapered contribution of each non-material evaluation term individually. This is necessary to verify the 50% dominance constraint and king safety < material constraint across the 6 reference positions.

   The `TermBreakdown` struct (defined inside `#[cfg(test)] mod tests`):
   ```rust
   struct TermBreakdown {
       material: i32,        // tapered material + PST score
       pawn_structure: i32,  // tapered pawn hash + passed pawn extras
       king_safety: i32,     // tapered king safety
       mobility: i32,        // tapered mobility
       piece_bonuses: i32,   // tapered piece bonuses
       king_centralization: i32, // tapered king centralization
   }
   ```

   The function computes each term's MG and EG components independently, retrieves the phase via `compute_phase(pos)`, then applies the tapered interpolation `((mg * phase) + (eg * (MAX_PHASE - phase))) / MAX_PHASE` to each term separately to produce its final tapered contribution.

2. **Weight balance verification tests**: For each of the 6 reference positions (startpos + 5 quiet FENs from the task spec), compute the breakdown and verify:
   - No single non-material term's absolute value exceeds 50% of the sum of all non-material terms' absolute values
   - The king safety term's absolute value is less than the material term's absolute value

3. **Phase interpolation tests**: At least 3 tests using representative FEN positions for opening (high phase, near `MAX_PHASE`), middlegame (intermediate phase), and endgame (low phase, near 0) material configurations, verifying that `compute_phase()` returns appropriate values and that the tapered evaluation produces scores consistent with the phase region (e.g., MG-dominated terms weigh more in opening positions, EG-dominated terms weigh more in endgame positions).

4. **Weight adjustments**: If any of the 6 reference positions violate the constraints, adjust the relevant weight constants. Based on the current code analysis:
   - King safety MG weights (`PAWN_SHIELD_BONUS_MG`, `ATTACK_SCORE_SCALE_MG`) and mobility weights are moderate
   - Piece bonus weights (bishop pair 30/50, rook open file 20/10, etc.) are moderate
   - Pawn structure penalties are moderate
   
   The implementer must evaluate all 6 positions, check the constraints, and only adjust weights that cause violations. Adjustments should be conservative — scale down the offending term rather than restructuring.

**Exports**: No new public exports. The diagnostic function and struct are test-only.

**Interfaces/Contracts**: The `evaluate()` function's public signature remains unchanged. All modifications are internal (test code and potentially weight constant adjustments in sibling modules).

## 4. API Contracts

N/A — This task modifies no public API. The `evaluate()` function signature is unchanged. All new code is test-only.

## 5. Test Plan

All tests are added to `crates/eval/src/lib.rs` in the existing `#[cfg(test)] mod tests` block.

### Test 1: `weight_balance_no_term_exceeds_50_percent`
- **Setup**: Create positions from the 6 reference FENs (startpos + 5 quiet middlegame positions from the task spec)
- **Logic**: For each position, compute `TermBreakdown` via the diagnostic function. Sum the absolute values of all non-material terms. Assert that no single non-material term's absolute value exceeds 50% of that sum.
- **Assertion**: `term.abs() * 2 <= total_non_material_abs` for each non-material term in each position

### Test 2: `king_safety_less_than_material_in_quiet_positions`
- **Setup**: Same 6 reference FEN positions
- **Logic**: For each position, compute `TermBreakdown`. Assert that `king_safety.abs() < material.abs()`.
- **Assertion**: `breakdown.king_safety.abs() < breakdown.material.abs()` for each position

### Test 3: `phase_interpolation_opening`
- **Setup**: A position with full material (or near-full): `rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1`
- **Logic**: Verify `compute_phase()` returns a value close to `MAX_PHASE` (>= 20). Verify the evaluation score is influenced predominantly by MG terms.
- **Assertion**: `phase >= 20`

### Test 4: `phase_interpolation_middlegame`
- **Setup**: A position with reduced material (e.g., each side has 1 rook, 1 bishop, pawns): `r1b1k3/pppp1ppp/8/8/8/8/PPPP1PPP/R1B1K3 w - - 0 1`
- **Logic**: Verify `compute_phase()` returns an intermediate value (between 2 and 20 exclusive).
- **Assertion**: `phase > 2 && phase < 20`

### Test 5: `phase_interpolation_endgame`
- **Setup**: A pure pawn endgame or near-endgame: `4k3/pppp4/8/8/8/8/PPPP4/4K3 w - - 0 1`
- **Logic**: Verify `compute_phase()` returns 0 (or near 0). Verify the score is dominated by EG components.
- **Assertion**: `phase == 0`

### Test 6: `all_m07_terms_contribute_mg_and_eg`
- **Setup**: A position where multiple eval terms are active — e.g., `r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4` (Italian Game mainline from the reference set)
- **Logic**: Compute the TermBreakdown. Verify that pawn_structure, king_safety, mobility, and piece_bonuses each have non-zero MG or EG components (before tapering), confirming they contribute to both phases.
- **Assertion**: For each non-material term, verify the raw MG or EG component (or both) is non-zero. At minimum, the term's tapered value should be computable (i.e., the term participates in the interpolation).

## 6. Implementation Order

1. **Evaluate reference positions manually**: Run the existing `evaluate()` on all 6 reference positions and inspect the individual term contributions by temporarily computing them. Determine if any weight adjustments are needed.

2. **Adjust weights (if needed)**: Modify weight constants in the relevant module files (`king_safety.rs`, `mobility.rs`, `pieces.rs`, `pawn.rs`, `king_centralization.rs`) to satisfy the 50% dominance constraint and king safety < material constraint. Only change values that violate the constraints.

3. **Add `TermBreakdown` struct and diagnostic function**: Add the test-only `TermBreakdown` struct and `eval_term_contributions()` function inside `#[cfg(test)] mod tests` in `lib.rs`.

4. **Add weight balance tests**: Add `weight_balance_no_term_exceeds_50_percent` and `king_safety_less_than_material_in_quiet_positions` tests.

5. **Add phase interpolation tests**: Add `phase_interpolation_opening`, `phase_interpolation_middlegame`, and `phase_interpolation_endgame` tests.

6. **Add MG/EG contribution test**: Add `all_m07_terms_contribute_mg_and_eg` test.

7. **Run all tests**: Verify all existing and new tests pass. Run `cargo clippy` and `cargo fmt`.

## 7. Verification Commands

```bash
# Run all eval crate tests
cargo test -p chess-eval

# Run only the new integration tests by name
cargo test -p chess-eval weight_balance_no_term_exceeds_50_percent
cargo test -p chess-eval king_safety_less_than_material_in_quiet_positions
cargo test -p chess-eval phase_interpolation_opening
cargo test -p chess-eval phase_interpolation_middlegame
cargo test -p chess-eval phase_interpolation_endgame
cargo test -p chess-eval all_m07_terms_contribute_mg_and_eg

# Run all workspace tests to confirm no regressions
cargo test --workspace

# Linting
cargo fmt --check
cargo clippy -- -D warnings
```

## 8. Design Deviations

**Deviation 1: Diagnostic function scope**

- **Parent spec requires**: "Verification that all evaluation terms from P01 and P02 contribute separate MG and EG scores combined via `compute_phase()` interpolation" — implying inspection of the pipeline.
- **Why direct inspection is impractical**: The `evaluate()` function computes a single scalar; there is no existing mechanism to extract per-term contributions. Adding a public decomposition API would be over-engineering for a verification task.
- **Alternative chosen**: A `#[cfg(test)]`-only diagnostic function inside the test module that re-computes each term independently and applies tapered interpolation per-term. This provides the necessary observability for test assertions without polluting the public API. The diagnostic function duplicates some logic from `evaluate()` but this is acceptable because it is test-only code whose sole purpose is verifying the integration.

**Deviation 2: Weight adjustment file scope**

- **Parent spec states**: "Modify: `crates/eval/src/lib.rs` — weight adjustments across all M07 evaluation terms" and "Potentially modify weight constants in any eval module files from P01 and P02."
- **Why this is ambiguous**: Weight constants live in their respective module files (`king_safety.rs`, `mobility.rs`, `pieces.rs`, `pawn.rs`, `king_centralization.rs`), not in `lib.rs`. The `lib.rs` file only orchestrates calls to these modules.
- **Alternative chosen**: If weight adjustments are needed to satisfy the constraints, the constants will be modified in the module files where they are defined (e.g., `ATTACK_SCORE_SCALE_MG` in `king_safety.rs`), since that is where they live. No weight constants exist in `lib.rs` to adjust. This is a clarification, not a true deviation — the parent spec explicitly allows modifying "any eval module files from P01 and P02."