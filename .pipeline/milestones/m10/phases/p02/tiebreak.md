## Phase P02 — Training & Integration

### Goal

Generate training data from the existing handcrafted engine via self-play, train the NNUE network using external tooling (`bullet` or `nnue-pytorch`), and integrate the trained NNUE evaluation into the engine's search loop as a replacement for the handcrafted evaluation — with the handcrafted eval retained as a fallback and a UCI option to select evaluation mode.

### Design Decisions

1. **Training data format**: Each record consists of a FEN string, the handcrafted evaluation score (in centipawns), and the game result (win/draw/loss from the side-to-move perspective). This format is compatible with both `bullet` and `nnue-pytorch` training pipelines.

2. **Self-play data generation**: A dedicated binary in `tools/` uses the handcrafted engine to play games against itself, recording positions with evaluations and outcomes. The self-play framework from M08 provides the game-management infrastructure; the data generator adds evaluation export on top.

3. **External training tooling**: `bullet` or `nnue-pytorch` is consumed as external tooling — no training code is built within this project. The training pipeline is documented as a reproducible workflow: generate data → train with external tool → produce `.nnue` file → embed into engine → validate.

4. **Evaluation integration point**: NNUE evaluation is called from `crates/search` during the search loop, replacing the call to the handcrafted eval in `crates/eval`. The handcrafted eval remains available and is used as fallback when no `.nnue` file is loaded or when the UCI option selects it.

5. **UCI evaluation mode option**: A new UCI option `EvalMode` is added in `crates/uci` allowing the user to select between `nnue` and `classical` evaluation. The engine defaults to NNUE when a network is loaded.

6. **Iterative improvement workflow**: The generate → train → embed → test cycle is repeated until the trained network demonstrates positive Elo gain over the handcrafted eval, validated by SPRT self-play match using CuteChess-CLI from M08.

### Tasks

**T01 — Training Data Generation**

Build a self-play data generation tool that uses the handcrafted engine to produce training data. The tool plays games against itself using randomized openings, and for each position in each game, exports the FEN, the handcrafted evaluation score, and the game result. Deliverables: a binary in `tools/` that generates training data files in the format required by `bullet` or `nnue-pytorch`, with configurable game count, time control, and output path.

**T02 — Training Pipeline**

Establish the end-to-end training workflow using `bullet` or `nnue-pytorch` as external tooling. Document the exact steps to go from generated training data to a trained `.nnue` weights file. Execute the iterative generate → train → embed → test → repeat workflow until the trained network demonstrates positive Elo gain over the handcrafted eval. Deliverables: a trained `.nnue` weights file, documentation of the training pipeline (commands, configuration, data requirements), and validation results from the SPRT match.

**T03 — Evaluation Switchover**

Integrate NNUE evaluation into the search loop in `crates/search`, replacing the handcrafted eval call with an NNUE forward pass when a network is loaded. Add the UCI option `EvalMode` in `crates/uci` to select between NNUE and classical evaluation modes. Retain the handcrafted eval in `crates/eval` as a fallback. Deliverables: search integration that calls NNUE eval during search, UCI `EvalMode` option, fallback to handcrafted eval when no network is loaded or when classical mode is selected.

### Exit Criteria

1. Training data generation produces self-play data with position evaluations exported as FEN + eval + game result
2. Training pipeline using `bullet` or `nnue-pytorch` produces a trained `.nnue` network file
3. The trained network demonstrates positive Elo gain over the handcrafted eval, validated by an SPRT self-play match (Elo0=0, Elo1=30, alpha=0.05, beta=0.05) using CuteChess-CLI, passing the H1 hypothesis (Elo gain > 0) at 95% confidence
4. NNUE evaluation is integrated into the search loop in `crates/search`
5. UCI option `EvalMode` selects between NNUE and handcrafted evaluation modes
6. Handcrafted eval in `crates/eval` remains available as fallback when no network is loaded or when classical mode is selected

### Dependencies

- **Phase P01 (NNUE Architecture)** — the network architecture, inference engine (`crates/nnue`), and `.nnue` weight-loading format must be complete
- **M08 (Testing, Tuning & Strength Validation)** — self-play framework and SPRT testing infrastructure for data generation and validation
- **M09 (Opening Book & Endgame Tablebases)** — completed engine with book and tablebase support provides the baseline for self-play data generation
- **crates/eval** — existing handcrafted evaluation that NNUE replaces and that remains as fallback
- **crates/search** — search framework into which NNUE evaluation is integrated
- **crates/uci** — UCI protocol layer where the `EvalMode` option is added

### Artifacts

- `tools/datagen` (or equivalent binary) — training data generation tool
- Trained `.nnue` weights file
- Training pipeline documentation
- Modified `crates/search` — NNUE eval integration in the search loop
- Modified `crates/uci` — UCI `EvalMode` option
- Modified `crates/engine` — wiring of NNUE network loading and evaluation mode selection