Now I have a thorough understanding of the codebase. Let me produce the implementation plan.

## 1. Deliverables

| # | File | Purpose |
|---|------|---------|
| 1 | `tools/pgn-extract/Cargo.toml` | Crate manifest for the PGN extraction tool binary |
| 2 | `tools/pgn-extract/src/main.rs` | PGN-to-tuning-dataset extractor: reads CCRL PGN files, replays games, filters quiet positions, outputs `FEN|result` lines compatible with the Texel tuner |
| 3 | `Cargo.toml` (workspace root) | Updated `members` to include `tools/pgn-extract` |
| 4 | `crates/eval/src/material.rs` | Updated material value constants with Texel-tuned values |
| 5 | `crates/eval/src/pst.rs` | Updated PST arrays (all 12 tables) with Texel-tuned values |
| 6 | `crates/eval/src/pawn.rs` | Updated pawn structure weight constants with Texel-tuned values |
| 7 | `crates/eval/src/king_safety.rs` | Updated king safety weight constants with Texel-tuned values |
| 8 | `crates/eval/src/mobility.rs` | Updated mobility weight constants with Texel-tuned values |
| 9 | `crates/eval/src/pieces.rs` | Updated piece bonus constants with Texel-tuned values |
| 10 | `crates/eval/src/king_centralization.rs` | Updated king centralization weight constants with Texel-tuned values |

## 2. Dependencies

- **T01 (Texel tuner)**: Must be complete — the `tools/texel` binary is used to run the optimization.
- **Phase P01/T03 (Self-play framework)**: Must be complete — the `tools/selfplay` binary is needed for SPRT validation.
- **External data**: CCRL game database in PGN format (downloaded externally to a local path, e.g., `data/ccrl.pgn`).
- **Workspace crate dependencies for `tools/pgn-extract`**: `chess-types`, `chess-board`, `chess-movegen` (to replay games and check quiescence).
- No new external crate dependencies needed — PGN parsing is simple enough to implement inline using standard library I/O, consistent with the project's convention of minimizing external crates.

## 3. Implementation Details

### 3.1 `tools/pgn-extract/Cargo.toml`

**Purpose**: Define the PGN extraction tool binary as a workspace member.

```toml
[package]
name = "pgn-extract"
version = "0.1.0"
edition.workspace = true

[dependencies]
chess-types = { path = "../../crates/types" }
chess-board = { path = "../../crates/board" }
chess-movegen = { path = "../../crates/movegen" }
```

### 3.2 `tools/pgn-extract/src/main.rs`

**Purpose**: Read CCRL PGN files and produce a filtered dataset of quiet positions with game outcomes in the `FEN|result` format the Texel tuner consumes.

**CLI interface**:
- `--pgn <path>` — path to input PGN file
- `--output <path>` — path to output dataset file
- `--max-positions <N>` — optional cap on number of positions to extract (default: unlimited)
- `--skip-first <N>` — number of plies to skip from each game's start (default: 10, to avoid book positions)

**Key logic**:

1. **PGN parsing**: Read the PGN file line-by-line. Parse header tags (specifically `[Result "..."]`). Parse movetext by extracting SAN move tokens, skipping move numbers, comments, and annotations. Map the `Result` tag: `"1-0"` → `1.0`, `"0-1"` → `0.0`, `"1/2-1/2"` → `0.5`. Skip games with `"*"` result.

2. **Game replay**: For each parsed game, start from the standard starting position (or from `[FEN "..."]` if present) and replay each SAN move using `chess_movegen::generate_legal_moves` to find the matching legal move, then `Position::make_move`.

3. **SAN move matching**: Parse the SAN token to extract: piece kind (or pawn if none), destination square, disambiguation (file/rank), promotion piece, capture flag. Match against legal moves by comparing piece kind, destination, and disambiguation.

4. **Quiescence filter**: After each move (beyond `skip-first` plies), check if the position is quiet:
   - Side to move is NOT in check (use `Position::is_square_attacked` on king square)
   - No captures available (generate legal moves, check none have a capture flag or land on an occupied square — alternatively, check if any legal move captures a piece by checking if the destination square is occupied by the opponent)

   Only emit positions that pass both checks.

5. **Output format**: Write one line per position: `<FEN>|<result>` where result is `1.0`, `0.5`, or `0.0` (from white's perspective, matching the Texel tuner's expected format from T01's `load_dataset` function).

6. **Progress reporting**: Print to stderr every 10,000 games processed showing game count and position count.

**Exports**: None (binary only).

### 3.3 `Cargo.toml` (workspace root)

**Purpose**: Add `tools/pgn-extract` to the workspace members list.

Change: Append `"tools/pgn-extract"` to the `members` array, inserting it before `"crates/engine"`.

### 3.4–3.10 Eval crate source files (tuned weight updates)

**Purpose**: Replace the existing hand-tuned evaluation constants with Texel-optimized values.

**Process**: Run the Texel tuner (`tools/texel`) on the extracted dataset, then use its stdout output to update each file's constants. The tuner's `print_weights` function (from T01) already outputs constants organized by file with matching names.

**Files and constants to update**:

- **`material.rs`**: `PAWN_VALUE`, `KNIGHT_VALUE`, `BISHOP_VALUE`, `ROOK_VALUE`, `QUEEN_VALUE` (5 constants)
- **`pst.rs`**: All 12 PST arrays: `MG_PAWN_TABLE`, `EG_PAWN_TABLE`, `MG_KNIGHT_TABLE`, `EG_KNIGHT_TABLE`, `MG_BISHOP_TABLE`, `EG_BISHOP_TABLE`, `MG_ROOK_TABLE`, `EG_ROOK_TABLE`, `MG_QUEEN_TABLE`, `EG_QUEEN_TABLE`, `MG_KING_TABLE`, `EG_KING_TABLE` (768 values total)
- **`pawn.rs`**: `DOUBLED_PAWN_PENALTY_MG`, `DOUBLED_PAWN_PENALTY_EG`, `ISOLATED_PAWN_PENALTY_MG`, `ISOLATED_PAWN_PENALTY_EG`, `BACKWARD_PAWN_PENALTY_MG`, `BACKWARD_PAWN_PENALTY_EG`, `CONNECTED_PAWN_BONUS_MG`, `CONNECTED_PAWN_BONUS_EG`, `BLOCKED_PASSED_PAWN_PENALTY_MG`, `BLOCKED_PASSED_PAWN_PENALTY_EG`, `PASSED_PAWN_BONUS_MG`, `PASSED_PAWN_BONUS_EG`, `KING_PROXIMITY_OWN_BONUS`, `KING_PROXIMITY_OPP_BONUS` (14 constants/arrays)
- **`king_safety.rs`**: `PAWN_SHIELD_BONUS_MG`, `PAWN_SHIELD_BONUS_EG`, `OPEN_FILE_NEAR_KING_PENALTY_MG`, `OPEN_FILE_NEAR_KING_PENALTY_EG`, `KNIGHT_ATTACK_WEIGHT`, `BISHOP_ATTACK_WEIGHT`, `ROOK_ATTACK_WEIGHT`, `QUEEN_ATTACK_WEIGHT`, `ATTACK_SCORE_SCALE_MG`, `ATTACK_SCORE_SCALE_EG` (10 constants/arrays)
- **`mobility.rs`**: `KNIGHT_MOBILITY_MG`, `KNIGHT_MOBILITY_EG`, `BISHOP_MOBILITY_MG`, `BISHOP_MOBILITY_EG`, `ROOK_MOBILITY_MG`, `ROOK_MOBILITY_EG`, `QUEEN_MOBILITY_MG`, `QUEEN_MOBILITY_EG` (8 constants)
- **`pieces.rs`**: `BISHOP_PAIR_BONUS_MG`, `BISHOP_PAIR_BONUS_EG`, `ROOK_OPEN_FILE_BONUS_MG`, `ROOK_OPEN_FILE_BONUS_EG`, `ROOK_SEMI_OPEN_FILE_BONUS_MG`, `ROOK_SEMI_OPEN_FILE_BONUS_EG`, `ROOK_SEVENTH_RANK_BONUS_MG`, `ROOK_SEVENTH_RANK_BONUS_EG`, `KNIGHT_OUTPOST_BONUS_MG`, `KNIGHT_OUTPOST_BONUS_EG` (10 constants)
- **`king_centralization.rs`**: `KING_CENTER_BONUS_WEIGHT`, `KING_PASSED_PAWN_PROXIMITY_WEIGHT` (2 constants)

**Contract**: Only the numeric values of these constants change. No function signatures, types, visibility modifiers, or logic change. All existing tests must still pass with the new values (the tests are relative/comparative, not absolute value checks).

## 4. API Contracts

### PGN extraction tool CLI

**Input**: PGN file in standard PGN format (one or more games).

**Output**: Text file with one position per line in format:
```
<FEN>|<result>
```
Example:
```
rnbqkb1r/pp2pppp/2p2n2/3p4/2PP4/5N2/PP2PPPP/RNBQKB1R w KQkq - 0 4|0.5
r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3|1.0
```

This format is directly consumed by the Texel tuner's `load_dataset` function (T01), which splits on `|` and parses the FEN with `Position::from_fen` and the result as `f64`.

### Texel tuner (existing, from T01)

**Input**: `--data <path>` pointing to the dataset file produced above.

**Output**: Rust `const` declarations printed to stdout, one per constant, organized by source file with comments.

### Updated eval crate

**Input/output contracts unchanged** — all public function signatures remain identical. Only internal constant values change.

## 5. Test Plan

### 5.1 PGN extraction tool — verify dataset meets requirements

**Test**: Run the extractor on the CCRL PGN database and verify:
- Output file contains at least 1,000,000 lines (1 million positions)
- Each line matches the `<FEN>|<result>` format where result is 0.0, 0.5, or 1.0
- Sample positions are parseable by `Position::from_fen`
- Verify quiescence filter: sample 100 random positions from the output, load each, and confirm the side to move is not in check and no immediate captures are available

**Setup**: Run `cargo build --release -p pgn-extract`, then `./target/release/pgn-extract --pgn data/ccrl.pgn --output data/ccrl_quiet.txt`

### 5.2 Texel tuner produces optimized weights

**Test**: Run the tuner on the extracted dataset and verify:
- Tuner completes without error
- Output contains all expected constant names
- Final MSE is lower than initial MSE (tuner improves the fit)

**Setup**: `cargo run --release -p texel -- --data data/ccrl_quiet.txt --iterations 100 > tuned_weights.txt 2>tuning_log.txt`

### 5.3 Tuned weights compile and pass existing tests

**Test**: After applying tuned weights to eval crate source files:
- `cargo build` succeeds
- `cargo test` passes (all existing eval tests must pass)
- `cargo clippy -- -D warnings` passes
- `cargo fmt --check` passes

### 5.4 SPRT validation of Elo gain

**Test**: Using the self-play framework from P01/T03, run SPRT testing comparing the tuned engine vs. the untuned engine:
- Build the untuned engine first (from the git state before weight changes)
- Build the tuned engine (with new weights)
- Run SPRT with bounds Elo0=0, Elo1=10, alpha=0.05, beta=0.05
- The test passes when SPRT accepts H1 (tuned engine is at least +10 Elo stronger), confirming the +20 Elo requirement

**Setup**: This requires building two engine binaries (pre-tune and post-tune) and running them against each other using the selfplay framework or an external match tool.

## 6. Implementation Order

1. **Create `tools/pgn-extract/Cargo.toml`** and add `tools/pgn-extract` to workspace `Cargo.toml` members.

2. **Implement `tools/pgn-extract/src/main.rs`** — PGN parser, game replayer, quiescence filter, `FEN|result` output writer. Verify it compiles with `cargo build -p pgn-extract`.

3. **Download CCRL PGN database** to `data/ccrl.pgn` (external step, not automated).

4. **Extract dataset** — run `pgn-extract` on the CCRL database to produce `data/ccrl_quiet.txt` with 1M+ quiet positions.

5. **Verify dataset** — check line count (`wc -l data/ccrl_quiet.txt` ≥ 1,000,000), sample format validity.

6. **Run Texel tuner** — `cargo run --release -p texel -- --data data/ccrl_quiet.txt --iterations 100 > tuned_weights.txt`. Verify MSE decreases.

7. **Apply tuned weights** to all eval crate source files (`material.rs`, `pst.rs`, `pawn.rs`, `king_safety.rs`, `mobility.rs`, `pieces.rs`, `king_centralization.rs`), replacing existing constant values with tuned values from `tuned_weights.txt`.

8. **Verify compilation and tests** — run `cargo fmt --check && cargo clippy -- -D warnings && cargo test`.

9. **SPRT validation** — build pre-tune engine binary (from git stash/branch), build post-tune engine binary, run SPRT self-play at Elo0=0, Elo1=10, alpha=0.05, beta=0.05, confirm tuned engine gains ≥ +20 Elo.

## 7. Verification Commands

```bash
# Build the PGN extraction tool
cargo build --release -p pgn-extract

# Extract quiet positions from CCRL PGN (assumes data/ccrl.pgn exists)
./target/release/pgn-extract --pgn data/ccrl.pgn --output data/ccrl_quiet.txt --skip-first 10

# Verify dataset has at least 1M positions
wc -l data/ccrl_quiet.txt

# Run Texel tuner on the dataset
cargo run --release -p texel -- --data data/ccrl_quiet.txt --iterations 100 > tuned_weights.txt 2>tuning_log.txt

# After applying tuned weights to eval crate source files:
cargo fmt --check
cargo clippy -- -D warnings
cargo test

# Build tuned engine for SPRT testing
cargo build --release -p engine

# SPRT validation (using selfplay framework, comparing tuned vs untuned binaries)
# First, save untuned binary before applying weights:
# cp target/release/engine target/release/engine-untuned
# Then after applying weights and rebuilding:
# Run SPRT match (details depend on match runner setup from P01/T03 or P03)
```

## 8. Design Deviations

**Deviation 1: PGN extraction tool as a separate binary instead of a script**

- **Parent spec**: The task spec lists "A position extraction/filtering script or tool to process CCRL PGN data into the tuner's input format" as a deliverable.
- **Why a binary**: The project is entirely Rust with no scripting infrastructure (no Python, no Node.js). Processing PGN files requires replaying chess moves, which requires the engine's move generation and position representation. A Rust binary can directly link against `chess-board`, `chess-types`, and `chess-movegen` to replay games in-process, ensuring correct SAN parsing using the same legal move generation the engine uses. A script would need to either shell out to an external tool or reimplement move generation.
- **Alternative chosen**: `tools/pgn-extract` as a Rust binary workspace member, consistent with the existing `tools/perft`, `tools/tactical`, `tools/selfplay`, and `tools/texel` pattern.

**Deviation 2: SAN parsing implemented inline rather than using an external PGN library**

- **Parent spec**: Does not prescribe an approach for PGN parsing.
- **Why inline**: The project's coding conventions mandate minimizing external crates. PGN parsing for this use case is straightforward (header extraction, SAN movetext tokenization) and can be implemented in ~150 lines. The critical part — SAN-to-move resolution — must use the engine's own `generate_legal_moves` anyway, so an external library would add dependency bloat with minimal benefit.
- **Alternative chosen**: Inline PGN tokenizer using `std::io::BufRead` and `std::str` methods, with legal move matching via `chess_movegen::generate_legal_moves`.