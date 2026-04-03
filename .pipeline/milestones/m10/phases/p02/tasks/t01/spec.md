## Task T01 — Training Data Generation

### Objective

Build a self-play data generation tool that uses the handcrafted chess engine to produce training data for NNUE network training. The tool plays games against itself using randomized openings and exports position data in a format compatible with external training tooling (`bullet` or `nnue-pytorch`).

### Deliverables

- A binary in `tools/datagen` (or equivalent location under `tools/`) that generates training data files
- Each record in the output contains: FEN string, handcrafted evaluation score (centipawns), and game result (win/draw/loss from side-to-move perspective)
- Output format must be compatible with both `bullet` and `nnue-pytorch` training pipelines
- Configurable parameters: game count, time control, and output file path

### Key Implementation Details

- The self-play framework from M08 provides game-management infrastructure; this tool adds evaluation export on top
- Use randomized openings to ensure training data diversity
- For each position in each game, record the FEN, the handcrafted eval score, and the final game result
- The engine with opening book (M09) and tablebase support (M09) provides the baseline for generating high-quality self-play data
- The handcrafted evaluation from `crates/eval` (M03/M07) is the evaluation source for training labels

### Files to Create or Modify

- **Create**: `tools/datagen/` — new binary crate for training data generation
- **Modify**: `Cargo.toml` (workspace root) — add the new `tools/datagen` binary to the workspace members

### Dependencies

- Phase P01 (NNUE Architecture) must be complete — the network architecture and `.nnue` format define what the training data targets
- M08 (Testing, Tuning & Strength Validation) — self-play framework and SPRT testing infrastructure
- M09 (Opening Book & Endgame Tablebases) — completed engine with book and tablebase support
- `crates/eval` — existing handcrafted evaluation used to label positions
- `crates/search` — search framework used for self-play game playing
- No dependencies on other tasks within this phase

### Verification Criteria

1. The `tools/datagen` binary compiles and runs successfully
2. Running the tool produces an output file containing training records in the expected format
3. Each record contains a valid FEN, a centipawn evaluation score, and a game result (1.0/0.5/0.0 or win/draw/loss)
4. The output file can be parsed by `bullet` or `nnue-pytorch` without errors
5. Game count, time control, and output path are configurable via command-line arguments
6. The tool successfully completes self-play games using randomized openings
7. `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` all pass