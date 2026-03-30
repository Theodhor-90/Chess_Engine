I now have all the context I need. Here is the complete task plan:

## 1. Deliverables

- **`tests/data/perft_suite.txt`** — Data file containing 100+ perft positions in the format `FEN;depth;expected_nodes`, one per line. Positions sourced from published, externally-validated suites: CPW (Chess Programming Wiki) perft results, Kiwipete, Ethereal perft collection, and other established collections. Covers standard positions plus edge cases (castling rights, en passant, promotions, discovered checks, pins).
- **`tests/perft_suite.rs`** — Integration test that loads positions from the data file, runs `perft()` on each, and asserts the computed node count matches the reference value. Reports pass/fail per position with descriptive failure messages including the FEN, depth, expected count, and actual count.
- **`crates/movegen/Cargo.toml`** (modify) — Add a `[[test]]` entry to register `perft_suite` as an integration test target, following the existing pattern used for `perft`.

## 2. Dependencies

- `chess-movegen` crate (provides `perft` function) — already exists
- `chess-board` crate (provides `Position::from_fen`) — already exists
- No new external crates required. The data file is read at compile time using `include_str!()` to avoid runtime file path issues.

## 3. Implementation Details

### `tests/data/perft_suite.txt`

- Plain text file, one position per line
- Format: `<FEN>;<depth>;<expected_node_count>`
- Lines starting with `#` are comments (for attributing sources)
- Contains 100+ positions sourced from:
  - **CPW standard suite** (6 positions — startpos, Kiwipete, positions 3–6): depths 1–5
  - **Ethereal perft collection**: ~30 positions at depths 1–5 covering complex middlegame positions
  - **Marcel van Kervinck's rocechess collection**: ~20 positions with promotions, en passant, castling edge cases
  - **Additional published CPW positions**: positions exercising discovered checks, double checks, pins, en passant captures that expose king, castling through/into/out-of check prevention
- Depths chosen to keep individual position runtimes reasonable (depths 1–5 for complex positions, up to depth 6 for simpler positions) so the full suite completes in a reasonable time under `cargo test`
- The existing 6 positions from `tests/perft.rs` are included in the suite (same FEN, same depth, same expected counts) to maintain coverage continuity

### `tests/perft_suite.rs`

- **Imports**: `use chess_board::Position;` and `use chess_movegen::perft;`
- **Data loading**: Uses `include_str!("data/perft_suite.txt")` to embed the data file at compile time. This avoids runtime working-directory issues and is the idiomatic Rust pattern for test data.
- **Parsing**: A helper function `parse_perft_entries()` that:
  - Splits the included string by newlines
  - Skips empty lines and lines starting with `#`
  - Splits each line by `;` into `(fen, depth, expected_nodes)`
  - Returns a `Vec<(&str, u32, u64)>`
- **Test function**: A single `#[test] fn test_perft_suite()` that:
  - Calls `parse_perft_entries()` to get all entries
  - Asserts the entry count is at least 100 (meta-check ensuring the suite hasn't been accidentally truncated)
  - Iterates over all entries
  - For each: creates `Position::from_fen(fen).expect("valid fen")`, calls `perft(&mut pos, depth)`, compares to expected
  - On failure, panics with a message including the position index, FEN, depth, expected count, and actual count
  - Tracks and reports total pass/fail counts
- **Pattern note**: Uses a single test function iterating over all positions rather than 100+ individual `#[test]` functions. This matches the data-driven nature of the suite and avoids massive test output noise. The failure message includes full context for debugging any individual failure.

### `crates/movegen/Cargo.toml` (modification)

- Add a second `[[test]]` entry below the existing one:
  ```toml
  [[test]]
  name = "perft_suite"
  path = "../../tests/perft_suite.rs"
  ```
- This registers `tests/perft_suite.rs` as an integration test under the `chess-movegen` crate, consistent with how `tests/perft.rs` is already registered.

## 4. API Contracts

N/A — this task creates test infrastructure only, with no new public API surface.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| `test_perft_suite` runs successfully | All 100+ perft positions produce node counts matching published reference values, confirming movegen correctness |
| Entry count ≥ 100 assertion | The data file contains at least 100 positions (meta-check against accidental truncation) |
| Edge case coverage in data file | Positions covering castling rights, en passant, promotions, discovered checks, and pins are included (verified by inspection of the data file) |

The existing `tests/perft.rs` with its 6 individual tests remains untouched and continues to run as before.

## 6. Implementation Order

1. **`tests/data/perft_suite.txt`** — Assemble the 100+ perft positions with verified reference node counts from published sources. Organize with source-attribution comments.
2. **`tests/perft_suite.rs`** — Implement the integration test: data parsing, iteration, perft execution, and assertion with descriptive failure messages.
3. **`crates/movegen/Cargo.toml`** — Add the `[[test]]` entry for `perft_suite`.
4. **Verification** — Run `cargo test --test perft_suite` and confirm all positions pass.

## 7. Verification Commands

```bash
# Run the new extended perft suite
cargo test --test perft_suite

# Run the existing perft tests to confirm they still pass
cargo test --test perft

# Run all tests to ensure nothing is broken
cargo test

# Verify formatting and lints
cargo fmt --check
cargo clippy -- -D warnings
```

## 8. Design Deviations

**Data file format choice**: The task spec says "A data file (or embedded constants)" — this plan uses a data file (`tests/data/perft_suite.txt`) loaded via `include_str!()` rather than a Rust source file with 100+ hardcoded constants. Rationale:

- The spec explicitly permits either approach ("or embedded constants")
- A separate data file is cleaner for 100+ entries — easier to audit, diff, and extend
- `include_str!()` embeds the data at compile time, so there is no runtime file-path dependency
- This is a standard Rust pattern for test data and introduces no new dependencies

This is not a deviation from the spec but a choice within its stated options. No other deviations are needed.