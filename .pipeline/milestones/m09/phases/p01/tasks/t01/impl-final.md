All three verification commands pass:
- `cargo fmt --check -p engine` — clean
- `cargo clippy -p engine -- -D warnings` — clean
- `cargo test -p engine` — 7/7 tests pass

**Files modified:**
- `crates/engine/src/book/mod.rs` — fixed `cargo fmt` formatting violations (5 diffs: collapsed multi-line `let` binding, `from_be_bytes` array, function signature, `find` closure, and FEN `let` binding onto single lines per rustfmt rules)