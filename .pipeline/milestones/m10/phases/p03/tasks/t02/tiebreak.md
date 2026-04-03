All verification commands pass. Summary of changes:

- **`benches/nnue_bench.rs`** — Fixed multi-line import to single-line format to pass `cargo fmt --check` (the sole issue from review v2)

Files already correct from prior attempts (no changes needed):
- **`crates/nnue/src/accumulator.rs`** — SIMD wiring (`simd::vec_add_i16`/`simd::vec_sub_i16`) in `add_feature`/`remove_feature`, `use crate::simd` import, `accumulator_add_remove_matches_scalar_reference` test
- **`crates/nnue/src/simd.rs`** — `#[allow(dead_code)]` removed from `vec_add_i16`, `vec_sub_i16`, `vec_add_scalar`, `vec_sub_scalar`
- **`Cargo.toml`** (workspace root) — `criterion` dev-dependency + `[[bench]]` target for `nnue_bench`
- **`benches/nnue_bench.rs`** — 4 benchmark functions: `bench_forward_pass`, `bench_accumulator_add_remove`, `bench_evaluate_position`, `bench_push_pop_cycle`

Verification results:
- `cargo test -p chess-nnue` — 29/29 pass
- `cargo test -p chess-nnue --features simd-neon` — 29/29 pass
- `cargo clippy -p chess-nnue -- -D warnings` — clean
- `cargo clippy -p chess-nnue --features simd-neon -- -D warnings` — clean
- `cargo fmt --check` — clean
- `cargo test --workspace --lib` — 489/489 pass (3 ignored)
- `cargo bench --bench nnue_bench --no-run` — compiles clean