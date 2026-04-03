All verification is complete. The full `cargo test` run (background task b6b25ae) also finished with exit code 0. Here's the summary:

## Files modified

- **`crates/uci/src/lib.rs`** — Added `InvalidSetOption` error variant, `SetOption { name, value }` enum variant, `parse_setoption()` function, `"setoption"` dispatch in `parse()`. 4 unit tests added.
- **`crates/uci/src/output.rs`** — Added `option_string()`, `option_combo()`, `info_string()` functions. 3 unit tests added.
- **`crates/engine/src/main.rs`** — Added `book` and `book_mode` fields to `EngineState`. Added `SetOption` handler (loads book file, sets mode). Added book consultation before search in `Go` handler. Emits `BookFile`/`BookMode` option declarations in `Uci` handler.

## Verification results

- `cargo fmt --check` — pass
- `cargo clippy -- -D warnings` — pass
- `cargo test --lib` — 455 tests pass (0 failures)
- `cargo test` (full, including integration) — pass (exit code 0)