## Files created/modified

- **Created** `crates/nnue/Cargo.toml` — crate manifest with dependencies on `chess-types` and `chess-board`
- **Created** `crates/nnue/src/arch.rs` — 11 network topology constants (`HALFKP_FEATURES`, `L1_SIZE`, `L2_SIZE`, `OUTPUT_SIZE`, quantization shifts, `CLIPPED_RELU_MAX`) with quantization scheme documentation
- **Created** `crates/nnue/src/feature.rs` — `feature_index()` function and `HalfKpFeature` struct for HalfKP feature index computation; 3 tests (`feature_index_in_range`, `feature_index_unique`, `feature_index_varies_with_king_square`)
- **Created** `crates/nnue/src/network.rs` — `Accumulator` struct with `new()`, `init_from_bias()`, `add_feature()`, `remove_feature()`; `Network` struct with `new_zeroed()`; 3 tests (`accumulator_init_from_bias`, `accumulator_add_remove_roundtrip`, `network_new_zeroed`)
- **Created** `crates/nnue/src/lib.rs` — module declarations and public re-exports
- **Modified** `Cargo.toml` (root) — added `"crates/nnue"` to workspace members after `"crates/movegen"`