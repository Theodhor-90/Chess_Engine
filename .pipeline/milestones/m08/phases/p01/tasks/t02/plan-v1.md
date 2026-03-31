I now have complete context. Here is the implementation plan:

## 1. Deliverables

- **`tests/data/wac.epd`** — EPD file containing the Win At Chess tactical puzzle suite. Each line has a FEN (first 4 fields), followed by `bm` (best move) operations in standard algebraic notation (SAN).
- **`tests/data/ecm.epd`** — EPD file containing the Encyclopedia of Chess Middlegames tactical puzzle suite in EPD format.
- **`tests/data/sts.epd`** — EPD file containing the Strategic Test Suite in EPD format.
- **`tools/tactical/Cargo.toml`** — Cargo package manifest for the tactical test runner binary, declaring it as a workspace member with dependencies on the engine binary path for spawning.
- **`tools/tactical/src/main.rs`** — Standalone binary that loads EPD files, spawns the engine as a child process, communicates via UCI (stdin/stdout), sends `position fen <fen>` + `go depth N` or `go movetime T`, parses `bestmove` responses, compares against the expected best move(s), and reports solve rates per suite.
- **`Cargo.toml`** (modify) — Add `tools/tactical` to the workspace `members` list.

## 2. Dependencies

- The engine binary (`crates/engine`) must be built before running the tactical tool, since it spawns `engine` as a child process.
- No new external crate dependencies. The tactical runner uses only `std` (process spawning via `std::process::Command`, I/O via `std::io`, time via `std::time`).
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

- STS is organized into themed categories (e.g., open files, knight outposts, etc.). The EPD file combines all positions.
- Same format: FEN + `bm` + optional `id`.

### `tools/tactical/Cargo.toml`

- Package name: `tactical`
- Edition: workspace-inherited
- No external dependencies — uses only `std`.

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

1. **EPD parser**: Parse each line to extract FEN (first 4 fields, with ` 0 1` appended for halfmove/fullmove if not present) and `bm` field(s). The `bm` field may contain multiple moves separated by spaces (any is accepted as correct). Skip comment lines (starting with `#`) and blank lines.

2. **SAN-to-UCI move matching**: The EPD `bm` field uses SAN (e.g., `Qg6`, `Nxf7+`). The engine returns `bestmove` in UCI long algebraic (e.g., `g3g6`, `e5f7`). The comparison approach: since implementing a full SAN parser is non-trivial, the tool will:
   - Parse the SAN move from the EPD `bm` field.
   - Send `position fen <fen>` to the engine, then `go depth N` or `go movetime T`.
   - Receive `bestmove <uci_move>`.
   - To compare: generate a list of legal moves from the position by asking the engine. Instead of that complexity, implement a minimal SAN parser that extracts the target square, piece type, and promotion from the SAN string, then match against the UCI move's from/to/promotion. Specifically:
     - Parse the UCI `bestmove` response to get the from-square, to-square, and optional promotion character.
     - Parse each SAN candidate to determine piece type, disambiguation (file/rank), target square, capture flag, promotion, and check indicators.
     - To resolve SAN → UCI comparison without full move generation in the tool: send `position fen <fen>` then `go depth 1` silently first is too complex. Instead, use a simpler approach — compare the bestmove from the engine against all `bm` entries by converting both to a canonical form. Since the engine outputs UCI notation and the EPD has SAN, the tool needs a way to bridge these.

   **Practical approach**: The tool will spawn the engine, set up the position, ask it to search, get the `bestmove` in UCI format. Then to verify if it matches the EPD's SAN `bm`, the tool also asks the engine to search at depth 1 from the position (no, too slow). Better: include a lightweight SAN-to-UCI converter within the tool that uses the `chess-board` and `chess-movegen` crates as dependencies to generate legal moves and match SAN against them.

   **Revised dependency approach**: Add `chess-types`, `chess-board`, and `chess-movegen` as dependencies to the tactical tool. This allows the tool to:
   - Parse the FEN into a `Position`.
   - Generate legal moves via `generate_legal_moves`.
   - Match SAN strings to legal `Move` values by comparing piece type, target square, disambiguation, and promotion.
   - Convert the matched `Move` to UCI string via its `Display` impl.
   - Compare the UCI string against the engine's `bestmove` response.

3. **Engine communication via UCI**:
   - Spawn the engine binary as a child process with piped stdin/stdout.
   - Send `uci\n`, wait for `uciok`.
   - For each position: send `isready\n`, wait for `readyok`, send `position fen <fen>\n`, send `go depth N\n` or `go movetime T\n`, read lines until `bestmove <move>` is received.
   - After all positions: send `quit\n`.

4. **Solve-rate reporting**:
   - Per suite: print suite file name, number solved, total positions, and percentage.
   - Format: `Suite: wac.epd — Solved: 210/300 (70.0%)`

5. **SAN parser** (minimal, within `main.rs`):
   - Handles standard SAN: piece letter (K/Q/R/B/N, absent for pawn), optional disambiguation (file letter or rank digit), optional `x` for capture, target square, optional `=Q/R/B/N` for promotion, optional `+`/`#` for check/checkmate.
   - Castling: `O-O` and `O-O-O`.
   - Matches parsed SAN components against the generated legal moves to find the unique matching `Move`.

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

1. **Create `tools/tactical/Cargo.toml`** — Define the package with dependencies on `chess-types`, `chess-board`, `chess-movegen`.
2. **Modify `Cargo.toml`** (workspace root) — Add `"tools/tactical"` to the workspace members list.
3. **Create EPD data files** — `tests/data/wac.epd`, `tests/data/ecm.epd`, `tests/data/sts.epd` with positions sourced from publicly available collections.
4. **Create `tools/tactical/src/main.rs`** — Implement in this order:
   a. Argument parsing (engine path, suite files, depth/movetime mode).
   b. EPD parser (extract FEN and `bm` field(s) from each line).
   c. Minimal SAN parser + SAN-to-Move resolver using `chess-movegen::generate_legal_moves`.
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
- **Problem**: EPD files use SAN notation (e.g., `Qg6`, `Nxf7+`) while the engine returns UCI long algebraic notation (e.g., `g3g6`, `e5f7`). Converting between these formats without access to the position's legal moves is error-prone and fragile — SAN is inherently ambiguous without context (e.g., `Nc3` could come from multiple squares).
- **Alternative chosen**: The tactical binary depends on `chess-types`, `chess-board`, and `chess-movegen` crate to parse the FEN into a `Position`, generate legal moves, and match SAN strings against them to produce a UCI string for comparison. This is the standard approach used by chess tools (e.g., CuteChess, python-chess) and produces reliable SAN-to-UCI conversion. The tool still communicates with the engine via UCI as specified — the internal crate usage is solely for move notation translation.