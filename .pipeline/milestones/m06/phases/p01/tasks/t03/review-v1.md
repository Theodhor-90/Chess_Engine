The review is complete. The structured output has already been provided with the verdict and issues. To summarize:

**Verdict: needs_revision**

Two issues found:

1. **Major** (`crates/search/src/lib.rs`): `cargo fmt --check` fails — the reverse futility pruning `if`-condition uses multi-line formatting that `rustfmt` wants collapsed to a single line. The spec's exit criterion 8 requires `cargo fmt --check` to pass.

2. **Critical** (`crates/search/src/lib.rs`): All 7 futility-specific tests from the locked plan are missing: `futility_margins_match_spec`, `reverse_futility_prunes_node`, `futility_skips_quiet_moves`, `futility_does_not_prune_in_check`, `futility_does_not_prune_captures`, `futility_preserves_wac_solve_rate`, and `futility_interacts_correctly_with_nmp_and_lmr`. The implementation logic itself is correct, but the plan explicitly requires these tests, and the spec's exit criterion 4 (WAC solve-rate validation) depends on them.