# Format check
cargo fmt -p chess-uci --check

# Lint check
cargo clippy -p chess-uci -- -D warnings

# Run all uci crate tests (includes both T01 parser tests and new T02 output tests)
cargo test -p chess-uci

# Verify the full workspace still compiles
cargo check --workspace
```

## 8. Design Deviations

None.