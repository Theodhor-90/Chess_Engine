Now I have complete context. Let me produce the revised plan.

## 1. Deliverables

| # | File | Action | Description |
|---|------|--------|-------------|
| 1 | `crates/engine/Cargo.toml` | Create | Binary crate manifest depending on `chess-uci`, `chess-search`, `chess-eval`, `chess-movegen`, `chess-board`, `chess-types`, and `anyhow` |
| 2 | `crates/engine/src/main.rs` | Create | Engine main loop binary: stdin/stdout UCI communication, command dispatch, search thread management, stop signal handling via `Arc<AtomicBool>` |
| 3 | `Cargo.toml` | Modify | Add `crates/engine` to workspace members (note: `crates/uci` is already listed) |
| 4 | `crates/search/src/lib.rs` | Modify | Extend `SearchContext` to accept an `Arc<AtomicBool>` stop flag and check it in `check_time`; update the `search` function signature to accept an optional stop flag and an optional per-depth callback |

## 2. Dependencies

| Dependency | Crate | What it provides |
|------------|-------|-----------------|
| `chess-uci` | `crates/uci` | `parse()` for UCI command parsing, `output::*` for UCI response formatting |
| `chess-search` | `crates/search` | `search()` function for iterative deepening search |
| `chess-eval` | `crates/eval` | (transitive via search, not directly used by engine binary) |
| `chess-movegen` | `crates/movegen` | `generate_legal_moves()` for resolving UCI move strings to `Move` objects |
| `chess-board` | `crates/board` | `Position` struct with `startpos()`, `from_fen()`, `make_move()` |
| `chess-types` | `crates/types` | `Move`, `Square`, `File`, `Rank`, `MoveFlag`, `Color`, `PieceKind` types |
| `anyhow` | external | Error handling in the binary crate |

No new external dependencies beyond `anyhow`. All internal crate dependencies already exist in the workspace.

## 3. Implementation Details

### 3.1 `crates/search/src/lib.rs` — Stop Flag Extension

**Purpose:** Allow the engine binary to pass an external `Arc<AtomicBool>` stop flag that the search checks alongside the existing time-budget check.

**Changes:**

1. Add `use std::sync::{Arc, atomic::{AtomicBool, Ordering}};` to imports.

2. Add a `stop_flag: Option<Arc<AtomicBool>>` field to `SearchContext`:
   ```rust
   pub struct SearchContext {
       start: Instant,
       time_budget: Duration,
       nodes: u64,
       aborted: bool,
       killers: KillerTable,
       pv_table: PvTable,
       prev_pv: Vec<Move>,
       stop_flag: Option<Arc<AtomicBool>>,
   }
   ```

3. Extend `check_time` to also check the stop flag:
   ```rust
   fn check_time(&mut self) {
       if self.start.elapsed() >= self.time_budget {
           self.aborted = true;
       }
       if let Some(ref flag) = self.stop_flag {
           if flag.load(Ordering::Relaxed) {
               self.aborted = true;
           }
       }
   }
   ```

4. Update the `search` function signature to accept an optional stop flag and an optional per-depth callback:
   ```rust
   pub fn search(
       pos: &mut Position,
       time_budget: Duration,
       stop_flag: Option<Arc<AtomicBool>>,
       on_depth: Option<&dyn Fn(u8, i32, u64, Duration, &[Move])>,
   ) -> Option<Move>
   ```

5. Pass the stop flag into the `SearchContext` constructor inside `search`:
   ```rust
   let mut ctx = SearchContext {
       start: Instant::now(),
       time_budget,
       nodes: 0,
       aborted: false,
       killers: KillerTable::new(),
       pv_table: PvTable::new(),
       prev_pv: Vec::new(),
       stop_flag,
   };
   ```

6. After each successful (non-aborted) iteration in the `search` loop, invoke the callback if provided. Insert this after the `ctx.prev_pv = ctx.pv_table.extract_pv();` line:
   ```rust
   if let Some(ref cb) = on_depth {
       cb(depth, score, ctx.nodes, ctx.start.elapsed(), &ctx.prev_pv);
   }
   ```

7. Update `test_ctx()` in the tests to include `stop_flag: None`, and update the two manual `SearchContext` constructions in `pv_ordering_reduces_nodes` to include `stop_flag: None`.

8. Update the two calls to `search()` in `iterative_deepening_returns_legal_move` and `iterative_deepening_finds_better_moves_at_depth` to pass `None, None` for the two new parameters. Similarly update `search_respects_time_budget`, `search_returns_none_for_checkmate`, and `search_returns_none_for_stalemate`.

### 3.2 `crates/engine/Cargo.toml` — Binary Crate Manifest

```toml
[package]
name = "engine"
version = "0.1.0"
edition.workspace = true

[[bin]]
name = "engine"
path = "src/main.rs"

[dependencies]
chess-uci = { path = "../uci" }
chess-search = { path = "../search" }
chess-movegen = { path = "../movegen" }
chess-board = { path = "../board" }
chess-types = { path = "../types" }
anyhow = "1"
```

### 3.3 `Cargo.toml` — Workspace Update

Add `"crates/engine"` to the `members` list. The existing list is `["crates/types", "crates/board", "crates/movegen", "crates/eval", "crates/search", "crates/uci", "tools/perft"]`. Append `"crates/engine"`.

### 3.4 `crates/engine/src/main.rs` — Engine Main Loop

**Purpose:** The top-level binary that reads UCI commands from stdin, dispatches them, manages a search thread, and writes UCI responses to stdout.

**Constants:**
```rust
const ENGINE_NAME: &str = "ChessEngine";
const ENGINE_AUTHOR: &str = "Theodhor";
```

**State:**
```rust
struct EngineState {
    position: Position,
    search_handle: Option<JoinHandle<Option<Move>>>,
    stop_flag: Arc<AtomicBool>,
}
```

**`main` function:**
1. Create `EngineState` with `Position::startpos()`, no search handle, and a fresh `Arc<AtomicBool>` set to `false`.
2. Read lines from `stdin` in a loop using `BufRead::lines()`.
3. For each line, parse it with `chess_uci::parse()`. Silently ignore parse errors (standard UCI practice — GUIs may send unknown commands).
4. Match on `UciCommand` variants and dispatch.

**Command handlers:**

- **`Uci`**: Print `id name ChessEngine`, `id author Theodhor`, `uciok` (using `chess_uci::output::id_name()`, `id_author()`, `uciok()`). Each followed by `println!`.

- **`IsReady`**: Print `readyok` (using `chess_uci::output::readyok()`). If a search is running, wait for it to finish first to ensure the engine is truly ready.

- **`UciNewGame`**: Reset `position` to `Position::startpos()`. If a search is running, stop it first.

- **`Position { fen, moves }`**: Set up the position:
  1. If `fen` is `Some(fen_str)`, call `Position::from_fen(&fen_str)`. If it fails, ignore the command.
  2. If `fen` is `None`, use `Position::startpos()`.
  3. For each move string in `moves`, resolve it to a `Move` by generating legal moves and matching the UCI string representation. Apply via `pos.make_move(mv)` (discard the `UndoInfo` — the engine doesn't need to unmake).

  **UCI move string resolution:** Parse the move string (e.g., `"e2e4"`, `"a7a8q"`) into from/to squares and optional promotion piece. Then find the matching legal move from `generate_legal_moves()`:
  ```rust
  fn parse_uci_move(pos: &mut Position, move_str: &str) -> Option<Move> {
      let bytes = move_str.as_bytes();
      if bytes.len() < 4 || bytes.len() > 5 {
          return None;
      }
      let from_file = bytes[0].wrapping_sub(b'a');
      let from_rank = bytes[1].wrapping_sub(b'1');
      let to_file = bytes[2].wrapping_sub(b'a');
      let to_rank = bytes[3].wrapping_sub(b'1');
      let from = Square::from_file_rank(File::try_from(from_file).ok()?, Rank::try_from(from_rank).ok()?);
      let to = Square::from_file_rank(File::try_from(to_file).ok()?, Rank::try_from(to_rank).ok()?);
      let promo_char = bytes.get(4).copied();

      let legal_moves = chess_movegen::generate_legal_moves(pos);
      legal_moves.into_iter().find(|mv| {
          mv.from_sq() == from && mv.to_sq() == to && match promo_char {
              Some(b'q') => mv.promotion_piece() == Some(PieceKind::Queen),
              Some(b'r') => mv.promotion_piece() == Some(PieceKind::Rook),
              Some(b'b') => mv.promotion_piece() == Some(PieceKind::Bishop),
              Some(b'n') => mv.promotion_piece() == Some(PieceKind::Knight),
              _ => !mv.is_promotion(),
          }
      })
  }
  ```

- **`Go(params)`**:
  1. If a search is already running, stop it first (set stop flag, join thread).
  2. Reset the stop flag to `false`.
  3. Determine the time budget:
     - If `params.movetime` is set: use `Duration::from_millis(params.movetime)`.
     - If `params.infinite` is set or `params.depth` is set: use `Duration::from_secs(86400)` (24 hours — effectively infinite, relies on `stop` to halt). Depth limiting is not yet supported; it is deferred to P02 T03.
     - If time params (`wtime`/`btime`) are set: for P01, use a fixed default of 5 seconds. Full time management comes in P02.
     - Otherwise (bare `go` with no params): use `Duration::from_secs(86400)` (effectively infinite, relies on `stop`).
  4. Clone the position for the search thread. Clone the `Arc<AtomicBool>`.
  5. Spawn a thread that calls `chess_search::search()` with the position, time budget, stop flag, and an `on_depth` callback that prints `info` lines to stdout.
  6. The thread prints `bestmove` when search completes. Store the `JoinHandle`.

  **Thread implementation:**
  ```rust
  let mut search_pos = state.position.clone();
  let stop = Arc::clone(&state.stop_flag);
  state.search_handle = Some(std::thread::spawn(move || {
      let result = chess_search::search(
          &mut search_pos,
          time_budget,
          Some(stop),
          Some(&|depth, score, nodes, elapsed, pv| {
              let time_ms = elapsed.as_millis() as u64;
              let nps = if time_ms > 0 { nodes * 1000 / time_ms } else { 0 };
              let uci_score = score_to_uci(score);
              let info = chess_uci::output::InfoFields::new()
                  .depth(depth as u32)
                  .score(uci_score)
                  .nodes(nodes)
                  .nps(nps)
                  .time(time_ms)
                  .pv(pv.to_vec());
              if let Some(line) = info.to_uci_string() {
                  println!("{line}");
              }
          }),
      );
      if let Some(mv) = result {
          println!("{}", chess_uci::output::bestmove(mv, None));
      } else {
          // Stalemate/checkmate — output a null move (UCI spec allows "bestmove 0000")
          println!("bestmove 0000");
      }
      result
  }));
  ```

  **Score conversion (`score_to_uci` helper):** The search returns an `i32` score. The `MATE_SCORE` constant in the search crate is `30000`. Scores near mate use the pattern `MATE_SCORE - ply` for winning and `-(MATE_SCORE - ply)` for losing. The conversion logic:
  ```rust
  fn score_to_uci(score: i32) -> chess_uci::output::Score {
      if score > chess_search::MATE_SCORE - 100 {
          // Engine is winning — mate in N moves.
          // score = MATE_SCORE - ply, so ply = MATE_SCORE - score.
          // Moves = ceil(ply / 2) = (ply + 1) / 2.
          let ply = chess_search::MATE_SCORE - score;
          chess_uci::output::Score::Mate(((ply + 1) / 2) as i32)
      } else if score < -(chess_search::MATE_SCORE - 100) {
          // Engine is losing — being mated in N moves.
          // score = -(MATE_SCORE - ply), so ply = MATE_SCORE + score.
          // Moves = ceil(ply / 2) = (ply + 1) / 2, negated for losing side.
          let ply = chess_search::MATE_SCORE + score;
          chess_uci::output::Score::Mate(-(((ply + 1) / 2) as i32))
      } else {
          chess_uci::output::Score::Cp(score)
      }
  }
  ```

- **`Stop`**: Set `stop_flag` to `true`. Join the search thread (wait for it to finish, which will print `bestmove`). Set `search_handle` to `None`.

- **`Quit`**: Set `stop_flag` to `true`. Join the search thread if running. Exit the process.

**Stdout flushing:** After each `println!`, stdout is auto-flushed (line-buffered when connected to a pipe). However, to be safe, explicitly flush after each write using `io::stdout().flush().ok()` or wrap stdout in a `BufWriter` and flush manually. Actually, `println!` is line-buffered when not a terminal, so explicit flushing may be needed. Use `use std::io::Write;` and call `std::io::stdout().flush().ok();` after critical outputs.

## 4. API Contracts

### Search crate — updated `search` signature

```rust
pub fn search(
    pos: &mut Position,
    time_budget: Duration,
    stop_flag: Option<Arc<AtomicBool>>,
    on_depth: Option<&dyn Fn(u8, i32, u64, Duration, &[Move])>,
) -> Option<Move>
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `pos` | `&mut Position` | Position to search |
| `time_budget` | `Duration` | Maximum search time |
| `stop_flag` | `Option<Arc<AtomicBool>>` | External stop signal; if `Some`, checked every 1024 nodes alongside time budget |
| `on_depth` | `Option<&dyn Fn(u8, i32, u64, Duration, &[Move])>` | Callback after each completed depth: `(depth, score, nodes, elapsed, pv)` |
| **return** | `Option<Move>` | Best move found, or `None` for checkmate/stalemate |

### Engine binary — UCI protocol contract

| Input | Output |
|-------|--------|
| `uci` | `id name ChessEngine\nid author Theodhor\nuciok` |
| `isready` | `readyok` |
| `ucinewgame` | (no output, resets state) |
| `position startpos` | (no output, sets position) |
| `position startpos moves e2e4 e7e5` | (no output, sets position and applies moves) |
| `position fen <fen> [moves ...]` | (no output, sets position from FEN) |
| `go depth 5` | `info depth 1 score cp ... nodes ... nps ... time ... pv ...\n...\nbestmove e2e4` (note: depth limit not enforced in P01; searches until stopped) |
| `go movetime 1000` | (info lines + bestmove after ~1 second) |
| `go infinite` | (info lines until `stop`) |
| `stop` | (search stops, `bestmove` emitted if not already) |
| `quit` | (process exits) |

## 5. Test Plan

### 5.1 Search crate tests — existing tests updated

All existing tests in `crates/search/src/lib.rs` must continue to pass with the new `stop_flag` parameter set to `None` and `on_depth` set to `None`. No behavioral change for existing callers.

### 5.2 Search crate tests — new stop flag test

A new test is added to verify that setting the stop flag to `true` causes the search to abort promptly and return a result:

| # | Test name | Verifies |
|---|-----------|----------|
| 1 | `test_stop_flag_aborts_search` | Create an `Arc<AtomicBool>` set to `false`. Start `search()` on the start position with a 60-second time budget and the stop flag. From a separate thread, set the flag to `true` after 50ms. Assert that `search()` returns within 500ms (well before the 60s budget) and returns `Some(move)` (the start position has legal moves, and at least depth 1 should complete in <50ms). |

Implementation:
```rust
#[test]
fn test_stop_flag_aborts_search() {
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        stop_clone.store(true, Ordering::Relaxed);
    });

    let mut pos = Position::startpos();
    let start = Instant::now();
    let mv = search(&mut pos, Duration::from_secs(60), Some(stop), None);
    let elapsed = start.elapsed();

    assert!(elapsed < Duration::from_millis(500), "search should abort within 500ms, took {:?}", elapsed);
    assert!(mv.is_some(), "search should find at least one move before being stopped");
}
```

### 5.3 Test modifications

| # | Test | Change |
|---|------|--------|
| 1 | All existing `search()` calls in `crates/search/src/lib.rs` tests | Update call sites to pass `None, None` for the two new parameters |
| 2 | `test_ctx()` helper in search tests | Add `stop_flag: None` to `SearchContext` initialization |
| 3 | `pv_ordering_reduces_nodes` test | Add `stop_flag: None` to the two `SearchContext` manual initializations |

Specifically, the five `search()` calls in these tests must be updated:
- `iterative_deepening_returns_legal_move`
- `iterative_deepening_finds_better_moves_at_depth`
- `search_respects_time_budget`
- `search_returns_none_for_checkmate`
- `search_returns_none_for_stalemate`

### 5.4 Engine binary verification (manual / integration)

The engine binary is verified via the verification commands below (compilation, linting, workspace tests). The task spec's verification criteria #10 specifies manual verification in CuteChess/Arena. No automated integration tests are added for the binary in this task.

## 6. Implementation Order

| Step | Action | Details |
|------|--------|---------|
| 1 | Modify `crates/search/src/lib.rs` | Add `stop_flag` field to `SearchContext`, extend `check_time`, update `search` signature to accept `stop_flag` and `on_depth`, add the `on_depth` callback invocation after each completed depth, update all test call sites, add `test_stop_flag_aborts_search` test |
| 2 | Create `crates/engine/Cargo.toml` | New binary crate manifest |
| 3 | Create `crates/engine/src/main.rs` | Engine main loop with all command handlers, `parse_uci_move` function, `score_to_uci` function |
| 4 | Modify `Cargo.toml` | Add `"crates/engine"` to workspace members |
| 5 | Run verification commands | `cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `cargo build -p engine`, UCI handshake test |

## 7. Verification Commands

```bash
# Format check
cargo fmt --all --check

# Lint check
cargo clippy --workspace -- -D warnings

# Run all workspace tests (including updated search crate tests and new stop flag test)
cargo test --workspace

# Build the engine binary specifically
cargo build -p engine

# Verify UCI handshake (send uci + quit, check output)
echo -e "uci\nquit" | cargo run -p engine
```

## 8. Design Deviations

**Deviation 1: `on_depth` callback parameter on `search()`**

- **What the parent spec requires:** The task spec says "emits `info` lines during search" and "emits `bestmove` when search completes or is stopped." It does not prescribe a specific mechanism for the search to communicate per-depth results to the engine loop.
- **Why this is needed:** The existing `search()` function returns only `Option<Move>` and prints nothing. The engine needs per-depth info (depth, score, nodes, time, PV) to emit `info` lines as UCI requires. Without a callback, the engine would have no way to emit `info` lines during search.
- **Alternative chosen:** Add an `on_depth: Option<&dyn Fn(u8, i32, u64, Duration, &[Move])>` parameter to `search()`. When `None` (existing callers), behavior is unchanged. When `Some`, the callback is invoked after each completed depth with the iteration's results. This is the minimal change that enables UCI info output without restructuring the search architecture.