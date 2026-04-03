# T03 — Search Parameter Tuning

## Objective

Tune LMR reduction table values, null move reduction depth, futility margins, and aspiration window sizes. Each parameter change must be validated individually via self-play SPRT testing — no bulk untested parameter changes.

## Deliverables

- Tuned LMR reduction table applied to the `search` crate
- Tuned null move reduction depth applied to the `search` crate
- Tuned futility margins applied to the `search` crate
- Tuned aspiration window sizes applied to the `search` crate
- SPRT test results for each individual parameter change confirming non-regression (Elo0=0, Elo1=5, alpha=0.05, beta=0.05)

## Key Implementation Details

- Each search parameter is tuned and tested independently — a change is only applied if it passes SPRT validation
- SPRT bounds: Elo0=0, Elo1=5, alpha=0.05 (Type I error), beta=0.05 (Type II error)
- Uses the self-play framework from Phase P01/T03 to run SPRT tests
- Parameters to tune:
  - **LMR reduction table**: The depth reduction values indexed by depth and move count in late move reductions
  - **Null move reduction depth**: The reduction factor (R) used in null move pruning
  - **Futility margins**: The margin values for futility pruning and reverse futility pruning at different depths
  - **Aspiration window sizes**: The initial aspiration window delta and widening strategy in iterative deepening
- Tuning methodology: For each parameter, test candidate values against the current best via SPRT. Accept changes that pass; reject those that fail.
- Tuned values replace existing constants in the `search` crate source files

## Files to Create or Modify

- `crates/search/src/*.rs` — updated search parameter constants (LMR table, null move reduction depth, futility margins, aspiration window sizes)
- Optionally, a tuning script or tool that automates running SPRT tests for each parameter candidate

## Dependencies

- **T02 (Evaluation Weight Tuning)** should be complete first — search parameter tuning should be done on top of tuned evaluation weights for accurate results
- **Phase P01/T03 (Self-play framework)** must be complete — required for running SPRT validation tests
- **M06 (Advanced Search Techniques)** must be complete — LMR, null move pruning, futility pruning, and aspiration windows must all be implemented

## Verification Criteria

1. Each parameter change (LMR table, null move reduction depth, futility margins, aspiration window sizes) has a corresponding SPRT test result
2. All accepted changes pass SPRT at Elo0=0, Elo1=5, alpha=0.05, beta=0.05
3. Updated parameter values are applied to `crates/search/` source files
4. `cargo build`, `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt --check` all pass with the new parameters
5. No regressions in existing test suites (perft, tactical tests, self-play stability)
6. SPRT test results are documented (which parameters were changed, old vs new values, SPRT outcome)