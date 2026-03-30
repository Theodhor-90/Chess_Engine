# T03 — Self-Play Framework

## Objective

Implement a tool that orchestrates self-play games between two instances of the engine via UCI. The framework uses randomized openings, runs at least 2,000 games, and detects crashes (process exit), illegal moves (engine sends invalid UCI output), assertion failures (non-zero exit code), and time losses (engine exceeds allocated time). This validates the full engine stack end-to-end under sustained use.

## Deliverables

- A standalone binary under `tools/selfplay/` that runs self-play tournaments via UCI
- A set of randomized opening positions (FENs or short opening move sequences) for game diversity
- Reporting of game outcomes (win/draw/loss counts from each side) and any detected errors (crashes, illegal moves, assertion failures, time losses)
- Successful completion of 2,000+ games without crashes, illegal moves, or assertion failures

## Files to Create or Modify

- **Create**: `tools/selfplay/` — directory for the self-play framework binary
- **Create**: `tools/selfplay/src/main.rs` (or appropriate Cargo binary structure) — the self-play orchestrator
- **Create**: A data file containing randomized opening positions (FENs or opening move sequences)
- **Modify**: `Cargo.toml` (workspace root) — add the new binary to the workspace

## Key Implementation Details

- The self-play harness launches two instances of the engine binary and communicates via UCI stdin/stdout, matching how CuteChess-CLI and other tournament managers drive engines.
- Each game starts from a randomized opening position to ensure diversity and avoid repetitive games.
- The framework must detect and report:
  - **Crashes**: engine process exits unexpectedly
  - **Illegal moves**: engine sends invalid UCI output or illegal move strings
  - **Assertion failures**: engine exits with non-zero exit code
  - **Time losses**: engine exceeds allocated time for a move
- Game adjudication: detect draws by repetition, 50-move rule, insufficient material, and stalemate. Adjudicate wins by checkmate.
- Time control must be configurable (e.g., time per move or increment-based).
- Must support running at least 2,000 games in a single invocation.
- Final output summarizes: total games, wins/draws/losses, and any errors encountered.

## Dependencies

- **T01** (Extended Perft Suite): No strict code dependency, but perft validation ensures movegen is correct before running thousands of self-play games.
- **T02** (Tactical Test Suite): No strict code dependency. Can be developed after or in parallel.
- Depends on M01–M07 being complete, particularly UCI protocol (M04) since this tool communicates via UCI.

## Verification Criteria

1. The binary builds and runs successfully: `cargo run --bin selfplay` (or equivalent)
2. The framework launches two engine instances and communicates via UCI
3. Games start from randomized opening positions
4. At least 2,000 games complete successfully
5. No crashes, illegal moves, or assertion failures are detected across the 2,000+ game run
6. Output reports: total games played, win/draw/loss counts, and any errors detected
7. Time losses are tracked and reported (a few time losses may be acceptable, but crashes and illegal moves must be zero)