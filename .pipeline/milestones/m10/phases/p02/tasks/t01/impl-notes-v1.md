The background test run already completed successfully — all 507 tests passed with exit code 0. The implementation is complete. Here's the final summary:

- **Created** `tools/datagen/Cargo.toml` — binary crate manifest
- **Created** `tools/datagen/src/main.rs` — self-play data generation binary
- **Modified** `Cargo.toml` (workspace root) — added `"tools/datagen"` to workspace members

All verification commands pass: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo build`, `cargo run` (produced 25 training records from 2 games), and `cargo test` (507 tests, 0 failures).