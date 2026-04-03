# Format check
cargo fmt --check --manifest-path crates/board/Cargo.toml

# Lint check
cargo clippy -p chess-board -- -D warnings

# Run all tests in the board crate
cargo test -p chess-board

# Run only the unmake_move tests (pattern match)
cargo test -p chess-board unmake
```

## 8. Design Deviations

None.