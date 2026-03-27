## Task T02 — UCI Output

### Objective

Add UCI output formatting functions/types to the `crates/uci/` crate that produce correctly formatted UCI response strings for `id`, `uciok`, `readyok`, `bestmove`, and `info` lines. This is the output half of the UCI protocol layer.

### Deliverables

- UCI output formatting functions or types added to `crates/uci/src/lib.rs` (or a submodule)
- Functions/types to format the following UCI responses:
  - `id name <engine_name>` and `id author <author_name>`
  - `uciok`
  - `readyok`
  - `bestmove <move> [ponder <move>]` — moves in UCI coordinate notation (e.g., `e2e4`, `a7a8q`)
  - `info` lines with fields: `depth`, `score cp <centipawns>` / `score mate <moves>`, `nodes`, `nps`, `pv <move1> <move2> ...`, `time <ms>`
- Unit tests verifying output strings match UCI specification format

### Implementation Details

- Output must conform to the UCI specification so standard GUIs (Arena, CuteChess) can parse it
- Moves in `bestmove` and `pv` use UCI coordinate notation via the existing `Move`'s `Display` implementation (format: `e2e4`, promotions like `a7a8q`)
- The `info` line should support any subset of fields — not all fields need to be present in every info output
- Consider using a builder pattern or struct for `info` line construction since it has many optional fields
- No I/O in the formatting functions — they produce `String` values. The engine binary (T03) handles actual stdout writing
- Maintain `thiserror` error handling, no `unwrap()` in library code

### Dependencies

- Depends on **T01** (this task adds to the `crates/uci/` crate created in T01)
- Depends on M01 types crate for `Move` type (UCI coordinate notation output)

### Verification Criteria

1. `cargo test -p uci` passes with unit tests covering:
   - `id name` and `id author` produce correct format
   - `uciok` and `readyok` produce exact strings
   - `bestmove` formats moves correctly in UCI notation
   - `bestmove` with optional ponder move
   - `info` lines with various field combinations (depth only, depth+score+nodes, full info line, etc.)
   - `info` score types: `score cp <n>` and `score mate <n>`
   - `pv` with multiple moves formatted correctly
2. `cargo clippy -p uci -- -D warnings` passes
3. `cargo fmt -p uci --check` passes