Now I have complete context. Let me produce the implementation plan.

## 1. Deliverables

| # | File | Action | Purpose |
|---|------|--------|---------|
| 1 | `crates/movegen/src/lib.rs` | Modified | Add public `perft` function and re-export it |
| 2 | `tests/perft.rs` | New | Integration tests validating perft against six standard reference positions at depth 5 |
| 3 | `tools/perft.rs` | New | Standalone perft runner binary with FEN + depth CLI args and optional divide mode |
| 4 | `Cargo.toml` | Modified | Add `tools/perft.rs` binary target to workspace root and ensure it can depend on `chess-movegen` and `chess-board` |

## 2. Dependencies

- **Crate dependencies**: The perft binary (`tools/perft.rs`) needs `chess-movegen` (for `generate_legal_moves` and `perft`) and `chess-board` (for `Position::from_fen`). These are already workspace members; the binary just needs to declare them as dependencies.
- **No new external crates required.** CLI argument parsing will use `std::env::args()` to avoid adding a dependency.
- **Existing sibling tasks completed**: T01 (`generate_castling_moves`) and T02 (`generate_legal_moves`) are already implemented and available in `crates/movegen/src/lib.rs`.

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
- For each move: call `pos.make_move(mv)`, recurse with `depth - 1`, call `pos.unmake_move(mv, undo)`, accumulate the node count.
- Return the total count.

**Exports**: Add `pub use` or just make the function `pub` at the module top level (it's defined directly in `lib.rs`, so no re-export needed).

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

### 3.3 `tools/perft.rs` — Standalone perft runner binary

**Purpose**: CLI tool for running perft on any FEN at any depth, with optional divide mode for debugging.

**Interface**:
```
Usage: perft [--divide] <fen> <depth>
```

**Key Logic**:
- Parse command-line arguments via `std::env::args()`.
- Detect `--divide` flag.
- Parse FEN string and depth.
- In normal mode: call `perft(pos, depth)` and print the result.
- In divide mode: generate legal moves via `generate_legal_moves`, for each move call `make_move`, run `perft` at `depth - 1`, call `unmake_move`, print `"{move}: {count}"` for each move (using `Move`'s `Display` impl which outputs UCI-style like `e2e4`), then print the total sum.
- Exit with a non-zero code if FEN parsing fails or arguments are invalid, printing usage to stderr.

### 3.4 `Cargo.toml` — Binary target

**Purpose**: Register the perft tool as a binary in the workspace.

**Changes**: Add a `[[bin]]` section for `tools/perft.rs`:
```toml
[[bin]]
name = "perft"
path = "tools/perft.rs"
```

Also add `[dependencies]` for the root `Cargo.toml` binary:
```toml
[dependencies]
chess-movegen = { path = "crates/movegen" }
chess-board = { path = "crates/board" }
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

2. **Update `Cargo.toml`** — add `[[bin]]` target for `tools/perft.rs` and the necessary dependencies at the workspace root.

3. **Create `tests/perft.rs`** — write six integration tests, one per reference position. Run `cargo test --test perft` to validate. If any perft count mismatches, debug using divide mode (step 4) to isolate the faulty move.

4. **Create `tools/perft.rs`** — implement the CLI binary with argument parsing, normal mode, and divide mode. Verify with `cargo run --bin perft -- "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1" 5`.

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

**Deviation 1: FEN string parsing in the perft binary**

- **Parent spec states**: "Accepts a FEN string and depth as command-line arguments."
- **Issue**: A FEN string contains spaces (up to 6 space-separated fields), which the shell splits into multiple arguments. The task spec does not specify how to handle this.
- **Chosen approach**: The binary will accept the FEN as a single quoted argument (standard CLI convention). Internally, it will collect all arguments between `--divide`/start and the final numeric depth argument, joining them with spaces to reconstruct the FEN. This avoids requiring users to remember to quote the FEN while also working correctly when they do quote it.