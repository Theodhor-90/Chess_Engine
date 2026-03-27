Now I have a thorough understanding of the codebase. Let me produce the task plan.

## 1. Deliverables

- **`crates/uci/Cargo.toml`** — New crate manifest for the UCI library crate, depending on `thiserror` only.
- **`crates/uci/src/lib.rs`** — UCI command parser module: `UciCommand` enum, `GoParams` struct, `UciError` error type, and `parse` function. Includes unit tests.

## 2. Dependencies

- `thiserror = "2"` — already used by other crates in the workspace (e.g., `chess-types`). Required for `UciError`.
- No dependency on other workspace crates. The task spec notes the parser may store moves as raw strings and defer parsing to the engine layer, so no dependency on `chess-types` is needed.
- No new external dependencies beyond `thiserror`.

## 3. Implementation Details

### `crates/uci/Cargo.toml`

```toml
[package]
name = "chess-uci"
version = "0.1.0"
edition.workspace = true

[dependencies]
thiserror = "2"
```

Follows the naming convention of other crates (`chess-types`, `chess-board`, `chess-search`, etc.). Uses `edition.workspace = true` per existing pattern.

### `crates/uci/src/lib.rs`

#### Error type — `UciError`

```rust
#[derive(Debug, thiserror::Error)]
pub enum UciError {
    #[error("empty input")]
    EmptyInput,
    #[error("unknown command: '{0}'")]
    UnknownCommand(String),
    #[error("invalid position command: {0}")]
    InvalidPosition(String),
    #[error("invalid go parameter: {0}")]
    InvalidGoParam(String),
}
```

Follows the `thiserror` pattern used by `FenError` in `crates/board/`.

#### `GoParams` struct

```rust
#[derive(Debug, Default, PartialEq, Eq)]
pub struct GoParams {
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
    pub movestogo: Option<u32>,
    pub depth: Option<u8>,
    pub nodes: Option<u64>,
    pub movetime: Option<u64>,
    pub infinite: bool,
    pub ponder: bool,
}
```

All time values in milliseconds as `u64` (matching UCI specification). `movestogo` as `u32`. `depth` as `u8` (consistent with the search crate's depth type). `infinite` and `ponder` as `bool`, defaulting to `false`.

#### `UciCommand` enum

```rust
#[derive(Debug, PartialEq, Eq)]
pub enum UciCommand {
    Uci,
    IsReady,
    UciNewGame,
    Position {
        fen: Option<String>,
        moves: Vec<String>,
    },
    Go(GoParams),
    Stop,
    Quit,
}
```

- `Position::fen` is `None` for `startpos`, `Some(String)` for an explicit FEN.
- `Position::moves` is a `Vec<String>` of move strings in UCI coordinate notation (e.g., `"e2e4"`, `"a7a8q"`). Empty if no moves keyword present. Moves are stored as raw strings — parsing into `Move` values is deferred to the engine layer (as noted in the task spec's dependencies section).

#### `parse` function

```rust
pub fn parse(input: &str) -> Result<UciCommand, UciError>
```

Logic:
1. Trim the input, split on whitespace, get the first token as the command keyword.
2. Return `Err(UciError::EmptyInput)` if the input is empty or all whitespace.
3. Match keyword:
   - `"uci"` → `Ok(UciCommand::Uci)`
   - `"isready"` → `Ok(UciCommand::IsReady)`
   - `"ucinewgame"` → `Ok(UciCommand::UciNewGame)`
   - `"stop"` → `Ok(UciCommand::Stop)`
   - `"quit"` → `Ok(UciCommand::Quit)`
   - `"position"` → call `parse_position(tokens)` helper
   - `"go"` → call `parse_go(tokens)` helper
   - other → `Err(UciError::UnknownCommand(keyword.to_string()))`

#### `parse_position` internal helper

Takes the remaining tokens after `"position"`.

- If first token is `"startpos"`: set `fen = None`, advance past it.
- If first token is `"fen"`: collect the next 6 tokens as the FEN string (joined with spaces), set `fen = Some(fen_string)`. Return `Err(UciError::InvalidPosition)` if fewer than 6 tokens available.
- Otherwise: return `Err(UciError::InvalidPosition)`.
- If a `"moves"` token follows: collect all remaining tokens as the move list.
- Return `UciCommand::Position { fen, moves }`.

#### `parse_go` internal helper

Takes the remaining tokens after `"go"`.

- Initialize a `GoParams::default()`.
- Iterate through tokens as key-value pairs:
  - `"wtime"` → parse next token as `u64`, set `wtime`
  - `"btime"` → parse next token as `u64`, set `btime`
  - `"winc"` → parse next token as `u64`, set `winc`
  - `"binc"` → parse next token as `u64`, set `binc`
  - `"movestogo"` → parse next token as `u32`, set `movestogo`
  - `"depth"` → parse next token as `u8`, set `depth`
  - `"nodes"` → parse next token as `u64`, set `nodes`
  - `"movetime"` → parse next token as `u64`, set `movetime`
  - `"infinite"` → set `infinite = true` (no value token)
  - `"ponder"` → set `ponder = true` (no value token)
  - Unknown key → return `Err(UciError::InvalidGoParam)`
- If a value token is missing or fails to parse, return `Err(UciError::InvalidGoParam)`.
- Return `Ok(UciCommand::Go(params))`.

#### Unit tests (colocated in `#[cfg(test)] mod tests`)

Tests cover all verification criteria from the task spec:

| Test name | What it verifies |
|---|---|
| `parse_uci` | `"uci"` → `UciCommand::Uci` |
| `parse_isready` | `"isready"` → `UciCommand::IsReady` |
| `parse_ucinewgame` | `"ucinewgame"` → `UciCommand::UciNewGame` |
| `parse_stop` | `"stop"` → `UciCommand::Stop` |
| `parse_quit` | `"quit"` → `UciCommand::Quit` |
| `parse_position_startpos` | `"position startpos"` → `Position { fen: None, moves: [] }` |
| `parse_position_startpos_with_moves` | `"position startpos moves e2e4 e7e5"` → `Position { fen: None, moves: ["e2e4", "e7e5"] }` |
| `parse_position_fen` | `"position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"` → `Position { fen: Some("rnbqkbnr/..."), moves: [] }` |
| `parse_position_fen_with_moves` | `"position fen <fen> moves e2e4 d7d5"` → correct fen and moves parsed |
| `parse_go_all_params` | `"go wtime 300000 btime 300000 winc 2000 binc 2000 movestogo 40"` → all fields set |
| `parse_go_some_params` | `"go wtime 60000 btime 60000"` → only wtime/btime set, rest default |
| `parse_go_no_params` | `"go"` → `GoParams::default()` (all `None`/`false`) |
| `parse_go_infinite` | `"go infinite"` → `GoParams { infinite: true, .. }` |
| `parse_go_depth` | `"go depth 5"` → `GoParams { depth: Some(5), .. }` |
| `parse_go_movetime` | `"go movetime 1000"` → `GoParams { movetime: Some(1000), .. }` |
| `parse_go_ponder` | `"go ponder"` → `GoParams { ponder: true, .. }` |
| `parse_empty_input` | `""` → `Err(UciError::EmptyInput)` |
| `parse_whitespace_only` | `"   "` → `Err(UciError::EmptyInput)` |
| `parse_unknown_command` | `"invalid"` → `Err(UciError::UnknownCommand("invalid"))` |
| `parse_position_missing_args` | `"position"` → `Err(UciError::InvalidPosition)` |
| `parse_position_invalid_keyword` | `"position badarg"` → `Err(UciError::InvalidPosition)` |
| `parse_go_invalid_value` | `"go depth abc"` → `Err(UciError::InvalidGoParam)` |

## 4. API Contracts

```rust
use chess_uci::{parse, UciCommand, GoParams, UciError};

// Simple commands
let cmd = parse("uci")?;           // Ok(UciCommand::Uci)
let cmd = parse("isready")?;       // Ok(UciCommand::IsReady)
let cmd = parse("stop")?;          // Ok(UciCommand::Stop)

// Position with startpos and moves
let cmd = parse("position startpos moves e2e4 e7e5")?;
// Ok(UciCommand::Position { fen: None, moves: vec!["e2e4".into(), "e7e5".into()] })

// Position with FEN
let cmd = parse("position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")?;
// Ok(UciCommand::Position { fen: Some("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".into()), moves: vec![] })

// Go with time parameters
let cmd = parse("go wtime 300000 btime 300000 winc 2000 binc 2000")?;
// Ok(UciCommand::Go(GoParams { wtime: Some(300000), btime: Some(300000), winc: Some(2000), binc: Some(2000), ..Default::default() }))

// Error cases
let err = parse("");                // Err(UciError::EmptyInput)
let err = parse("badcmd");          // Err(UciError::UnknownCommand("badcmd".into()))
```

N/A for REST/HTTP.

## 5. Test Plan

All tests are unit tests colocated in `crates/uci/src/lib.rs` under `#[cfg(test)] mod tests`, following the project convention.

| Test | What it verifies |
|------|-----------------|
| `parse_uci` | `"uci"` parses to `UciCommand::Uci` |
| `parse_isready` | `"isready"` parses to `UciCommand::IsReady` |
| `parse_ucinewgame` | `"ucinewgame"` parses to `UciCommand::UciNewGame` |
| `parse_stop` | `"stop"` parses to `UciCommand::Stop` |
| `parse_quit` | `"quit"` parses to `UciCommand::Quit` |
| `parse_position_startpos` | `"position startpos"` parses with `fen: None, moves: []` |
| `parse_position_startpos_with_moves` | `"position startpos moves e2e4 e7e5"` parses with correct move list |
| `parse_position_fen` | `"position fen <6-field FEN>"` parses with correct fen string |
| `parse_position_fen_with_moves` | `"position fen <fen> moves e2e4 d7d5"` parses both fen and moves |
| `parse_go_all_params` | `go` with all numeric params sets all fields correctly |
| `parse_go_some_params` | `go` with partial params sets only those fields |
| `parse_go_no_params` | `"go"` alone produces `GoParams::default()` |
| `parse_go_infinite` | `"go infinite"` sets `infinite: true` |
| `parse_go_depth` | `"go depth 5"` sets `depth: Some(5)` |
| `parse_go_movetime` | `"go movetime 1000"` sets `movetime: Some(1000)` |
| `parse_go_ponder` | `"go ponder"` sets `ponder: true` |
| `parse_empty_input` | Empty string returns `Err(UciError::EmptyInput)` |
| `parse_whitespace_only` | Whitespace-only input returns `Err(UciError::EmptyInput)` |
| `parse_unknown_command` | Unknown command returns `Err(UciError::UnknownCommand)` |
| `parse_position_missing_args` | `"position"` with no subcommand returns `Err(UciError::InvalidPosition)` |
| `parse_position_invalid_keyword` | `"position badarg"` returns `Err(UciError::InvalidPosition)` |
| `parse_go_invalid_value` | `"go depth abc"` returns `Err(UciError::InvalidGoParam)` |

## 6. Implementation Order

1. **`crates/uci/Cargo.toml`** — create the crate manifest
2. **`crates/uci/src/lib.rs`** — implement in this order within the file:
   1. `UciError` enum with `thiserror` derive
   2. `GoParams` struct with `Default` derive
   3. `UciCommand` enum
   4. `parse_position` internal helper function
   5. `parse_go` internal helper function
   6. `parse` public function
   7. `#[cfg(test)] mod tests` with all unit tests

## 7. Verification Commands

```bash
cargo test -p chess-uci
cargo clippy -p chess-uci -- -D warnings
cargo fmt -p chess-uci --check
```

## 8. Design Deviations

None.