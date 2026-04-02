# T03 — UCI Options and Lazy Loading

## Objective

Add UCI options for configuring the Syzygy tablebase directory path and probe depth, implement lazy loading of tablebase files (loaded on first probe rather than at startup), and integrate option handling into the engine's `setoption` command processing.

## Deliverables

- UCI option `SyzygyPath` — a string option specifying the directory path to Syzygy tablebase files
- UCI option `SyzygyProbeDepth` — an integer option specifying the maximum piece count for probing during search (e.g., default 6 for up to 6-piece tablebases)
- Lazy loading: tablebase files are only loaded or memory-mapped when the first probe is requested, not at engine startup or when the option is set
- Integration into the engine's `setoption` command processing in `crates/engine/src/main.rs`
- The options are advertised in the UCI `uci` command response (printed with `option name ... type ...`)

## Key Implementation Details

- **SyzygyPath**: When set via `setoption`, store the path but do not immediately load tablebase files. On first probe, initialize the probing library with the configured path (lazy loading).
- **SyzygyProbeDepth**: Controls the maximum number of pieces on the board for which in-search tablebase probing is attempted. Root probing may use a higher or equal limit. Default value should be sensible (e.g., 6).
- **Lazy loading pattern**: Use a mechanism (e.g., `OnceCell`, `Option<T>` with lazy init, or similar) to defer tablebase initialization until the first probe call. This avoids memory and latency costs when tablebases are configured but not needed in the current game.
- **Re-initialization**: If `SyzygyPath` is changed via `setoption`, the next probe should use the new path (invalidate any cached initialization).
- **UCI output**: On `uci` command, print `option name SyzygyPath type string default <empty>` and `option name SyzygyProbeDepth type spin default 6 min 0 max 7` (or appropriate range).

## Files to Create or Modify

- Modify: `crates/engine/src/main.rs` — add `SyzygyPath` and `SyzygyProbeDepth` UCI option handling to `setoption` processing, advertise options in `uci` response
- Modify: Syzygy probing module/crate (from T01) — add lazy loading initialization logic
- Modify: `crates/uci/` — if UCI option definitions are centralized there, add the new options

## Dependencies

- **T01** (Syzygy tablebase probing) must be complete — the initialization and probe interface are required
- **T02** (Search integration) must be complete — the probe depth limit is used by the search integration to gate probing
- M04 (UCI Protocol) — UCI option support and `setoption` command processing are already in place

## Verification Criteria

1. `uci` command response includes `option name SyzygyPath type string default` and `option name SyzygyProbeDepth type spin default 6 min 0 max 7` (or similar)
2. Setting `SyzygyPath` via `setoption name SyzygyPath value /path/to/tablebases` stores the path without immediately loading files
3. First tablebase probe after setting the path triggers lazy initialization and loads the tablebase files
4. Changing `SyzygyPath` after initialization invalidates the previous state and re-initializes on next probe
5. `SyzygyProbeDepth` correctly controls the piece-count gate for in-search probing
6. When `SyzygyPath` is not set or is empty, no probing is attempted and no errors occur
7. `cargo test` passes for all modified crates
8. `cargo clippy -- -D warnings` passes
9. `cargo fmt --check` passes