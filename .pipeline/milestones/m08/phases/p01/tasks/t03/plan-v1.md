# Build the self-play binary
cargo build --bin selfplay

# Build the engine in release mode (needed for reasonable game speed)
cargo build --release --bin engine

# Run a quick smoke test (2 games)
cargo run --release --bin selfplay -- \
  --engine target/release/engine \
  --openings tools/selfplay/openings.txt \
  --games 2 \
  --movetime 50

# Run the full 2,000-game self-play session
cargo run --release --bin selfplay -- \
  --engine target/release/engine \
  --openings tools/selfplay/openings.txt \
  --games 2000 \
  --movetime 100

# Verify no clippy warnings
cargo clippy --bin selfplay -- -D warnings

# Verify formatting
cargo fmt --check
```

## 8. Design Deviations

**Deviation 1: Fresh engine processes per game instead of persistent reuse**

- **Parent spec states**: "The self-play harness launches two instances of the engine binary and communicates via UCI stdin/stdout."
- **Issue**: The phrasing could imply two long-lived engine instances across all 2,000 games. However, reusing processes risks state leakage between games (transposition table, history tables) that could mask bugs. Additionally, crash detection is simpler with per-game spawning — if a process dies mid-game, we know exactly which game caused it.
- **Alternative chosen**: Spawn a fresh pair of engine processes for each game. Send `uci`/`uciok`, `isready`/`readyok` at the start, `quit` at the end. This is how CuteChess-CLI operates by default and is the most robust approach for detecting assertion failures and crashes per-game.

**Deviation 2: Harness-side game adjudication instead of relying on engine output**

- **Parent spec states**: "Detect crashes, illegal moves, assertion failures, and time losses."
- **Clarification**: The spec does not prescribe where game-ending conditions (checkmate, stalemate, draws) are detected. Detecting them on the harness side (by maintaining a parallel `Position` and generating legal moves) is strictly more robust than relying on engine self-reporting, because it catches bugs where the engine fails to recognize checkmate or claims a draw incorrectly. This is the same approach used by CuteChess-CLI and other tournament managers.