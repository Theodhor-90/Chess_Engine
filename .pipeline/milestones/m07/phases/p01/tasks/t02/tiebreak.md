# Run all eval crate tests (including existing T01 tests and new T02 tests)
cargo test -p chess-eval

# Run clippy with warnings-as-errors for the eval crate
cargo clippy -p chess-eval -- -D warnings

# Run rustfmt check
cargo fmt --check -p chess-eval

# Run full workspace tests to ensure no regressions
cargo test --workspace
```

## 8. Design Deviations

None.