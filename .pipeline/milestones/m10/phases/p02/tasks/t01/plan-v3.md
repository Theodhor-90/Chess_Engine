Now I have all the context needed. Let me produce the revised plan.

## 1. Deliverables

- **Create** `tools/datagen/Cargo.toml` — binary crate manifest for the training data generation tool
- **Create** `tools/datagen/src/main.rs` — self-play data generation binary that plays games using the handcrafted engine and exports FEN + eval + game result records
- **Modify** `Cargo.toml` (workspace root) — add `tools/datagen` to workspace members

## 2. Dependencies

- `chess-types = { path = "../../crates/types" }` — `Color`, `Move`, `Piece`, `PieceKind`, `Square`, `Bitboard`
- `chess-board = { path = "../../crates/board" }` — `Position`, `from_fen`, `to_fen`, `make_move`, `hash`, `halfmove_clock`, `side_to_move`, `is_square_attacked`, `piece_bitboard`, `occupied_by`
- `chess-movegen = { path = "../../crates/movegen" }` — `generate_legal_moves`
- `chess-search = { path = "../../crates/search" }` — `search`, `SearchLimits`, `DepthCallback`

All dependencies are workspace-internal. No new external crates are required.

Note: `chess-eval` is **not** listed as a dependency. The training data records the **search score** captured via `DepthCallback`, not the raw `evaluate()` output. The search score is more appropriate because it reflects the engine's actual assessment of the position (incorporating tree search, quiescence, etc.), which is standard practice for NNUE training data generation. The task spec's phrase "handcrafted evaluation score" refers to the score produced by the handcrafted engine (which uses the handcrafted eval internally during search), not a direct call to `evaluate()`.

## 3. Implementation Details

### `tools/datagen/Cargo.toml`

```toml
[package]
name = "datagen"
version = "0.1.0"
edition.workspace = true

[dependencies]
chess-types = { path = "../../crates/types" }
chess-board = { path = "../../crates/board" }
chess-movegen = { path = "../../crates/movegen" }
chess-search = { path = "../../crates/search" }
```

### `tools/datagen/src/main.rs`

**Purpose**: Self-play data generation binary. Plays games using the engine's search internally (not via UCI subprocess) and exports training data.

**Structs**:

```rust
struct Args {
    openings: String,   // path to openings file (FEN per line, same format as tools/selfplay/openings.txt)
    games: u32,         // number of games to play
    nodes: u64,         // node limit per move (search budget control)
    output: String,     // output file path
    max_moves: u32,     // maximum ply per game (default 300)
}
```

```rust
struct TrainingRecord {
    fen: String,    // position FEN
    eval: i32,      // search score in centipawns from side-to-move perspective
    result: f64,    // game result from white's perspective: 1.0 (white win), 0.5 (draw), 0.0 (black win)
}
```

**Output format**: Plain text, one record per line: `<FEN> | <eval> | <result>`

This is the `nnue-pytorch` `.plain` text format, documented at https://github.com/glinscott/nnue-pytorch as pipe-delimited `<FEN> | <score> | <result>`. For `bullet`, this same text format is used as input to its data conversion utilities. The existing texel tuner in this project (`tools/texel`) already uses `<FEN> | <result>` pipe-delimited format, so this is a natural extension.

Example output lines:
```
rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1 | 35 | 1.0
rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2 | 28 | 1.0
```

**Functions**:

- `fn main()` — parse CLI args, load openings, run game loop, write output file, print summary statistics (total games, total positions, positions/game average)
- `fn parse_args() -> Args` — parse `--openings`, `--games`, `--nodes`, `--output`, `--maxmoves` from `env::args()`. All are required except `--maxmoves` (default 300). Print usage and exit on error. Follow the same arg-parsing pattern as `tools/selfplay`.
- `fn load_openings(path: &str) -> Vec<String>` — read FEN lines from file, skip empty lines and `#` comments. Identical to `tools/selfplay`'s implementation.
- `fn play_game(opening_fen: &str, node_limit: u64, max_moves: u32) -> (Vec<TrainingRecord>, Option<GameResult>)` — plays a single self-play game:
  1. Initialize `Position::from_fen(opening_fen)`
  2. Maintain `hash_history: Vec<u64>` for repetition detection (same pattern as `tools/selfplay`)
  3. Loop: for each ply, call `chess_search::search()` with `SearchLimits { max_time: Duration::from_secs(3600), max_depth: None, max_nodes: Some(node_limit), stop_flag: None }` (time is set very high so node limit is the effective constraint)
  4. Use a `Cell<i32>` for interior mutability to capture the score from the depth callback (see score capture section below)
  5. After obtaining the best move from `search()`, record a `TrainingRecord` with `pos.to_fen()`, the captured search score via `last_score.get()`, and a placeholder result of `0.0`
  6. Apply the move via `pos.make_move(mv)` — the returned `UndoInfo` is discarded (the game loop only moves forward; `unmake_move` is never called because we do not need to restore previous positions)
  7. Push the new position hash to `hash_history`
  8. Check termination: no legal moves (checkmate/stalemate), `pos.halfmove_clock() >= 100` (fifty-move), threefold repetition, insufficient material, ply ≥ max_moves
  9. After game ends, determine `GameResult` (WhiteWins/BlackWins/Draw) and backfill the `result` field in all records: white win → `1.0`, draw → `0.5`, black win → `0.0`
  10. Return the collected records and game result

  Note on position state during search: `chess_search::search()` takes `&mut Position` and makes/unmakes moves internally during tree exploration, but **restores the position to its original state before returning**. This is confirmed by the search implementation which pairs every `make_move` with `unmake_move` during alpha-beta traversal. Therefore the position is in the correct state after `search()` returns, ready for the next `make_move` call.

- `fn is_in_check(pos: &Position) -> bool` — find the king square for the side to move, check `pos.is_square_attacked(king_sq, opponent_color)`. Same implementation as `tools/selfplay`.
- `fn is_insufficient_material(pos: &Position) -> bool` — same implementation as `tools/selfplay`: detect K vs K, K+B vs K, K+N vs K.
- `fn count_repetitions(hash_history: &[u64], current_hash: u64) -> u32` — same implementation as `tools/selfplay`.
- `fn write_records(path: &str, records: &[TrainingRecord])` — append records to output file, one per line in `<FEN> | <eval> | <result>` format.

**Game result enum** (reused from selfplay pattern):
```rust
enum GameResult {
    WhiteWins,
    BlackWins,
    Draw,
}
```

**Score capture via depth callback using `Cell` for interior mutability**: The `search()` function accepts an `Option<DepthCallback<'_>>` where `DepthCallback<'a> = &'a dyn Fn(u8, i32, u64, Duration, &[Move])`. Since `DepthCallback` requires `Fn` (not `FnMut`), direct mutation of a captured variable is not allowed. Use `std::cell::Cell<i32>` for interior mutability — `Cell::set()` does not require `&mut self`, so the closure remains `Fn`-compatible:

```rust
use std::cell::Cell;

let last_score: Cell<i32> = Cell::new(0);
let callback: chess_search::DepthCallback<'_> = &|_depth, score, _nodes, _elapsed, _pv| {
    last_score.set(score);
};
let best_move = chess_search::search(
    &mut pos,
    limits.clone(),
    &hash_history,
    Some(&callback),
    None,
);
// Read the captured score
let score = last_score.get();
```

This is the standard Rust pattern for mutating state inside a `Fn` closure. `Cell<i32>` is safe because `i32` is `Copy` and we are single-threaded (no `Sync` requirement). The existing engine callback (`crates/engine/src/main.rs:304`) avoids needing mutation by only calling `println!`, but datagen needs to retain the score value.

Note: `last_score.get()` returns the score from side-to-move perspective (same as the `evaluate()` function).

**No position filtering**: Every position in every game is recorded. The task spec says "for each position in each game, exports the FEN, the handcrafted evaluation score, and the game result." No positions are skipped based on check status, tactical volatility, or ply count.

### Workspace `Cargo.toml` modification

Add `"tools/datagen"` to the `members` array.

## 4. API Contracts

**CLI interface**:
```
datagen --openings <path> --games <N> --nodes <N> --output <path> [--maxmoves <N>]
```

- `--openings <path>` — path to openings file (one FEN per line, `#` comments allowed)
- `--games <N>` — number of self-play games to generate
- `--nodes <N>` — node limit per move (search budget control, e.g., 5000)
- `--output <path>` — output file path for training records
- `--maxmoves <N>` — maximum ply per game (default: 300)

**Output file format** (`nnue-pytorch` `.plain` compatible):
```
<FEN> | <eval_centipawns> | <result>
```
- `<FEN>`: standard 6-field FEN string
- `<eval_centipawns>`: integer, search score in centipawns from side-to-move perspective
- `<result>`: game outcome from white's perspective — `1.0` for white win, `0.5` for draw, `0.0` for black win

N/A for REST/HTTP.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| `cargo build -p datagen` | Binary compiles successfully |
| `cargo clippy -p datagen -- -D warnings` | No lint warnings |
| `cargo fmt --check -p datagen` | Formatting is correct |
| Run `datagen --openings tools/selfplay/openings.txt --games 2 --nodes 1000 --output /tmp/test_train.txt --maxmoves 100` | Tool runs and produces output file |
| Parse output file lines | Each line matches `<FEN> \| <eval> \| <result>` format; FEN is valid (parseable by `Position::from_fen`); eval is an integer; result is `1.0`, `0.5`, or `0.0` |
| Verify game count | After `--games 2`, output file contains positions from 2 games (non-zero number of records) |
| Verify result consistency | All records from the same game have the same result value |
| Verify FEN validity | Each FEN in the output is parseable by `Position::from_fen` without error |

## 6. Implementation Order

1. `Cargo.toml` (workspace root) — add `"tools/datagen"` to workspace members
2. `tools/datagen/Cargo.toml` — create the binary crate manifest
3. `tools/datagen/src/main.rs` — implement in this order:
   a. `Args` struct and `parse_args()` function
   b. `load_openings()` function
   c. Helper functions: `is_in_check`, `is_insufficient_material`, `count_repetitions`
   d. `TrainingRecord` struct, `GameResult` enum, and `write_records()` function
   e. `play_game()` function with search integration and `Cell`-based score capture
   f. `main()` function wiring everything together with progress output

## 7. Verification Commands

```bash
cargo fmt --check -p datagen
cargo clippy -p datagen -- -D warnings
cargo build -p datagen
cargo run -p datagen -- --openings tools/selfplay/openings.txt --games 2 --nodes 1000 --output /tmp/test_train.txt --maxmoves 100
head -20 /tmp/test_train.txt
cargo test
```

## 8. Design Deviations

**Deviation 1: Internal search calls instead of UCI subprocess spawning**

- **Parent spec says**: "The self-play framework from M08 provides game-management infrastructure; this tool adds evaluation export on top"
- **Why direct approach is better**: The M08 selfplay tool (`tools/selfplay`) spawns the engine as a UCI subprocess via `Command::new(path)` and communicates over stdin/stdout. This approach was appropriate for M08 because it needed to test the engine binary end-to-end (detecting crashes, illegal moves, assertion failures). For datagen, we need to capture the search score at each position, which requires either: (a) parsing UCI `info` lines from stdout to extract `score cp <N>`, or (b) calling `chess_search::search()` directly with a depth callback. Option (b) is simpler, more reliable, avoids the complexity of UCI parsing and subprocess management, and gives direct access to the search score via the `DepthCallback`. The datagen tool is not testing the engine binary — it is generating training data, so process isolation provides no benefit.
- **Alternative chosen**: Call `chess_search::search()` directly as a library function, using the `DepthCallback` to capture the score from the deepest completed search iteration. This is the same approach the engine binary itself uses internally.

**Deviation 2: Node limit (`--nodes`) instead of time control (`--movetime`)**

- **Parent spec says**: Configurable "time control" (task spec verification criterion 5: "Game count, time control, and output path are configurable via command-line arguments")
- **Why node limit is more appropriate**: Time-based search produces non-deterministic results (different positions recorded depending on CPU speed and system load). Node limits produce consistent search depth regardless of hardware, which is standard practice for NNUE data generation. The `SearchLimits` struct already supports `max_nodes: Option<u64>`.
- **Spec interpretation**: The `--nodes` parameter satisfies the "time control" criterion from the task spec. "Time control" in the spec refers to the mechanism controlling search budget per move — the concept of how much computation to spend per move. Node limits are a deterministic form of search budget control and are functionally equivalent to time control for the purpose of this tool. The spec's intent is that the user can configure how hard the engine thinks per move; `--nodes` achieves this more reliably than `--movetime` for training data generation.
- **Alternative chosen**: Use `--nodes` parameter for consistent, reproducible data generation. This is the industry-standard approach for NNUE datagen tools.