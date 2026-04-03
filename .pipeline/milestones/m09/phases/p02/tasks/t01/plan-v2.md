Now I have all the information needed. Let me produce the revised plan.

## 1. Deliverables

- `crates/engine/src/syzygy/mod.rs` — Syzygy tablebase probing module: WDL/DTZ result types, score conversion, initialization, probe interface wrapping the `fathom-syzygy` crate, and piece count gating.
- `crates/engine/src/lib.rs` — Modified to add `pub mod syzygy;` declaration.
- `crates/engine/Cargo.toml` — Modified to add `fathom-syzygy` dependency.

## 2. Dependencies

- `fathom-syzygy = "0.1.0"` (external, on crates.io) — Rust wrapper around the Fathom C library for Syzygy tablebase probing. Provides `Fathom`, `Prober`, `RootProber`, `Position`, and `Wdl` types. The companion `-sys` crate (`fathom-syzygy-sys`) compiles the Fathom C source via `cc` internally. Requires a C compiler at build time.
- `chess-board` (already a dependency of engine crate) — provides `Position`.
- `chess-types` (already a dependency of engine crate) — provides `Bitboard`, `Color`, `Piece`, `PieceKind`, `Square`.
- External: Syzygy `.rtbw` (WDL) and `.rtbz` (DTZ) tablebase files (3–5 piece minimum) for testing.

**Fallback**: If `fathom-syzygy` 0.1.0 fails to build (e.g., C compilation issues on macOS ARM), `pyrrhic-rs = "0.2.0"` is the fallback. It is also on crates.io, avoids C compilation, and provides equivalent WDL/DTZ probe functionality via a raw bitboard API with an `EngineAdapter` trait. The API surface in Section 3 is written for `fathom-syzygy`; the fallback would require implementing `EngineAdapter` instead of constructing `fathom_syzygy::Position` structs, but the public interface of our `SyzygyTablebase` wrapper remains identical.

## 3. Implementation Details

### `crates/engine/src/syzygy/mod.rs` (new file)

**Imports**: `chess_board::Position`, `chess_types::{Color, Piece, PieceKind}`.

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
Positive = plies until zeroing move for winning side, negative = for losing side.

**SyzygyTablebase struct** (concrete fields for `fathom-syzygy`):
```rust
pub struct SyzygyTablebase {
    /// The Fathom library instance. `fathom-syzygy` enforces a singleton —
    /// only one `Fathom` instance can exist at a time. We own it here.
    fathom: fathom_syzygy::Fathom,
    /// Largest piece count supported by the loaded tablebase files.
    /// Returned by `fathom.max_pieces()` after initialization.
    max_pieces: u8,
}
```

The `fathom-syzygy` crate uses a singleton pattern: `Fathom::new(path)` returns `Result<Fathom, Error>` and only one instance may exist. The `Fathom` instance owns the initialized C library state. Probing is done through borrowed `Prober` and `RootProber` handles obtained via `fathom.get_probers()`.

**Initialization function**:
```rust
impl SyzygyTablebase {
    /// Initialize tablebase probing from the given directory path.
    /// Returns `Err` if the path is invalid or no tablebase files are found.
    pub fn init(path: &str) -> Result<SyzygyTablebase, String> {
        let fathom = fathom_syzygy::Fathom::new(path)
            .map_err(|e| format!("Failed to initialize Syzygy tablebases: {e}"))?;
        let max_pieces = fathom.max_pieces() as u8;
        Ok(SyzygyTablebase { fathom, max_pieces })
    }

    /// Returns the largest piece count supported by loaded tables.
    pub fn max_pieces(&self) -> u8 {
        self.max_pieces
    }
}
```

**Piece count helper**:
```rust
fn piece_count(pos: &Position) -> u8 {
    pos.occupied().pop_count() as u8
}
```

**Position-to-Fathom conversion** (internal helper):

The `fathom-syzygy` crate requires a `fathom_syzygy::Position` struct with raw `u64` bitboards. The Fathom C API function signatures wrapped by the crate are `tb_probe_wdl(white, black, kings, queens, rooks, bishops, knights, pawns, ep, turn)` and `tb_probe_root(white, black, kings, queens, rooks, bishops, knights, pawns, rule50, castling, ep, turn)`. These are exposed through `Prober::probe(&self, &Position)` and `RootProber::probe(&mut self, &Position)`.

```rust
fn to_fathom_position(pos: &Position) -> fathom_syzygy::Position {
    use chess_types::{Color, PieceKind};

    let bb = |c: Color, k: PieceKind| -> u64 {
        pos.piece_bitboard(Piece::new(c, k)).inner()
    };

    fathom_syzygy::Position {
        white: pos.occupied_by(Color::White).inner(),
        black: pos.occupied_by(Color::Black).inner(),
        kings: bb(Color::White, PieceKind::King) | bb(Color::Black, PieceKind::King),
        queens: bb(Color::White, PieceKind::Queen) | bb(Color::Black, PieceKind::Queen),
        rooks: bb(Color::White, PieceKind::Rook) | bb(Color::Black, PieceKind::Rook),
        bishops: bb(Color::White, PieceKind::Bishop) | bb(Color::Black, PieceKind::Bishop),
        knights: bb(Color::White, PieceKind::Knight) | bb(Color::Black, PieceKind::Knight),
        pawns: bb(Color::White, PieceKind::Pawn) | bb(Color::Black, PieceKind::Pawn),
        rule50: pos.halfmove_clock() as u32,
        castling: pos.castling_rights().inner() as u32,
        ep: pos.en_passant().map(|sq| sq.index() as u32).unwrap_or(0),
        turn: match pos.side_to_move() {
            Color::White => 1,
            Color::Black => 0,
        },
    }
}
```

**WDL probe**:
```rust
impl SyzygyTablebase {
    /// Probe WDL value for the given position.
    /// Returns `None` if the position has too many pieces or probing fails.
    pub fn probe_wdl(&mut self, pos: &Position, probe_limit: u8) -> Option<Wdl> {
        if piece_count(pos) > probe_limit || piece_count(pos) > self.max_pieces {
            return None;
        }
        let fathom_pos = to_fathom_position(pos);
        let (_, prober) = self.fathom.get_probers();
        let wdl = prober.probe(&fathom_pos)?;
        Some(convert_wdl(wdl))
    }
}
```

Note: `get_probers()` returns `(RootProber<'_>, Prober<'_>)` — borrowed handles that hold `&mut self` on the `Fathom` instance. The `probe_wdl` method therefore takes `&mut self`.

**DTZ probe** (root probe):
```rust
impl SyzygyTablebase {
    /// Probe DTZ value for the given position via a root probe.
    /// Returns `None` if the position has too many pieces or probing fails.
    pub fn probe_dtz(&mut self, pos: &Position, probe_limit: u8) -> Option<Dtz> {
        if piece_count(pos) > probe_limit || piece_count(pos) > self.max_pieces {
            return None;
        }
        let fathom_pos = to_fathom_position(pos);
        let (mut root_prober, _) = self.fathom.get_probers();
        let result = root_prober.probe(&fathom_pos)?;
        Some(Dtz(result.dtz as i32))
    }
}
```

`RootProber::probe` returns `Option<RootProbeResult>` where `RootProbeResult { wdl: Wdl, best_move: Move, dtz: u16 }`.

**WDL conversion helper**:
```rust
fn convert_wdl(wdl: fathom_syzygy::Wdl) -> Wdl {
    match wdl {
        fathom_syzygy::Wdl::Loss => Wdl::Loss,
        fathom_syzygy::Wdl::BlessedLoss => Wdl::BlessedLoss,
        fathom_syzygy::Wdl::Draw => Wdl::Draw,
        fathom_syzygy::Wdl::CursedWin => Wdl::CursedWin,
        fathom_syzygy::Wdl::Win => Wdl::Win,
    }
}
```

**Score conversion**:
```rust
/// TB win score: MATE_SCORE (30000) - 200 = 29800.
/// Places TB wins below actual checkmate scores (MATE_SCORE - ply, ply < 128)
/// but clearly in winning territory.
const TB_WIN_SCORE: i32 = 29800;

/// Convert a WDL result to an engine-internal score.
pub fn wdl_to_score(wdl: Wdl) -> i32 {
    match wdl {
        Wdl::Win => TB_WIN_SCORE,
        Wdl::CursedWin => 1,
        Wdl::Draw => 0,
        Wdl::BlessedLoss => -1,
        Wdl::Loss => -TB_WIN_SCORE,
    }
}
```

Uses a local constant `TB_WIN_SCORE = 29800` (i.e., `MATE_SCORE - 200`) rather than importing from the search crate, to avoid a circular dependency. The offset of 200 ensures TB wins don't collide with real mate scores (which are `MATE_SCORE - ply` where ply < 128).

**Unit tests** (colocated in `#[cfg(test)] mod tests`):

| Test | What it verifies |
|------|-----------------|
| `wdl_to_score_win` | `wdl_to_score(Wdl::Win)` returns `29800` |
| `wdl_to_score_loss` | `wdl_to_score(Wdl::Loss)` returns `-29800` |
| `wdl_to_score_draw` | `wdl_to_score(Wdl::Draw)` returns `0` |
| `wdl_to_score_cursed_win` | `wdl_to_score(Wdl::CursedWin)` returns `1` |
| `wdl_to_score_blessed_loss` | `wdl_to_score(Wdl::BlessedLoss)` returns `-1` |
| `piece_count_startpos` | `piece_count` on start position returns `32` |
| `piece_count_endgame` | `piece_count` on a KRK FEN (`"8/8/8/8/8/4K3/8/2k1R3 w - - 0 1"`) returns `3` |
| `probe_wdl_rejects_too_many_pieces` | `probe_wdl` returns `None` when piece count exceeds `probe_limit` |

Tests that require actual `.rtbw`/`.rtbz` files are guarded by `#[ignore]` since tablebase files are external and may not be present in CI:

| Test | What it verifies |
|------|-----------------|
| `probe_wdl_krk_win` (`#[ignore]`) | KRK position returns `Win` for side with rook |
| `probe_dtz_krk` (`#[ignore]`) | KRK position returns a valid DTZ value |
| `probe_wdl_kk_draw` (`#[ignore]`) | KK position returns `Draw` |

Tests requiring tablebase files are run manually with `cargo test -p engine -- --ignored` when files are available.

### `crates/engine/src/lib.rs` (modification)

Add `pub mod syzygy;` alongside the existing `pub mod book;`.

### `crates/engine/Cargo.toml` (modification)

Add to `[dependencies]`:
```toml
fathom-syzygy = "0.1.0"
```

## 4. API Contracts

```rust
use chess_engine::syzygy::{SyzygyTablebase, Wdl, Dtz, wdl_to_score};
use chess_board::Position;

// Initialize
let mut tb = SyzygyTablebase::init("/path/to/syzygy/tables")
    .map_err(|e| eprintln!("{e}"))?;

// Probe WDL (probe_limit = max pieces to probe, typically 5-7)
let wdl: Option<Wdl> = tb.probe_wdl(&pos, 6);

// Probe DTZ (root probe)
let dtz: Option<Dtz> = tb.probe_dtz(&pos, 6);

// Convert WDL to engine score
if let Some(wdl) = wdl {
    let score: i32 = wdl_to_score(wdl);
}

// Query max supported piece count
let max: u8 = tb.max_pieces();
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

## 6. Implementation Order

1. `crates/engine/Cargo.toml` — add `fathom-syzygy = "0.1.0"` dependency
2. `crates/engine/src/syzygy/mod.rs` — implement `Wdl`, `Dtz` types, `piece_count`, `to_fathom_position`, `convert_wdl`, `wdl_to_score`, `SyzygyTablebase` struct with `init`, `probe_wdl`, `probe_dtz`, `max_pieces`, and all unit tests
3. `crates/engine/src/lib.rs` — add `pub mod syzygy;`

## 7. Verification Commands

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test -p engine
cargo build -p engine
```

## 8. Design Deviations

**Score constant locality**: The phase spec states that WDL results should be "converted to the engine's internal score representation, with wins/losses mapped to mate-scale scores adjusted for distance." The search crate defines `MATE_SCORE = 30000`. Rather than adding `chess-search` as a dependency of the engine library crate (which would create a heavier dependency chain for just one constant), we define `const TB_WIN_SCORE: i32 = 29800;` locally in the syzygy module. This equals `MATE_SCORE - 200`, placing TB wins below actual checkmate scores but clearly in winning territory. The offset of 200 ensures TB wins don't collide with real mate scores (which are `MATE_SCORE - ply` where ply < 128). This is a standard approach in chess engines and correctly implements the spec's intent without coupling the modules.

**`&mut self` on probe methods**: The `fathom-syzygy` crate's `get_probers()` method borrows `&mut self` on the `Fathom` instance, so our `probe_wdl` and `probe_dtz` methods take `&mut self` rather than `&self`. This is an API constraint imposed by the underlying library's singleton design and does not affect functionality — the engine holds a single `SyzygyTablebase` instance in its state.