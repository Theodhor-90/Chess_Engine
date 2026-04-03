Good, I have all the context I need. Now let me produce the revised plan.

## 1. Deliverables

- **`tests/data/wac.epd`** — EPD file containing the Win At Chess tactical puzzle suite. Each line has a FEN (first 4 fields), followed by `bm` (best move) operations in standard algebraic notation (SAN).
- **`tests/data/ecm.epd`** — EPD file containing the Encyclopedia of Chess Middlegames tactical puzzle suite in EPD format.
- **`tests/data/sts.epd`** — EPD file containing the Strategic Test Suite in EPD format.
- **`tools/tactical/Cargo.toml`** — Cargo package manifest for the tactical test runner binary, declaring it as a workspace member with dependencies on `chess-types`, `chess-board`, and `chess-movegen` for SAN-to-UCI conversion.
- **`tools/tactical/src/main.rs`** — Standalone binary that loads EPD files, spawns the engine as a child process, communicates via UCI (stdin/stdout), sends `position fen <fen>` + `go depth N` or `go movetime T`, parses `bestmove` responses, compares against the expected best move(s) using SAN-to-UCI resolution via the engine's own crates, and reports solve rates per suite.
- **`Cargo.toml`** (modify) — Add `tools/tactical` to the workspace `members` list.

## 2. Dependencies

- The engine binary (`crates/engine`) must be built before running the tactical tool, since it spawns `engine` as a child process.
- No new external crate dependencies. The tactical runner depends on workspace crates (`chess-types`, `chess-board`, `chess-movegen`) for SAN-to-UCI conversion, and uses only `std` for everything else (process spawning via `std::process::Command`, I/O via `std::io`, time via `std::time`).
- EPD source files for WAC, STS, and ECM must be sourced from publicly available collections (e.g., the widely distributed WAC.epd with 300 positions, STS suites, ECM positions).

## 3. Implementation Details

### `tests/data/wac.epd`

- Standard EPD format: `<piece_placement> <side> <castling> <ep> bm <move>;` with optional `id` tags.
- WAC contains 300 tactical positions. Each line has a FEN (first 4 fields), followed by `bm` specifying the expected best move in SAN.
- Example line: `2rr3k/pp3pp1/1nnqbN1p/3pN3/2pP4/2P3Q1/PPB4P/R4RK1 w - - bm Qg6; id "WAC.001";`

### `tests/data/ecm.epd`

- Same EPD format as WAC. ECM contains ~600+ positions focused on middlegame combinations.
- Each line has FEN + `bm` + optional `id`.

### `tests/data/sts.epd`

- STS is organized into 15 themed categories (e.g., open files, knight outposts, undermining, pawn play); the EPD file combines all positions into a single file.
- Same format: FEN + `bm` + optional `id`.

### `tools/tactical/Cargo.toml`

- Package name: `tactical`
- Edition: workspace-inherited
- Dependencies: `chess-types`, `chess-board`, `chess-movegen` (workspace crates, specified via `path` references).

### `tools/tactical/src/main.rs`

**Purpose**: CLI binary that runs tactical test suites against the engine via UCI.

**Command-line interface**:
```
tactical --engine <path> --suite <epd_file> --depth <N>
tactical --engine <path> --suite <epd_file> --movetime <ms>
```

Arguments:
- `--engine <path>`: Path to the engine binary (required).
- `--suite <epd_file>`: Path to an EPD file (required). Can be specified multiple times to run multiple suites.
- `--depth <N>`: Fixed depth for `go depth N` mode (mutually exclusive with `--movetime`).
- `--movetime <ms>`: Fixed time in milliseconds for `go movetime T` mode (mutually exclusive with `--depth`).

**Key logic**:

1. **EPD parser**: Parse each line to extract FEN (first 4 fields, with ` 0 1` appended for halfmove/fullmove if not present) and `bm` field(s). The `bm` field may contain multiple moves separated by spaces (any is accepted as correct). Skip comment lines (starting with `#` or `%`) and blank lines.

2. **SAN-to-UCI move resolution**: The EPD `bm` field uses SAN (e.g., `Qg6`, `Nxf7+`). The engine returns `bestmove` in UCI long algebraic (e.g., `g3g6`, `e5f7`). To bridge these notations, the tactical tool uses the engine's own crates (`chess-types`, `chess-board`, `chess-movegen`) to:
   - Parse the FEN into a `Position` using `Position::from_fen`.
   - Generate legal moves via `chess-movegen`'s move generation API.
   - Match each SAN string from the `bm` field against the legal moves by comparing piece type, target square, disambiguation (file and/or rank), capture status, and promotion piece. See "SAN parser" below for the matching algorithm.
   - Convert each matched `Move` to its UCI long algebraic string (from-square + to-square + optional promotion letter).
   - Compare the resulting UCI string(s) against the engine's `bestmove` response. A position is solved if the engine's `bestmove` matches any of the `bm` moves.

3. **Engine communication via UCI**:
   - Spawn the engine binary as a child process with piped stdin/stdout.
   - Send `uci\n`, wait for `uciok`.
   - For each position: send `isready\n`, wait for `readyok`, send `position fen <fen>\n`, send `go depth N\n` or `go movetime T\n`, read lines until `bestmove <move>` is received.
   - After all positions: send `quit\n`.

4. **Solve-rate reporting**:
   - Per suite: print suite file name, number solved, total positions, and percentage.
   - Format: `Suite: wac.epd — Solved: 210/300 (70.0%)`

5. **SAN parser** (within `main.rs`):

   The SAN parser resolves SAN strings against a list of legal moves generated from the position. It handles the following forms:

   - **Piece moves**: Optional piece letter (`K`, `Q`, `R`, `B`, `N`; absent for pawns), optional disambiguation (file letter, rank digit, or both file+rank for cases like `R1a3` or `Qh4e1`), optional `x` for capture, target square (file+rank), optional `+`/`#` for check/checkmate.
   - **Pawn promotions**: Target square + `=` + promotion piece letter (e.g., `e8=Q`, `dxe1=N`).
   - **Castling**: `O-O` (kingside) and `O-O-O` (queenside). Also accept `0-0` and `0-0-0` (zero variant sometimes found in EPD files).

   **Matching algorithm**:
   1. Strip `+`, `#`, `!`, `?` suffixes from the SAN string.
   2. Check for castling notation — if matched, find the corresponding king move in the legal move list.
   3. Otherwise, parse the SAN into components: piece type (default pawn if absent), disambiguation characters, target square, capture flag, and promotion piece.
   4. Filter the legal moves to those matching all parsed components: same piece type, same target square, same promotion piece (if any), and matching any disambiguation (file, rank, or both).
   5. Exactly one legal move should remain after filtering. If so, that is the resolved move.

   **Error handling for SAN resolution failures**:
   - If a SAN string from `bm` does not match any legal move (malformed SAN, illegal position in EPD, or unsupported notation variant): log a warning message identifying the position (FEN and `id` if present) and the unresolvable SAN string, count the position as unsolved, and continue to the next position. Do not abort the entire run.
   - If a SAN string matches more than one legal move (ambiguous SAN): log a warning identifying the position and the ambiguous SAN string, count as unsolved, and continue.

## 4. API Contracts

N/A — this is a standalone CLI binary, not a library.

**CLI interface**:

```
# Fixed-depth mode
cargo run --bin tactical -- --engine target/release/engine --suite tests/data/wac.epd --depth 8

# Fixed-time mode
cargo run --bin tactical -- --engine target/release/engine --suite tests/data/wac.epd --movetime 5000

# Multiple suites
cargo run --bin tactical -- --engine target/release/engine \
  --suite tests/data/wac.epd \
  --suite tests/data/ecm.epd \
  --suite tests/data/sts.epd \
  --depth 10
```

**Output format**:
```
=== Tactical Test Suite ===
Engine: target/release/engine
Mode: depth 8

--- wac.epd (300 positions) ---
  [1/300] WAC.001: expected Qg6, got g3g6 ... PASS
  [2/300] WAC.002: expected Nf6, got d4d5 ... FAIL
  ...
Suite: wac.epd — Solved: 210/300 (70.0%)

--- ecm.epd (668 positions) ---
  ...
Suite: ecm.epd — Solved: 450/668 (67.4%)

=== Summary ===
Total solved: 660/968 (68.2%)
```

## 5. Test Plan

The task spec's exit criteria are:
1. The binary builds and runs successfully.
2. WAC, STS, and ECM EPD suites are present and parseable.
3. The tool successfully communicates with the engine via UCI, sending positions and receiving bestmove responses.
4. Solve rates are reported per suite in both fixed-depth and fixed-time modes.
5. Output format includes: suite name, number solved, total positions, percentage.

**Verification approach** (no automated test crate; verification via running the binary):

- **Build verification**: `cargo build --bin tactical` succeeds without errors.
- **EPD parsing verification**: Run the tool with each suite file; it should parse all lines without errors, reporting the total number of positions found.
- **UCI communication verification**: Run with `--depth 1` (fast) against the engine binary. The tool should successfully get `bestmove` responses for all positions without hanging or crashing.
- **Fixed-depth mode verification**: Run `--depth 8` and observe solve-rate output per suite.
- **Fixed-time mode verification**: Run `--movetime 1000` and observe solve-rate output per suite.
- **Output format verification**: Visually inspect that output includes suite name, number solved, total, and percentage.

## 6. Implementation Order

1. **Create `tools/tactical/Cargo.toml`** — Define the package with dependencies on `chess-types`, `chess-board`, `chess-movegen` (via path references).
2. **Modify `Cargo.toml`** (workspace root) — Add `"tools/tactical"` to the workspace members list.
3. **Create EPD data files** — `tests/data/wac.epd`, `tests/data/ecm.epd`, `tests/data/sts.epd` with positions sourced from publicly available collections.
4. **Create `tools/tactical/src/main.rs`** — Implement in this order:
   a. Argument parsing (engine path, suite files, depth/movetime mode).
   b. EPD parser (extract FEN and `bm` field(s) from each line).
   c. SAN parser and SAN-to-Move resolver using `chess-movegen` to generate legal moves and match parsed SAN components against them. Include handling of file+rank disambiguation (`R1a3`), castling (`O-O`, `O-O-O`, `0-0`, `0-0-0`), and promotions. On SAN resolution failure (no match or ambiguous match), log a warning and count the position as unsolved.
   d. UCI engine communication (spawn process, handshake, position/go/bestmove loop).
   e. Solve-rate tallying and output formatting.
5. **Build and test** — `cargo build --bin tactical`, then run against each suite with `--depth 1` for quick smoke test, then `--depth 8` for actual results.

## 7. Verification Commands

```bash
# Build the engine and tactical tool
cargo build --release --bin engine --bin tactical

# Quick smoke test with depth 1 (fast, verifies UCI communication and EPD parsing)
cargo run --release --bin tactical -- \
  --engine target/release/engine \
  --suite tests/data/wac.epd \
  --depth 1

# Fixed-depth test with WAC
cargo run --release --bin tactical -- \
  --engine target/release/engine \
  --suite tests/data/wac.epd \
  --depth 8

# Fixed-time test with WAC
cargo run --release --bin tactical -- \
  --engine target/release/engine \
  --suite tests/data/wac.epd \
  --movetime 1000

# All three suites at fixed depth
cargo run --release --bin tactical -- \
  --engine target/release/engine \
  --suite tests/data/wac.epd \
  --suite tests/data/ecm.epd \
  --suite tests/data/sts.epd \
  --depth 8

# All three suites at fixed time
cargo run --release --bin tactical -- \
  --engine target/release/engine \
  --suite tests/data/wac.epd \
  --suite tests/data/ecm.epd \
  --suite tests/data/sts.epd \
  --movetime 5000

# Verify clippy and fmt pass
cargo clippy --bin tactical -- -D warnings
cargo fmt --check
```

## 8. Design Deviations

**Deviation 1: Adding `chess-types`, `chess-board`, and `chess-movegen` as dependencies to the tactical tool**

- **Parent spec requires**: The tool communicates with the engine via UCI and compares `bestmove` to the EPD's expected solution.
- **Problem**: EPD files use SAN notation (e.g., `Qg6`, `Nxf7+`) while the engine returns UCI long algebraic notation (e.g., `g3g6`, `e5f7`). Converting between these formats without access to the position's legal moves is error-prone and fragile — SAN is inherently ambiguous without context (e.g., `Nc3` could come from multiple squares, `R1a3` requires both legal move context and rank disambiguation).
- **Alternative chosen**: The tactical binary depends on the workspace crates `chess-types`, `chess-board`, and `chess-movegen` to parse the FEN into a `Position`, generate legal moves, and match SAN strings against them to produce a UCI string for comparison. This is the standard approach used by chess tools (e.g., CuteChess, python-chess) and produces reliable SAN-to-UCI conversion. The tool still communicates with the engine via UCI as specified — the internal crate usage is solely for move notation translation.