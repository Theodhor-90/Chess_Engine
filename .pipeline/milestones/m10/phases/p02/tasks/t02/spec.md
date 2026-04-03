## Task T02 — Training Pipeline

### Objective

Establish the end-to-end training workflow using `bullet` or `nnue-pytorch` as external tooling. Document the exact steps to go from generated training data to a trained `.nnue` weights file. Execute the iterative generate → train → embed → test → repeat workflow until the trained network demonstrates positive Elo gain over the handcrafted eval.

### Deliverables

- A trained `.nnue` weights file that can be loaded by the inference engine from Phase P01
- Documentation of the training pipeline: exact commands, configuration files, data requirements, and expected outputs
- Validation results from an SPRT self-play match demonstrating positive Elo gain over the handcrafted eval

### Key Implementation Details

- **External tooling**: `bullet` or `nnue-pytorch` is consumed — no training code is built within this project
- **Reproducible workflow**: The pipeline documentation must be detailed enough for someone to reproduce the training from scratch: generate data → train with external tool → produce `.nnue` file → embed into engine → validate
- **SPRT validation**: The trained network must demonstrate positive Elo gain over the handcrafted eval, validated by an SPRT self-play match (Elo0=0, Elo1=30, alpha=0.05, beta=0.05) using CuteChess-CLI, passing the H1 hypothesis (Elo gain > 0) at 95% confidence
- **Iterative improvement**: The generate → train → embed → test cycle is repeated until the SPRT test passes
- The network architecture, topology, and weight format are defined in Phase P01 — the training must target that architecture
- The `.nnue` file produced must be loadable by the weight-loading code implemented in Phase P01 T03

### Files to Create or Modify

- **Create**: Training pipeline documentation (e.g., `docs/training-pipeline.md` or `tools/training/README.md`)
- **Create**: Training configuration files (hyperparameters, data paths, etc.)
- **Create**: Trained `.nnue` weights file (placed in a suitable location, e.g., `nets/` or `data/`)
- **Potentially modify**: `tools/datagen` if adjustments to data format are needed during training iteration

### Dependencies

- **T01 (Training Data Generation)** — training data must be generated before training can begin
- Phase P01 (NNUE Architecture) — network architecture, inference engine, and `.nnue` format must be complete
- M08 — CuteChess-CLI integration and SPRT testing framework for validation
- M09 — completed engine for baseline self-play

### Verification Criteria

1. A trained `.nnue` weights file exists and loads correctly via the Phase P01 weight-loading code
2. The inference engine produces valid evaluation scores when using the trained weights
3. Training pipeline documentation exists and describes the complete workflow with exact commands
4. An SPRT self-play match (Elo0=0, Elo1=30, alpha=0.05, beta=0.05) using CuteChess-CLI passes the H1 hypothesis (Elo gain > 0) at 95% confidence
5. The iterative workflow (generate → train → embed → test → repeat) is documented and has been executed at least once