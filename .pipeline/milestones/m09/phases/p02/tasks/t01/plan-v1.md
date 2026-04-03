I have all the context I need. The UCI output module doesn't have an `option_spin` function, so that's something we'll need to add for `SyzygyProbeDepth`. Now let me produce the task plan.

## 1. Deliverables

- `crates/engine/src/syzygy/mod.rs` — Syzygy tablebase probing module: WDL/DTZ result types, score conversion, initialization, probe interface wrapping the `fathom-syzygy` crate, and piece count gating.
- `crates/engine/src/lib.rs` — Modified to add `pub mod syzygy;` declaration.
- `crates/engine/Cargo.toml` — Modified to add `fathom-syzygy` dependency.
- `crates/uci/src/output.rs` — Modified to add `option_spin` function for declaring integer UCI options (needed by `SyzygyProbeDepth`).

## 2. Dependencies

- `fathom-syzygy` crate (external) — Rust bindings to the Fathom C library for Syzygy tablebase probing. Provides `tb_init`, `tb_probe_wdl`, `tb_probe_root_dtz` and related functions. The `cc` crate compilation of Fathom C source is handled internally by `fathom-syzygy`.
- `chess-board` (already a dependency of engine crate) — provides `Position`.
- `chess-types` (already a dependency of engine crate) — provides `Bitboard`, `Color`, `Piece`, `PieceKind`, `Square`.
- `chess-movegen` (already a dependency of engine crate) — provides `generate_legal_moves` for piece counting if needed.
- External: Syzygy `.rtbw` (WDL) and `.rtbz` (DTZ) tablebase files (3–5 piece minimum) for testing.

**Note on crate choice**: The task spec says "integrate `Fathom` (C library) or a Rust port." The `fathom-syzygy` crate wraps the Fathom C library via FFI and handles compilation through `cc` internally. If `fathom-syzygy` proves problematic at implementation time (e.g., build issues on macOS/ARM), `pyrrhic-rs` is an acceptable alternative — both provide WDL and DTZ probe functionality. This decision is finalized during implementation per the task spec.

## 3. Implementation Details

### `crates/uci/src/output.rs` (modification)

Add a single public function:

```rust
pub fn option_spin(name: &str, default: i32, min: i32, max: i32) -> String {
    format!("option name {name} type spin default {default} min {min} max {max}")
}
```

This is needed because `SyzygyProbeDepth` (from T03) is a spin-type UCI option, and the probe depth limit is used in this task's piece-count gating logic. While T03 owns the UCI option declaration, T01 must define the gating mechanism. Adding `option_spin` here is minimal infrastructure that the crate is missing.

### `crates/engine/src/syzygy/mod.rs` (new file)

**Imports**: `chess_board::Position`, `chess_types::{Bitboard, Color, Piece, PieceKind}`.

**WDL result type**:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wdl {
    Loss,
    BlessedLoss,  // Loss but saved by 50-move rule
    Draw,
    CursedWin,    // Win but drawn by 50-move rule
    Win,
}
```

**DTZ result type**:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dtz(pub i32);
```
Positive = moves until zeroing for winning side, negative = for losing side.

**Probe result**:
```rust
#[derive(Clone, Copy, Debug)]
pub struct ProbeResult {
    pub wdl: Wdl,
    pub dtz: Option<Dtz>,
}
```

**SyzygyTablebase struct**:
```rust
pub struct SyzygyTablebase {
    // Internal state from the fathom-syzygy crate (e.g., a Prober or similar handle).
    // Exact fields depend on the chosen crate's API.
    initialized: bool,
    max_pieces: u8,  // Largest piece count supported by loaded tables
}
```

**Initialization function**:
```rust
impl SyzygyTablebase {
    /// Initialize tablebase probing from the given directory path.
    /// Returns `Err` if the path is invalid or no tablebase files are found.
    pub fn init(path: &str) -> Result<SyzygyTablebase, String> { ... }
}
```
- Calls the underlying library's init function with the path.
- Stores the max supported piece count from the library.
- Returns an error string if initialization fails (no `unwrap()` — library crate convention).

**Piece count helper**:
```rust
fn piece_count(pos: &Position) -> u8 {
    pos.occupied().pop_count() as u8
}
```

**WDL probe**:
```rust
impl SyzygyTablebase {
    /// Probe WDL value for the given position.
    /// Returns `None` if the position has too many pieces or probing fails.
    pub fn probe_wdl(&self, pos: &Position, probe_limit: u8) -> Option<Wdl> { ... }
}
```
- Checks `piece_count(pos) <= probe_limit` and `piece_count(pos) <= self.max_pieces`. Returns `None` if either fails.
- Converts the `Position` fields to the format required by the underlying library (bitboards for each piece type, side to move, castling rights, en passant square, halfmove clock).
- Calls the library's WDL probe function.
- Maps the library's WDL result to our `Wdl` enum.

**DTZ probe**:
```rust
impl SyzygyTablebase {
    /// Probe DTZ value for the given position.
    /// Returns `None` if the position has too many pieces or probing fails.
    pub fn probe_dtz(&self, pos: &Position, probe_limit: u8) -> Option<Dtz> { ... }
}
```
- Same gating as `probe_wdl`.
- Calls the library's DTZ probe function.
- Returns `Dtz(value)`.

**Score conversion**:
```rust
/// Convert a WDL result to an engine-internal score.
/// Wins map to `MATE_SCORE - offset`, losses to `-(MATE_SCORE - offset)`.
/// `CursedWin` and `BlessedLoss` map to draw-ish scores (small positive/negative).
pub fn wdl_to_score(wdl: Wdl) -> i32 { ... }
```
- `Wdl::Win` → `MATE_SCORE - 200` (a large winning score below actual checkmate so search prefers checkmate over TB win, but clearly winning)
- `Wdl::CursedWin` → `1` (technically winning but drawn by 50-move rule; slightly above draw)
- `Wdl::Draw` → `0`
- `Wdl::BlessedLoss` → `-1` (technically losing but drawn by 50-move rule; slightly below draw)
- `Wdl::Loss` → `-(MATE_SCORE - 200)`

Uses the constant `MATE_SCORE` value of `30000` from `chess_search::MATE_SCORE`. To avoid a circular dependency (engine depends on search, not vice versa), we define a local constant `const TB_WIN_SCORE: i32 = 29800;` (i.e., `30000 - 200`) rather than importing from the search crate.

**Position-to-library conversion** (internal helper):

The position fields needed by Fathom's C API (or equivalent Rust wrapper) are:
- Bitboards per piece-color: extracted via `pos.piece_bitboard(Piece::new(color, kind)).inner()`
- Occupied bitboards: `pos.occupied_by(color).inner()`, `pos.occupied().inner()`
- Side to move: `pos.side_to_move()`
- Castling rights: `pos.castling_rights().inner()` — the internal u8 bitfield
- En passant square: `pos.en_passant().map(|sq| sq.index())` (0-63 index or 0/none)
- Halfmove clock: `pos.halfmove_clock()`

The exact mapping depends on the chosen probing library's function signatures.

**`// SAFETY:` comments**: All `unsafe` blocks wrapping FFI calls include `// SAFETY:` comments per project convention.

**Unit tests** (colocated in `#[cfg(test)] mod tests`):

| Test | What it verifies |
|------|-----------------|
| `wdl_to_score_win` | `wdl_to_score(Wdl::Win)` returns `TB_WIN_SCORE` (29800) |
| `wdl_to_score_loss` | `wdl_to_score(Wdl::Loss)` returns `-TB_WIN_SCORE` (-29800) |
| `wdl_to_score_draw` | `wdl_to_score(Wdl::Draw)` returns `0` |
| `wdl_to_score_cursed_win` | `wdl_to_score(Wdl::CursedWin)` returns `1` |
| `wdl_to_score_blessed_loss` | `wdl_to_score(Wdl::BlessedLoss)` returns `-1` |
| `piece_count_startpos` | `piece_count` on start position returns `32` |
| `piece_count_endgame` | `piece_count` on a KRK FEN returns `3` |
| `probe_wdl_rejects_too_many_pieces` | `probe_wdl` returns `None` when piece count exceeds `probe_limit` |

Tests that require actual `.rtbw`/`.rtbz` files (e.g., verifying correct WDL/DTZ for KRK positions) are guarded by an `#[ignore]` attribute since tablebase files are external and may not be present in CI. These tests include:
- `probe_wdl_krk_win` — KRK position returns `Win` for side with rook
- `probe_dtz_krk` — KRK position returns a valid DTZ value
- `probe_wdl_kk_draw` — KK position returns `Draw`

### `crates/engine/src/lib.rs` (modification)

Add `pub mod syzygy;` alongside the existing `pub mod book;`.

### `crates/engine/Cargo.toml` (modification)

Add `fathom-syzygy` to `[dependencies]`. Exact version pinned at implementation time based on latest stable release.

## 4. API Contracts

```rust
use chess_engine::syzygy::{SyzygyTablebase, Wdl, Dtz, ProbeResult, wdl_to_score};
use chess_board::Position;

// Initialize
let tb = SyzygyTablebase::init("/path/to/syzygy/tables")?;

// Probe WDL (probe_limit = max pieces to probe, typically 5-7)
let wdl: Option<Wdl> = tb.probe_wdl(&pos, 6);

// Probe DTZ
let dtz: Option<Dtz> = tb.probe_dtz(&pos, 6);

// Convert WDL to engine score
if let Some(wdl) = wdl {
    let score: i32 = wdl_to_score(wdl);
}
```

N/A for REST/HTTP.

## 5. Test Plan

| Test | What it verifies |
|------|-----------------|
| `wdl_to_score_win` | Win maps to `TB_WIN_SCORE` (29800) |
| `wdl_to_score_loss` | Loss maps to `-TB_WIN_SCORE` (-29800) |
| `wdl_to_score_draw` | Draw maps to `0` |
| `wdl_to_score_cursed_win` | CursedWin maps to `1` |
| `wdl_to_score_blessed_loss` | BlessedLoss maps to `-1` |
| `piece_count_startpos` | Start position has 32 pieces |
| `piece_count_endgame` | KRK FEN position has 3 pieces |
| `probe_wdl_rejects_too_many_pieces` | Returns `None` when `piece_count > probe_limit` |
| `probe_wdl_krk_win` (`#[ignore]`) | KRK returns `Win` for rook side (requires TB files) |
| `probe_dtz_krk` (`#[ignore]`) | KRK returns valid DTZ (requires TB files) |
| `probe_wdl_kk_draw` (`#[ignore]`) | KK returns `Draw` (requires TB files) |

Tests requiring tablebase files are `#[ignore]` and run manually with `cargo test -- --ignored` when files are available.

## 6. Implementation Order

1. `crates/uci/src/output.rs` — add `option_spin` function
2. `crates/engine/Cargo.toml` — add `fathom-syzygy` dependency
3. `crates/engine/src/syzygy/mod.rs` — implement `Wdl`, `Dtz`, `ProbeResult` types, `piece_count`, `wdl_to_score`, `SyzygyTablebase::init`, `probe_wdl`, `probe_dtz`, and all unit tests
4. `crates/engine/src/lib.rs` — add `pub mod syzygy;`
5. `Cargo.toml` (workspace root) — no change needed (engine crate already a workspace member)

## 7. Verification Commands

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test -p engine
cargo test -p chess-uci
cargo build -p engine
```

## 8. Design Deviations

**Score constant locality**: The phase spec states that WDL results should be "converted to the engine's internal score representation, with wins/losses mapped to mate-scale scores adjusted for distance." The search crate defines `MATE_SCORE = 30000`. Rather than adding `chess-search` as a dependency of the engine library crate (which would create a heavier dependency chain for just one constant), we define `const TB_WIN_SCORE: i32 = 29800;` locally in the syzygy module. This equals `MATE_SCORE - 200`, placing TB wins below actual checkmate scores but clearly in winning territory. The offset of 200 ensures TB wins don't collide with real mate scores (which are `MATE_SCORE - ply` where ply < 128). This is a standard approach in chess engines and correctly implements the spec's intent without coupling the modules.

**`option_spin` addition**: The task spec does not explicitly list modifying the UCI output module, but `SyzygyProbeDepth` is a spin-type UCI option (integer with min/max). The existing UCI output module only has `option_string` and `option_combo`. Adding `option_spin` is the minimal infrastructure needed to support the integer probe-depth option that T03 will declare and that T01's piece-count gating depends on.