# Format check
cargo fmt --check --manifest-path crates/board/Cargo.toml

# Lint check
cargo clippy -p chess-board -- -D warnings

# Run all tests in the board crate
cargo test -p chess-board

# Run all workspace tests to ensure nothing is broken
cargo test --workspace
```

## 8. Design Deviations

**Deviation 1: `pub(crate)` constructor instead of `pub` fields**

- **Parent spec states**: `crates/board/src/position.rs` — "may need to add the `from_fen` and `to_fen` methods to `Position`"
- **Issue**: The `Position` struct fields are private (no `pub` modifier), and `fen.rs` is a separate module that needs to construct `Position` instances. Making all fields `pub` would break encapsulation. Adding `pub(crate)` to each field would be invasive.
- **Alternative chosen**: Add a `pub(crate) fn new(...)` constructor to `Position` that accepts all fields. This allows `fen.rs` to construct positions while keeping fields private to external consumers. This is the minimal change and follows Rust encapsulation conventions.