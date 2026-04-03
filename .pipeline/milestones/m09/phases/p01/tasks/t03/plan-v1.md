Now I have everything I need. Here is the complete implementation plan:

## 1. Deliverables

- **Modify: `crates/uci/src/lib.rs`** — Add `SetOption` variant to `UciCommand` enum with `name` and `value` fields. Add `setoption` parsing in the `parse()` dispatcher. Add `option` output function support for the `uci` response. Add new error variant for invalid setoption commands.
- **Modify: `crates/uci/src/output.rs`** — Add `option_string()` and `option_combo()` output functions for emitting UCI option declarations during the `uci` handshake. Add `info_string()` helper for emitting `info string` messages.
- **Modify: `crates/engine/src/main.rs`** — Add `book: Option<PolyglotBook>` and `book_mode: BookMode` fields to `EngineState`. Handle `SetOption` command to load the book file and set the book mode. In the `Go` handler, consult the book before launching search — if a book move is found, emit `bestmove` immediately. Emit `option` lines for `BookFile` and `BookMode` during the `Uci` handshake.
- **Modify: `crates/engine/Cargo.toml`** — Add `chess_engine` library crate as a dependency (the engine binary needs to use `chess_engine::book`). *(Note: the binary is in the same crate as the library, so the book module is already accessible via `crate::book`. No Cargo.toml change needed for this.)*

## 2. Dependencies

- No new external crate dependencies are required.
- Existing crates used: `chess_engine::book` (from `crates/engine/src/book/mod.rs`, already available within the same crate), `chess-uci` (already a dependency of the engine binary), `chess-board` (already a dependency).
- External: A Polyglot `.bin` opening book file is needed for integration testing. Tests that require a real book file will be conditional or use synthetic in-memory book data.

## 3. Implementation Details

### 3.1 `crates/uci/src/lib.rs` — `SetOption` parsing

**Purpose**: Enable the UCI parser to recognize `setoption name <name> [value <value>]` commands.

**Changes**:

1. Add a new variant to `UciCommand`:
   ```rust
   SetOption { name: String, value: Option<String> },
   ```

2. Add a `parse_setoption` function that:
   - Finds the `name` keyword and collects all tokens until `value` keyword (or end) as the option name (joined with spaces, since UCI option names can be multi-word).
   - If `value` keyword is present, collects remaining tokens as the value (joined with spaces).
   - Returns `UciCommand::SetOption { name, value }`.

3. Add `"setoption"` branch in the `parse()` match that calls `parse_setoption(&tokens[1..])`.

4. Add a new error variant:
   ```rust
   #[error("invalid setoption command: {0}")]
   InvalidSetOption(String),
   ```

**Exports**: `UciCommand::SetOption` variant (already public via enum).

### 3.2 `crates/uci/src/output.rs` — UCI option output functions

**Purpose**: Provide functions to emit UCI `option` lines during the `uci` handshake and `info string` messages.

**Changes**:

1. Add `option_string(name, default)` function:
   ```rust
   pub fn option_string(name: &str, default: &str) -> String
   ```
   Returns `"option name <name> type string default <default>"`.

2. Add `option_combo(name, default, vars)` function:
   ```rust
   pub fn option_combo(name: &str, default: &str, vars: &[&str]) -> String
   ```
   Returns `"option name <name> type combo default <default> var <v1> var <v2> ..."`.

3. Add `info_string(msg)` function:
   ```rust
   pub fn info_string(msg: &str) -> String
   ```
   Returns `"info string <msg>"`.

### 3.3 `crates/engine/src/main.rs` — Book integration into engine loop

**Purpose**: Store book state in the engine, handle `setoption` to load/configure the book, and consult the book before searching on `go`.

**Changes to `EngineState`**:
```rust
struct EngineState {
    position: Position,
    game_history: Vec<u64>,
    search_handle: Option<JoinHandle<Option<Move>>>,
    stop_flag: Arc<AtomicBool>,
    pondering: bool,
    ponder_params: Option<chess_uci::GoParams>,
    book: Option<crate::book::PolyglotBook>,
    book_mode: crate::book::BookMode,
}
```

**Initialization**: `book: None`, `book_mode: BookMode::BestMove` (the default per spec).

**`Uci` command handler** — After the `id` lines, emit:
```
println!("{}", chess_uci::output::option_string("BookFile", ""));
println!("{}", chess_uci::output::option_combo("BookMode", "bestmove", &["bestmove", "weighted"]));
```

**`SetOption` handler** — New match arm:
```rust
chess_uci::UciCommand::SetOption { name, value } => {
    match name.to_lowercase().as_str() {
        "bookfile" => {
            if let Some(path) = value {
                if path.is_empty() {
                    state.book = None;
                } else {
                    match crate::book::PolyglotBook::from_file(&path) {
                        Ok(b) => state.book = Some(b),
                        Err(e) => {
                            println!("{}", chess_uci::output::info_string(
                                &format!("failed to load book: {e}")
                            ));
                            io::stdout().flush().ok();
                        }
                    }
                }
            } else {
                state.book = None;
            }
        }
        "bookmode" => {
            if let Some(val) = value {
                match val.to_lowercase().as_str() {
                    "bestmove" => state.book_mode = crate::book::BookMode::BestMove,
                    "weighted" => state.book_mode = crate::book::BookMode::Weighted,
                    _ => {}
                }
            }
        }
        _ => {}
    }
}
```

**`Go` command handler** — Before launching the search thread, insert book consultation:
```rust
chess_uci::UciCommand::Go(params) => {
    stop_search(&mut state);

    // Consult book before searching
    if let Some(ref book) = state.book {
        let key = crate::book::polyglot_hash(&state.position);
        let entries = book.probe(key);
        if !entries.is_empty() {
            if let Some(mv) = crate::book::select_book_move(
                &mut state.position,
                entries,
                state.book_mode,
            ) {
                println!("{}", chess_uci::output::info_string(
                    &format!("book move {mv}")
                ));
                println!("{}", chess_uci::output::bestmove(mv, None));
                io::stdout().flush().ok();
                continue; // skip search
            }
        }
    }

    // ... existing search code ...
}
```

The `continue` statement skips the rest of the Go handler (the search), falling back to the next iteration of the main stdin loop.

## 4. API Contracts

### UCI `setoption` parsing (in `crates/uci/src/lib.rs`)

**Input**: `"setoption name BookFile value /path/to/book.bin"`
**Output**: `UciCommand::SetOption { name: "BookFile".to_string(), value: Some("/path/to/book.bin".to_string()) }`

**Input**: `"setoption name BookMode value weighted"`
**Output**: `UciCommand::SetOption { name: "BookMode".to_string(), value: Some("weighted".to_string()) }`

**Input**: `"setoption name BookFile"`
**Output**: `UciCommand::SetOption { name: "BookFile".to_string(), value: None }`

### UCI option output functions (in `crates/uci/src/output.rs`)

**`option_string("BookFile", "")`** → `"option name BookFile type string default "`
**`option_combo("BookMode", "bestmove", &["bestmove", "weighted"])`** → `"option name BookMode type combo default bestmove var bestmove var weighted"`
**`info_string("book move e2e4")`** → `"info string book move e2e4"`

## 5. Test Plan

### 5.1 Unit tests for `setoption` parsing (`crates/uci/src/lib.rs`)

- **`parse_setoption_name_and_value`**: Parse `"setoption name BookFile value /path/to/book.bin"` → `SetOption { name: "BookFile", value: Some("/path/to/book.bin") }`.
- **`parse_setoption_name_only`**: Parse `"setoption name BookFile"` → `SetOption { name: "BookFile", value: None }`.
- **`parse_setoption_combo_value`**: Parse `"setoption name BookMode value weighted"` → `SetOption { name: "BookMode", value: Some("weighted") }`.
- **`parse_setoption_missing_name`**: Parse `"setoption"` → returns error.

### 5.2 Unit tests for UCI option output (`crates/uci/src/output.rs`)

- **`test_option_string`**: Verify `option_string("BookFile", "")` returns `"option name BookFile type string default "`.
- **`test_option_combo`**: Verify `option_combo("BookMode", "bestmove", &["bestmove", "weighted"])` returns `"option name BookMode type combo default bestmove var bestmove var weighted"`.
- **`test_info_string`**: Verify `info_string("book move e2e4")` returns `"info string book move e2e4"`.

### 5.3 Integration test for book consultation in engine loop

The task spec calls for an integration test confirming that a known book position returns a book move and that a non-book position falls through to search. Since the engine loop reads from stdin and writes to stdout, this is best validated via the verification commands (section 7) using a subprocess or manual protocol test. The unit tests above, combined with the existing book module tests from t01/t02, provide coverage of the individual components; the integration between them is verified by the protocol-level test in section 7.

## 6. Implementation Order

1. **`crates/uci/src/lib.rs`** — Add `InvalidSetOption` error variant, `SetOption` command variant, and `parse_setoption` function. Add unit tests.
2. **`crates/uci/src/output.rs`** — Add `option_string`, `option_combo`, and `info_string` functions. Add unit tests.
3. **`crates/engine/src/main.rs`** — Add `book` and `book_mode` fields to `EngineState`. Add `SetOption` handler. Add book consultation before search in `Go` handler. Emit option declarations in `Uci` handler.
4. **Run verification commands** — `cargo fmt`, `cargo clippy`, `cargo test`.

## 7. Verification Commands

```bash
# Format check
cargo fmt --check

# Lint check
cargo clippy -- -D warnings

# Run all tests (including new UCI parser tests and existing book tests)
cargo test

# Quick protocol smoke test: verify options appear in uci handshake
echo "uci" | cargo run --bin engine 2>/dev/null | grep -E "option name (BookFile|BookMode)"

# Verify setoption parsing doesn't crash on invalid book path
printf "uci\nsetoption name BookFile value /nonexistent/path.bin\nisready\nquit\n" | cargo run --bin engine 2>/dev/null | grep -E "(readyok|info string)"
```

## 8. Design Deviations

**Deviation 1: `setoption` parsing implemented in UCI crate, not as free-form handling in engine.**

- The task spec says "Modify: `crates/uci/src/...` — add new UCI option definitions for book path and mode". It also lists modifying the engine crate.
- The UCI crate currently has no `setoption` command support at all — there is no `SetOption` variant in `UciCommand`. The parser returns `UnknownCommand` for `setoption`.
- Rather than handling `setoption` as a raw string in the engine, the correct approach is to add proper `SetOption` parsing to the UCI crate (where all other command parsing lives) and then handle the parsed command in the engine. This follows the existing architectural pattern where all UCI protocol parsing is centralized in `crates/uci` and the engine binary only handles parsed commands.

**Deviation 2: Option names `BookFile` and `BookMode` instead of `OwnBook`.**

- The task spec says option names are "e.g., `OwnBook` or `BookFile`" and "e.g., `BookMode`" — these are given as examples, not mandates.
- `BookFile` is chosen for the path option because it clearly describes the option's purpose (a file path), whereas `OwnBook` typically means a boolean "use own book" toggle in UCI convention. `BookMode` is used as suggested.