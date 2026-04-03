All workspace tests passed. The review issues were already fixed in the current code — no additional changes were needed.

**Summary:**
- No files modified — both review issues (extra `quiescence()` argument, `cargo fmt` failures) were already resolved in the working tree
- All verification commands pass: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test -p chess-board` (75 passed), `cargo test -p chess-search` (81 passed), `cargo test` (full workspace including perft integration tests)