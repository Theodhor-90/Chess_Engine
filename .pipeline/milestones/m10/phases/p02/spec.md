## Phase P02 — Training & Integration

### Goal

Generate training data from the existing handcrafted engine via self-play, train the NNUE network using external tooling (`bullet` or `nnue-pytorch`), and integrate the trained NNUE evaluation into the engine's search loop as a replacement for the handcrafted evaluation — with the handcrafted eval retained as a fallback and a UCI option to select evaluation mode.

### Deliverables

- **T01**: Training data generation — use the handcrafted engine to generate self-play games with position evaluations. Export as training data (FEN + eval + game result).
- **T02**: Training pipeline — use `bullet` or `nnue-pytorch` to train the network on generated data. Iterate: generate data → train → embed → test → repeat.
- **T03**: Evaluation switchover — NNUE evaluation replaces handcrafted eval in search. Handcrafted eval remains as fallback. UCI option to select evaluation mode.

### Technical Decisions & Constraints

- Training data format: FEN + evaluation score + game result (win/draw/loss)
- Training data is generated via self-play using the existing handcrafted engine from M03/M07/M08
- External tooling is consumed, not built: `bullet` or `nnue-pytorch` for network training
- Iterative workflow: generate data → train → embed → test → repeat until quality targets are met
- NNUE evaluation replaces handcrafted eval in `crates/search` during the search loop
- Handcrafted eval from `crates/eval` must remain available as fallback
- A UCI option must be added to select between NNUE and handcrafted evaluation modes
- The trained network must evaluate positions more accurately than the handcrafted eval (exit criterion)
- Self-play framework from M08 and CuteChess-CLI integration are used for validation

### Dependencies

- **Phase P01 (NNUE Architecture)** — the network architecture, inference engine, and weight-loading format must be complete before training and integration can proceed
- **M08 (Testing, Tuning & Strength Validation)** — self-play framework and SPRT testing infrastructure required for data generation and validation
- **M09 (Opening Book & Endgame Tablebases)** — completed engine with book and tablebase support provides the baseline for self-play data generation
- **crates/eval** — existing handcrafted evaluation that NNUE replaces and that remains as fallback
- **crates/search** — search framework into which NNUE evaluation is integrated
- **crates/uci** — UCI protocol layer where the evaluation mode option is added