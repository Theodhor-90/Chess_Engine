# T03 — UCI Integration

## Objective

Wire the Polyglot book consultation into the engine's UCI loop and expose configuration via UCI options, so that when a `go` command is received the engine first checks the book and, if a book move is found, emits `bestmove` immediately without searching.

## Deliverables

- **UCI `option` for book file path**: Type `string`, default empty (no book loaded). When set, the engine loads the specified `.bin` file into memory. Name: e.g., `OwnBook` or `BookFile`.
- **UCI `option` for book selection mode**: Type `combo`, values `bestmove` and `weighted`, default `bestmove`. Name: e.g., `BookMode`.
- **Engine loop modification**: When a `go` command is received:
  1. Compute the Polyglot hash for the current position.
  2. Look up book entries for the position.
  3. If entries are found, select a move using the configured mode and emit `bestmove <move>` immediately (no search).
  4. If no entries are found (or no book is loaded), fall through to normal search.
- **Edge case handling**:
  - No book loaded (book path is empty or not set): skip book lookup, proceed to search.
  - Book file not found at configured path: log a warning (or UCI `info string`), continue without book.
  - Position not in book: fall through to search silently.
- **Integration test**: Confirm that a known book position returns a book move as `bestmove` and that a non-book position falls through to search.

## Key Implementation Details

- The book file is loaded into memory when the UCI `setoption` command sets the book path. This aligns with the design decision to read the entire file on initialization.
- The book object (containing loaded entries) should be stored in the engine's state alongside existing state (e.g., position, search parameters).
- Output the selected book move in UCI algebraic notation (e.g., `bestmove e2e4`).
- Consider emitting `info string book move <move>` before `bestmove` for debugging/transparency.

## Files to Create or Modify

- Modify: `crates/uci/src/...` — add new UCI option definitions for book path and mode
- Modify: `crates/engine/src/...` — integrate book consultation into the engine's `go` handler, store book state
- Potentially modify: `crates/engine/src/main.rs` or the engine loop module — wire book lookup before search invocation

## Dependencies

- Depends on **T01** (Polyglot book format parser) — needs the book reader and Polyglot hash function.
- Depends on **T02** (Book move selection) — needs the selection interface and `BookMode` enum.
- Requires existing UCI infrastructure from **M04** (UCI Protocol milestone) — option parsing, engine loop, `bestmove` output.

## Verification Criteria

1. `cargo test` passes all new tests (unit and integration).
2. UCI `option` commands for book path and mode are listed in the engine's `uci` response.
3. Setting the book path via `setoption` loads the book file into memory.
4. A known starting-position book entry is correctly looked up and returned as `bestmove` without triggering search.
5. When no book is loaded or the position is not in the book, the engine falls through to normal search.
6. Invalid/missing book file path is handled gracefully (no crash, appropriate warning).
7. `cargo clippy -- -D warnings` and `cargo fmt --check` pass.