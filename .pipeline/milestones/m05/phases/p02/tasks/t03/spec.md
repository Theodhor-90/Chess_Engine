# T03 — Repetition Detection

## Objective

Implement repetition detection by maintaining a position history stack of Zobrist hashes. Detect threefold repetition and fifty-move rule draws during search, returning a draw score of 0. Update the engine binary to build the game history from UCI position moves and pass it to the search.

## Deliverables

- **Position history stack** (`Vec<u64>`) added to `SearchContext` in `crates/search/src/lib.rs`
- **Repetition detection** (threefold) and fifty-move rule check at the start of `negamax`, before the TT probe — returns draw score of 0 if detected
- **Updated `search` function signature** accepting `game_history: &[u64]` parameter containing Zobrist hashes from the UCI position setup
- **Updated `crates/engine/src/main.rs`** to build the game history `Vec<u64>` from the UCI `position` command's move list and pass it to `search`
- **Tests** verifying:
  - (a) Threefold repetition is detected and returns draw score
  - (b) Fifty-move rule is detected and returns draw score
  - (c) The engine avoids repeating positions when winning
  - (d) The engine seeks repetition when losing

## Files to Modify

- `crates/search/src/lib.rs` — Add position history stack to `SearchContext`, repetition/fifty-move detection in `negamax`, update `search` function signature
- `crates/engine/src/main.rs` — Build game history from UCI position moves and pass to `search`

## Key Implementation Details

- The history is initialized from the game's move history (provided via UCI `position` command with moves) before search begins
- During search, `make_move` pushes the new Zobrist hash and `unmake_move` pops it
- Repetition check scans backward from the current position, but only positions since the last irreversible move (capture or pawn move, indicated by halfmove clock reset) need checking — this bounds the scan to at most 100 entries rather than the entire game history
- Draw score is 0 (no contempt value at this stage)
- Threefold repetition: if the current hash appears 2+ times previously in the history, it's a threefold repetition
- Fifty-move rule: if the halfmove clock reaches 100, return draw score

## Dependencies

- **T01** (TT-Based Move Ordering): Must be in place so search correctly handles TT moves alongside repetition checks
- **T02** (IID): Must be in place so IID and repetition checks coexist correctly in `negamax`
- **M05/P01** (Transposition Table): TT probe/store, Zobrist hashing
- **M01** (Core Types): `Position` struct with halfmove clock, Zobrist hashing in `crates/board/src/zobrist.rs`
- **M04** (UCI Protocol): UCI command loop in `crates/engine/src/main.rs` that provides the position and move history

## Verification Criteria

1. Threefold repetition is correctly detected and returns a draw score during search
2. Fifty-move rule is correctly detected and returns a draw score during search
3. Engine avoids repeating positions when ahead in material
4. Engine seeks repetition when behind in material
5. All existing tests in `crates/search/` continue to pass
6. `cargo test` passes across the full workspace