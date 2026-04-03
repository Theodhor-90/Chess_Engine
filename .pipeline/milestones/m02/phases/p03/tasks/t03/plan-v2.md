# Compile the entire workspace (ensures no compile errors)
cargo build --workspace

# Run all unit tests (movegen, board, types crates)
cargo test --workspace

# Run just the perft integration tests
cargo test --test perft

# Run the perft binary in normal mode
cargo run --bin perft -- "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1" 5

# Run the perft binary in divide mode
cargo run --bin perft -- --divide "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1" 5

# Lint checks
cargo fmt --check
cargo clippy --workspace -- -D warnings
```

## 8. Design Deviations

**Deviation 1: Perft binary as a separate workspace member crate**

- **Parent spec states**: "Create a standalone perft runner binary in `tools/` (e.g., `tools/perft/main.rs` or `tools/perft.rs`)."
- **Issue**: The workspace root `Cargo.toml` is a pure virtual workspace manifest containing only `[workspace]` and `[workspace.package]` — it has no `[package]` section. A `[[bin]]` target requires a `[package]` section in the same `Cargo.toml`. Adding a `[package]` to the workspace root would change it from a virtual workspace to a hybrid workspace-and-package, which is unconventional and couples the binary to the workspace root.
- **Chosen approach**: Create `tools/perft/` as a separate workspace member crate with its own `Cargo.toml` (containing `[package]` and `[dependencies]`) and `src/main.rs`. Add `"tools/perft"` to the workspace `members` list. This is the idiomatic Cargo approach for tool binaries in a workspace, keeps the workspace root clean, and matches the parent spec's suggested `tools/perft/main.rs` path pattern. The binary is built via `cargo run --bin perft` or `cargo build --bin perft`.

**Deviation 2: FEN string parsing in the perft binary**

- **Parent spec states**: "Accepts a FEN string and depth as command-line arguments."
- **Issue**: A FEN string contains spaces (up to 6 space-separated fields), which the shell splits into multiple arguments. The task spec does not specify how to handle this.
- **Chosen approach**: The binary will accept the FEN as a single quoted argument (standard CLI convention). Internally, it will collect all arguments between `--divide`/start and the final numeric depth argument, joining them with spaces to reconstruct the FEN. This avoids requiring users to remember to quote the FEN while also working correctly when they do quote it.