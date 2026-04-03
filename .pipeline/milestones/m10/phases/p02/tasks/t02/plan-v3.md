# Manual verification — not a new test file, just a one-off CLI check
xxd -l 28 nets/nn.nnue  # Verify header: magic NNUE, version 1, correct arch hash and dims
```
No new Rust test file is created — the existing tests in `crates/nnue/src/loader.rs` already validate the loader. The trained `nets/nn.nnue` file is verified manually since it is produced outside the build system.
**Pass criteria**: `cargo test -p nnue` passes. Manual header inspection shows correct magic bytes (`4e4e5545`), version (`01000000`), and matching architecture hash and dimensions.

### Test 2: Inference produces valid scores

**What**: Verify the trained network produces bounded evaluations for the starting position.
**Method**: Manually load `nets/nn.nnue`, create an `Accumulator`, call `refresh()` with the starting position, then call `forward()`. This is performed as a manual check (not a new test file) since the trained weights file is produced outside the build system.
**Pass criteria**: The returned score satisfies `|score| < 3000` centipawns for the starting position (a roughly equal position should evaluate near 0; scores beyond ±3000 indicate broken weights).

### Test 3: Training pipeline documentation is complete and accurate

**What**: Verify `docs/training-pipeline.md` contains all 7 sections (Prerequisites, Generate Data, Train, Convert, Validate Loading, SPRT Validation, Iterative Workflow) with exact, copy-pasteable commands.
**Pass criteria**: Every command in the document uses correct CLI flags matching the actual tool interfaces (`datagen`, `nnue-train`, `convert.py`). The SPRT section provides a CuteChess-CLI template command with `option.EvalMode=nnue` and notes that the exact invocation will be finalized by T03. No placeholder values — all arguments are concrete.

### Test 4: Convert script produces valid output

**What**: Verify `tools/training/convert.py` produces a file that passes loader validation.
**Method**: Run the script on bullet's training output, then load the result via the Rust loader:
```bash
python3 tools/training/convert.py --input data/training-output/epoch_100.bin --output /tmp/test.nnue
cargo test -p nnue  # Existing loader tests validate format compliance
xxd -l 28 /tmp/test.nnue  # Verify header
```
**Pass criteria**: `crates::nnue::loader::load()` succeeds on the converted file. Header fields match expected values.

### Test 5: SPRT validation passes (handoff to T03)

**What**: The trained network demonstrates positive Elo gain over the handcrafted eval.
**Handoff contract**: T02 produces `nets/nn.nnue` and documents the SPRT command template in `docs/training-pipeline.md`. T03 integrates NNUE eval into the search loop, finalizes the SPRT invocation mechanism (either extending `tools/sprt` with per-engine UCI option support or invoking CuteChess-CLI directly), and runs the SPRT match as the final validation step. If SPRT rejects, T03 reports back to iterate the T02 pipeline (retrain with adjusted hyperparameters or more data, following the concrete remediation steps in the Iterative Workflow documentation).
**Pass criteria**: SPRT exits with `ACCEPT` status, indicating the H1 hypothesis (Elo gain > 0) is confirmed at 95% confidence. This criterion is satisfied jointly by T02 (network quality) and T03 (integration correctness).

## 6. Implementation Order

1. **Create `tools/training/Cargo.toml` and `tools/training/src/main.rs`** — Define the bullet training script with the HalfKP 40960→256→32→1 architecture. Hyperparameters: initial learning rate `0.001` with ×0.1 drops at epochs 75 and 90, batch size `16384`, 100 epochs, WDL weight `0.5`, clipped-ReLU activation. The save callback writes raw `f32` weights with a 24-byte dimension header.

2. **Create `tools/training/convert.py`** — Implement the weight conversion script that reads the raw `f32` weight file (24-byte dimension header + flat `f32` arrays in layer order) and writes the project's quantized `.nnue` binary format (28-byte header + `i16`/`i8`/`i32` arrays). Validate input dimensions before converting. Test with a synthetic weight file (e.g., all-zeros) to verify header and layout correctness before real training.

3. **Generate training data** — Run `tools/datagen` to produce a training dataset. Start with 1,000 games (~70K positions) to validate the pipeline end-to-end, then scale to 150,000 games (~10M positions) for the final training run.

4. **Train the network** — Run `tools/training/src/main.rs` via Cargo to train on the generated data. Monitor loss convergence across epochs.

5. **Convert trained weights** — Run `convert.py` to produce `nets/nn.nnue` from the best epoch's raw weights. Verify it loads correctly via `cargo test -p nnue` and manual header inspection.

6. **Validate inference** — Load the network and verify it produces evaluation scores with `|score| < 3000` centipawns for the starting position and other known positions.

7. **Create `docs/training-pipeline.md`** — Write the complete documentation capturing the exact commands, configuration, and workflow executed in steps 1–6, plus the SPRT validation procedure template (finalized by T03). Written last to ensure it reflects the actual successful workflow.

8. **Place trained `nets/nn.nnue`** — Commit the final trained weights file.

## 7. Verification Commands

```bash
# 1. Verify the project compiles
cargo build --release

# 2. Verify datagen builds (T01 prerequisite)
cargo build --release -p datagen

# 3. Build the training script (standalone, not part of workspace)
cargo build --release --manifest-path tools/training/Cargo.toml

# 4. Generate a small training dataset for pipeline validation
cargo run --release -p datagen -- \
  --openings tools/selfplay/openings.txt \
  --games 1000 \
  --nodes 5000 \
  --output /tmp/test_training.plain \
  --maxmoves 300

# 5. Verify training data format (spot check first 3 lines)
head -3 /tmp/test_training.plain

# 6. Run a short training (5 epochs) to validate the pipeline
cargo run --release --manifest-path tools/training/Cargo.toml -- \
  --data /tmp/test_training.plain \
  --output-dir /tmp/training-output \
  --epochs 5 \
  --batch-size 16384 \
  --lr 0.001

# 7. Verify raw weight file was produced
ls -la /tmp/training-output/epoch_5.bin

# 8. Convert raw weights to .nnue format
python3 tools/training/convert.py \
  --input /tmp/training-output/epoch_5.bin \
  --output /tmp/test.nnue

# 9. Verify .nnue file header (magic=NNUE, version=1, dims)
xxd -l 28 /tmp/test.nnue

# 10. Verify the .nnue file loads via existing loader tests
cargo test -p nnue

# 11. Verify the final trained nets/nn.nnue file exists and has correct header
xxd -l 28 nets/nn.nnue

# 12. Verify documentation exists
cat docs/training-pipeline.md

# 13. Verify conversion script exists
ls -la tools/training/convert.py
```

## 8. Design Deviations

**Deviation 1: Weight conversion script as pipeline tooling**

- **Parent spec requires**: "Training pipeline using `bullet` or `nnue-pytorch` produces a trained `.nnue` network file" and lists deliverables as "documentation of the training pipeline: exact commands, configuration files, data requirements."
- **What we produce instead**: A Python conversion script (`tools/training/convert.py`) and a Rust training script (`tools/training/src/main.rs`) in addition to configuration and documentation.
- **Why this is necessary**: Bullet does not natively output arbitrary custom `.nnue` formats. Bullet provides a save callback mechanism where each engine defines its own weight serialization. The standard approach in the chess engine community is: (1) bullet trains and exposes raw floating-point weights via a save callback, (2) the engine project provides its own serialization logic to write those weights in its custom format. Our project's `.nnue` format has a project-specific 28-byte header with a custom architecture hash and specific quantization scheme (`i16`/`i8`/`i32` at `2^6`/`2^12` scales) that no external tool produces natively.
- **Spec justification**: The task spec's deliverable "documentation of the training pipeline: exact commands, configuration files, data requirements, and expected outputs" encompasses all tooling needed to execute the pipeline. The conversion script and training script are pipeline artifacts — analogous to build tools — required to produce the `.nnue` deliverable. The task spec also lists "Training configuration files (hyperparameters, data paths, etc.)" under files to create, and the training script is the authoritative source of those hyperparameters.

**Deviation 2: SPRT validation is a joint T02+T03 exit criterion**

- **Parent spec requires**: "An SPRT self-play match (Elo0=0, Elo1=30) using CuteChess-CLI passes the H1 hypothesis."
- **Why T02 alone cannot satisfy this**: The SPRT match requires two engine binaries — one with NNUE eval, one with handcrafted eval. The UCI `EvalMode` option and search-level NNUE integration are implemented in T03 (Evaluation Switchover), not T02. Without T03, there is no engine binary capable of using NNUE evaluation during search.
- **Chosen resolution — explicit handoff contract with T03**:
  - **T02 responsibility**: Produce a trained `nets/nn.nnue` file that loads correctly and produces valid inference scores (`|score| < 3000` cp for the starting position). Document the complete SPRT validation procedure with a CuteChess-CLI template command in `docs/training-pipeline.md`.
  - **T03 responsibility**: Integrate NNUE eval into the search loop, finalize the SPRT invocation mechanism (either extend `tools/sprt` with per-engine UCI option flags or invoke CuteChess-CLI directly using the template from T02's documentation), build the candidate binary, and execute the SPRT match using `nets/nn.nnue` produced by T02. If the SPRT test rejects (network not strong enough), T03 reports failure back so the T02 pipeline can be iterated (more data, adjusted hyperparameters, retrain).
  - **Joint exit criterion**: The phase-level exit criterion "SPRT self-play match passes" is satisfied when T03 runs the match and it passes. T02's contribution is the network quality; T03's contribution is the integration correctness.
- **Why not build a temporary harness in T02**: Building a temporary search integration solely for SPRT testing would duplicate T03's scope (search integration, UCI option), violate scope discipline, and produce throwaway code. The handoff contract is cleaner and ensures each task owns its deliverables.