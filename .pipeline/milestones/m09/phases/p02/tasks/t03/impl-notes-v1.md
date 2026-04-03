All three verification commands pass (`cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`).

## Files modified

- **`crates/uci/src/output.rs`** — Added `pub fn option_spin(name, default, min, max)` helper for UCI spin option advertisement; added `test_option_spin` unit test
- **`crates/engine/src/syzygy/mod.rs`** — Added `LazySyzygyTablebase` struct with `new()`, `ensure_init()`, and `TbProber` trait implementation (`probe_wdl`, `probe_root`); added 3 unit tests: `lazy_no_path_returns_none`, `lazy_init_deferred`, `lazy_probe_limit_respected`
- **`crates/engine/src/main.rs`** — Added `syzygy_path: String` and `syzygy_probe_depth: u8` fields to `EngineState`; advertised `SyzygyPath` and `SyzygyProbeDepth` UCI options in `Uci` handler; added `setoption` handling for `syzygypath` and `syzygyprobedepth`; wired `LazySyzygyTablebase` construction and passing to `chess_search::search` in the `Go` handler
- **`crates/search/src/lib.rs`** — Formatting only (pre-existing `cargo fmt` violations fixed by `cargo fmt --all`)