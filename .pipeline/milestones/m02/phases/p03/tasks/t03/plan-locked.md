## 1. Deliverables

| # | File | Action | Purpose |
|---|------|--------|---------|
| 1 | `crates/movegen/src/lib.rs` | Modified | Add public `perft` function |
| 2 | `tests/perft.rs` | New | Integration tests validating perft against six standard reference positions at depth 5 |
| 3 | `tools/perft/Cargo.toml` | New | Cargo manifest for the standalone perft runner binary crate |
| 4 | `tools/perft/src/main.rs` | New | Standalone perft runner binary with FEN + depth CLI args and optional divide mode |
| 5 | `Cargo.toml` | Modified | Add `"tools/perft"` to the workspace `members` list |

## 2. Dependencies

- **Crate dependencies**: The perft binary crate (`tools/perft`) depends on `chess-movegen` (for `generate_legal_moves` and `perft`) and `chess-board` (for `Position::from_fen`). These are already workspace members; the binary crate declares them as path dependencies.
- **No new external crates required.** CLI argument parsing uses `std::env::args()` to avoid adding a dependency.
- **Existing sibling tasks completed**: T01 (`generate_castling_moves` in `crates/movegen/src/castling.rs`) and T02 (`generate_legal_moves` in `crates/movegen/src/lib.rs`) are already implemented and available.

## 3. Implementation Details

### 3.1 `crates/movegen/src/lib.rs` — `perft` function

**Purpose**: Public function for recursively counting leaf nodes to validate move generation correctness.

**Signature**:
```rust
pub fn perft(pos: &mut Position, depth: u32) -> u64
```

**Key Logic**:
- Base case: if `depth == 0`, return `1`.
- Generate legal moves via `generate_legal_moves(pos)`.
- For each move: call `pos.make_move(mv)` to get an `UndoInfo`, recurse with `depth - 1`, call `pos.unmake_move(mv, undo)`, accumulate the node count.
- Return the total count.

**Placement**: Defined directly in `lib.rs` alongside `generate_legal_moves`. No re-export needed since it is already at the crate root.

### 3.2 `tests/perft.rs` — Integration test suite

**Purpose**: Validate perft node counts against six known reference positions at depth 5.

**Key Logic**:
- Import `chess_movegen::perft` and `chess_board::Position`.
- Define one `#[test]` function per position (six total).
- Each test parses the FEN via `Position::from_fen`, calls `perft(&mut pos, 5)`, and asserts exact equality with the expected node count.

**Test specifications**:

| Test Function | FEN | Depth | Expected |
|---|---|---|---|
| `test_perft_startpos` | `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1` | 5 | 4,865,609 |
| `test_perft_kiwipete` | `r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -` | 5 | 193,690,690 |
| `test_perft_cpw_pos3` | `8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - -` | 5 | 674,624 |
| `test_perft_cpw_pos4` | `r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq -` | 5 | 15,833,292 |
| `test_perft_cpw_pos5` | `rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ -` | 5 | 89,941,194 |
| `test_perft_cpw_pos6` | `r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - -` | 5 | 164,075,551 |

### 3.3 `tools/perft/` — Standalone perft runner binary crate

**Purpose**: CLI tool for running perft on any FEN at any depth, with optional divide mode for debugging.

#### 3.3.1 `tools/perft/Cargo.toml`

```toml
[package]
name = "perft"
version = "0.1.0"
edition.workspace = true

[dependencies]
chess-movegen = { path = "../../crates/movegen" }
chess-board = { path = "../../crates/board" }
```

#### 3.3.2 `tools/perft/src/main.rs`

**Interface**:
```
Usage: perft [--divide] <fen> <depth>
```

**Key Logic**:
- Parse command-line arguments via `std::env::args()`.
- Detect `--divide` flag if present.
- Collect remaining arguments: all arguments between the optional `--divide` flag and the final argument are joined with spaces to form the FEN string; the final argument is parsed as the depth (`u32`).
- Parse FEN via `Position::from_fen()`. On failure, print usage to stderr and exit with code 1.
- In normal mode: call `chess_movegen::perft(&mut pos, depth)` and print the result in the format `Perft({depth}) = {count}`.
- In divide mode: call `chess_movegen::generate_legal_moves(&mut pos)`, for each move call `pos.make_move(mv)`, run `chess_movegen::perft(&mut pos, depth - 1)`, call `pos.unmake_move(mv, undo)`, print `"{move}: {count}"` for each move (using `Move`'s `Display` impl which outputs UCI-style notation like `e2e4`), then print the total sum in the format `Total: {total}`.

#### 3.3.3 `Cargo.toml` (workspace root)

Add `"tools/perft"` to the workspace members list:

```toml
[workspace]
resolver = "2"
members = ["crates/types", "crates/board", "crates/movegen", "tools/perft"]
```

## 4. API Contracts

### `perft` function

**Input**: `pos: &mut Position` — a valid chess position (mutable for internal make/unmake); `depth: u32` — search depth.

**Output**: `u64` — exact count of leaf nodes at the given depth.

**Contract**: The position is restored to its original state after the call. At `depth == 0`, returns 1. At `depth > 0`, returns the sum of `perft(pos, depth - 1)` after each legal move.

**Example**:
```rust
let mut pos = Position::startpos();
assert_eq!(perft(&mut pos, 0), 1);
assert_eq!(perft(&mut pos, 1), 20);
assert_eq!(perft(&mut pos, 5), 4_865_609);
```

### Perft runner binary CLI

**Input**: `perft [--divide] "<FEN>" <depth>`

**Output (normal mode)**:
```
Perft(5) = 4865609
```

**Output (divide mode)**:
```
e2e3: 599
e2e4: 600
...
Total: 4865609
```

**Error handling**: Invalid arguments or unparseable FEN prints usage to stderr and exits with code 1.

## 5. Test Plan

### Integration Tests (`tests/perft.rs`)

All six tests follow the same pattern: parse FEN, call `perft`, assert exact match.

| Test | Verifies |
|------|----------|
| `test_perft_startpos` | Starting position depth 5 = 4,865,609 |
| `test_perft_kiwipete` | Kiwipete depth 5 = 193,690,690 — heavy castling, en passant, promotions |
| `test_perft_cpw_pos3` | CPW Position 3 depth 5 = 674,624 — endgame with rook, pins |
| `test_perft_cpw_pos4` | CPW Position 4 depth 5 = 15,833,292 — promotions, castling edge cases |
| `test_perft_cpw_pos5` | CPW Position 5 depth 5 = 89,941,194 — pawn promotion, partial castling rights |
| `test_perft_cpw_pos6` | CPW Position 6 depth 5 = 164,075,551 — symmetric, complex middle game |

**Test setup**: Each test creates a `Position` via `Position::from_fen()`. No shared state between tests. Tests use `#[test]` attribute directly (not `#[cfg(test)]` since this is in `tests/`).

**Note**: Kiwipete and CPW positions 5 and 6 at depth 5 involve ~90M–193M nodes and may take several seconds. No timeout annotation needed; Cargo's default test timeout is sufficient.

## 6. Implementation Order

1. **Add `perft` function to `crates/movegen/src/lib.rs`** — implement `pub fn perft(pos: &mut Position, depth: u32) -> u64` using `generate_legal_moves`, `make_move`, and `unmake_move`.

2. **Create `tools/perft/` crate** — create `tools/perft/Cargo.toml` and `tools/perft/src/main.rs` as a separate workspace member crate for the perft runner binary.

3. **Update workspace root `Cargo.toml`** — add `"tools/perft"` to the workspace `members` list.

4. **Create `tests/perft.rs`** — write six integration tests, one per reference position. Run `cargo test --test perft` to validate. If any perft count mismatches, debug using divide mode (step 5) to isolate the faulty move.

5. **Verify perft binary** — run `cargo run --bin perft -- "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1" 5` and confirm output. Test divide mode with `--divide`.

## 7. Verification Commands

```bash
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