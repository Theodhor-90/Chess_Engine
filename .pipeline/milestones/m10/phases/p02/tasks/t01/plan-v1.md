Now I have everything I need. Let me produce the implementation plan.

## 1. Deliverables

- **Create** `tools/datagen/Cargo.toml` — binary crate manifest for the training data generation tool
- **Create** `tools/datagen/src/main.rs` — self-play data generation binary that plays games using the handcrafted engine and exports FEN + eval + game result records
- **Modify** `Cargo.toml` (workspace root) — add `tools/datagen` to workspace members

## 2. Dependencies

- `chess-types = { path = "../../crates/types" }` — `Color`, `Move`, `Piece`, `PieceKind`, `Square`, `Bitboard`
- `chess-board = { path = "../../crates/board" }` — `Position`, `from_fen`, `to_fen`, `make_move`, `unmake_move`, `hash`
- `chess-movegen = { path = "../../crates/movegen" }` — `generate_legal_moves`
- `chess-search = { path = "../../crates/search" }` — `search`, `SearchLimits`, `DepthCallback`
- `chess-eval = { path = "../../crates/eval" }` — `evaluate`, `PawnHashTable` (for standalone position evaluation)

All dependencies are workspace-internal. No new external crates are required.

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
chess-eval = { path = "../../crates/eval" }
```

### `tools/datagen/src/main.rs`

**Purpose**: Self-play data generation binary. Plays games using the engine's search internally (not via UCI subprocess) and exports training data.

**Structs**:

```rust
struct Args {
    openings: String,   // path to openings file (FEN per line, same format as tools/selfplay/openings.txt)
    games: u32,         // number of games to play
    nodes: u64,         // node limit per move (soft limit — controls search strength/speed tradeoff)
    output: String,     // output file path
    max_moves: u32,     // maximum ply per game (default 300)
    min_ply: u32,       // minimum ply before recording positions (default 16, skip early opening)
}
```

```rust
struct TrainingRecord {
    fen: String,    // position FEN
    eval: i32,      // handcrafted evaluation in centipawns from side-to-move perspective
    result: f64,    // game result from white's perspective: 1.0 (white win), 0.5 (draw), 0.0 (black win)
}
```

**Output format**: Plain text, one record per line. Format: `<FEN> | <eval> | <result>`

This format is chosen because:
- The task spec requires compatibility with both `bullet` and `nnue-pytorch`
- `nnue-pytorch` supports `.plain` text format natively
- `bullet` can accept text-based input for conversion
- The existing texel tuner in this project already uses `<FEN> | <result>` format (pipe-delimited), so adding an eval field is a natural extension
- Plain text is human-readable and trivially parseable by any training pipeline

Example output lines:
```
rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1 | 35 | 1.0
rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2 | 28 | 1.0
```

**Functions**:

- `fn main()` — parse CLI args, load openings, run game loop, write output file, print summary statistics (total games, total positions, positions/game average)
- `fn parse_args() -> Args` — parse `--openings`, `--games`, `--nodes`, `--output`, `--maxmoves`, `--minply` from `env::args()`. All are required except `--maxmoves` (default 300) and `--minply` (default 16). Print usage and exit on error. Follow the same arg-parsing pattern as `tools/selfplay`.
- `fn load_openings(path: &str) -> Vec<String>` — read FEN lines from file, skip empty lines and `#` comments. Identical to `tools/selfplay`'s implementation.
- `fn play_game(opening_fen: &str, node_limit: u64, max_moves: u32, min_ply: u32) -> (Vec<TrainingRecord>, Option<GameResult>)` — plays a single self-play game:
  1. Initialize `Position::from_fen(opening_fen)`
  2. Maintain `hash_history: Vec<u64>` for repetition detection (same pattern as `tools/selfplay`)
  3. Loop: for each ply, call `chess_search::search()` with `SearchLimits { max_time: Duration::from_secs(3600), max_depth: None, max_nodes: Some(node_limit), stop_flag: None }` (time is set very high so node limit is the effective constraint)
  4. Use the depth callback to capture the score from the deepest completed iteration
  5. After obtaining the best move from `search()`, if `ply >= min_ply` and position is not in check and the move is not a capture/promotion (quiet position filtering), record a `TrainingRecord` with `pos.to_fen()`, the captured search score, and a placeholder result of `0.0`
  6. Apply the move via `pos.make_move(mv)`, push hash to history
  7. Check termination: no legal moves (checkmate/stalemate), halfmove clock ≥ 100 (fifty-move), threefold repetition, insufficient material, ply ≥ max_moves
  8. After game ends, determine `GameResult` (WhiteWins/BlackWins/Draw) and backfill the `result` field in all records: white win → `1.0`, draw → `0.5`, black win → `0.0`
  9. Return the collected records and game result

- `fn is_in_check(pos: &Position) -> bool` — same implementation as `tools/selfplay`: find king square, check `is_square_attacked`
- `fn is_insufficient_material(pos: &Position) -> bool` — same implementation as `tools/selfplay`
- `fn count_repetitions(hash_history: &[u64], current_hash: u64) -> u32` — same implementation as `tools/selfplay`
- `fn is_capture(pos: &Position, mv: Move) -> bool` — check if the destination square of `mv` is occupied in `pos.occupied()` before the move, or if the move is en passant
- `fn write_records(path: &str, records: &[TrainingRecord])` — append records to output file, one per line in `<FEN> | <eval> | <result>` format

**Game result enum** (reused from selfplay pattern):
```rust
enum GameResult {
    WhiteWins,
    BlackWins,
    Draw,
}
```

**Score capture via depth callback**: The `search()` function accepts an `Option<DepthCallback<'_>>` where `DepthCallback<'a> = &'a dyn Fn(u8, i32, u64, Duration, &[Move])`. The second parameter is the score. Use a mutable variable captured by the closure to store the latest score from the deepest completed iteration:
```rust
let mut last_score: i32 = 0;
let callback: chess_search::DepthCallback<'_> = &|_depth, score, _nodes, _elapsed, _pv| {
    last_score = score;
};
let best_move = chess_search::search(
    &mut pos,
    limits.clone(),
    &hash_history,
    Some(&callback),
    None,
);
```
Note: `last_score` is from side-to-move perspective (same as the `evaluate()` function).

**Quiet position filtering**: Only record positions where:
1. `ply >= min_ply` (skip early opening book territory)
2. Position is not in check
3. The best move is not a capture (destination square not occupied) — this heuristic avoids recording tactically volatile positions

### Workspace `Cargo.toml` modification

Add `"tools/datagen"` to the `members` array.

## 4. API Contracts

**CLI interface**:
```
datagen --openings <path> --games <N> --nodes <N> --output <path> [--maxmoves <N>] [--minply <N>]
```

- `--openings <path>` — path to openings file (one FEN per line, `#` comments allowed)
- `--games <N>` — number of self-play games to generate
- `--nodes <N>` — node limit per move (e.g., 5000)
- `--output <path>` — output file path for training records
- `--maxmoves <N>` — maximum ply per game (default: 300)
- `--minply <N>` — minimum ply before recording positions (default: 16)

**Output file format**:
```
<FEN> | <eval_centipawns> | <result>
```
- `<FEN>`: standard 6-field FEN string
- `<eval_centipawns>`: integer, search score in centipawns from side-to-move perspective
- `<result>`: game outcome from white's perspective — `1` for white win, `0.5` for draw, `0` for black win

N/A for REST/HTTP.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| `cargo build -p datagen` | Binary compiles successfully |
| `cargo clippy -p datagen -- -D warnings` | No lint warnings |
| `cargo fmt --check -p datagen` | Formatting is correct |
| Run `datagen --openings tools/selfplay/openings.txt --games 2 --nodes 1000 --output /tmp/test_train.txt --maxmoves 100` | Tool runs and produces output file |
| Parse output file lines | Each line matches `<FEN> \| <eval> \| <result>` format; FEN is valid (parseable by `Position::from_fen`); eval is an integer; result is `1`, `0.5`, or `0` |
| Verify game count | After `--games 2`, output file contains positions from 2 games (non-zero number of records) |
| Verify result consistency | All records from the same game have the same result value |
| Verify FEN validity | Each FEN in the output is parseable by `Position::from_fen` without error |

## 6. Implementation Order

1. `Cargo.toml` (workspace root) — add `"tools/datagen"` to workspace members
2. `tools/datagen/Cargo.toml` — create the binary crate manifest
3. `tools/datagen/src/main.rs` — implement in this order:
   a. `Args` struct and `parse_args()` function
   b. `load_openings()` function
   c. Helper functions: `is_in_check`, `is_insufficient_material`, `count_repetitions`, `is_capture`
   d. `TrainingRecord` struct, `GameResult` enum, and `write_records()` function
   e. `play_game()` function with search integration and score capture
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

**Deviation: Internal search calls instead of UCI subprocess spawning**

- **Parent spec says**: "The self-play framework from M08 provides game-management infrastructure; this tool adds evaluation export on top"
- **Why direct approach is better**: The M08 selfplay tool spawns the engine as a UCI subprocess via `Command::new(path)` and communicates over stdin/stdout. This approach was appropriate for M08 because it needed to test the engine binary end-to-end (detecting crashes, illegal moves, assertion failures). For datagen, we need to capture the search score at each position, which requires either: (a) parsing UCI `info` lines from stdout to extract `score cp <N>`, or (b) calling `chess_search::search()` directly with a depth callback. Option (b) is simpler, more reliable, avoids the complexity of UCI parsing and subprocess management, and gives direct access to the search score via the `DepthCallback`. The datagen tool is not testing the engine binary — it is generating training data, so process isolation provides no benefit.
- **Alternative chosen**: Call `chess_search::search()` directly as a library function, using the `DepthCallback` to capture the score from the deepest completed search iteration. This is the same approach the engine binary itself uses internally.

**Deviation: Node limit instead of time control**

- **Parent spec says**: Configurable "time control"
- **Why node limit is more appropriate**: Time-based search produces non-deterministic results (different positions recorded depending on CPU speed and system load). Node limits produce consistent search depth regardless of hardware, which is standard practice for NNUE data generation. The `SearchLimits` struct already supports `max_nodes: Option<u64>`. The CLI parameter is named `--nodes` rather than `--movetime`. A very high `max_time` is set as a safety backstop.
- **Alternative chosen**: Use `--nodes` parameter for consistent, reproducible data generation. This is the industry-standard approach for NNUE datagen tools.