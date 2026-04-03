Now I have all the information needed to write the revised plan. The key feedback points are:

1. **Format**: Remove stray preamble line
2. **Correctness (castling)**: Fathom requires castling=0 (non-zero causes failure). Must document this and pass 0.
3. **Correctness (ep)**: Fathom uses 0 for "no en passant" — but square index 0 is a1. Need to verify this is correct for Fathom (it is — Fathom C API uses 0 for no ep).
4. **Correctness (turn)**: Fathom uses `true` for white, `false` for black. Must verify against `fathom-syzygy` Rust wrapper.
5. **Completeness**: Acknowledge that `fathom-syzygy` API is assumed and specify verification/fallback steps.

## 1. Deliverables

- `crates/engine/src/syzygy/mod.rs` — Syzygy tablebase probing module: WDL/DTZ result types, score conversion, initialization, probe interface wrapping the `fathom-syzygy` crate, and piece count gating.
- `crates/engine/src/lib.rs` — Modified to add `pub mod syzygy;` declaration.
- `crates/engine/Cargo.toml` — Modified to add `fathom-syzygy` dependency.

## 2. Dependencies

- `fathom-syzygy = "0.1.0"` (external, on crates.io) — Rust wrapper around the Fathom C library for Syzygy tablebase probing. The companion `-sys` crate (`fathom-syzygy-sys`) compiles the Fathom C source via `cc` internally. Requires a C compiler at build time.
- `chess-board` (already a dependency of engine crate) — provides `Position`.
- `chess-types` (already a dependency of engine crate) — provides `Bitboard`, `Color`, `Piece`, `PieceKind`, `Square`.
- `chess-search` (already a dependency of engine crate) — provides `MATE_SCORE` constant.
- External: Syzygy `.rtbw` (WDL) and `.rtbz` (DTZ) tablebase files (3–5 piece minimum) for testing.

**API assumption caveat**: The plan assumes the `fathom-syzygy` crate exposes `Fathom::new(path)`, `fathom.max_pieces()`, `fathom.get_probers() -> (RootProber, Prober)`, `Prober::probe(&fathom_syzygy::Position) -> Option<Wdl>`, `RootProber::probe(&fathom_syzygy::Position) -> Option<RootProbeResult>`, and a `fathom_syzygy::Position` struct with raw `u64` bitboard fields. These signatures are based on the crate's docs.rs documentation. The first implementation step is to add the dependency and run `cargo doc -p fathom-syzygy --open` to confirm the exact API surface before writing any code. If the API differs materially, adapt the wrapper accordingly — the public interface of our `SyzygyTablebase` wrapper remains identical regardless. If `fathom-syzygy` fails to build (e.g., C compilation issues on macOS ARM), switch to `pyrrhic-rs = "0.2.0"` as a fallback; `pyrrhic-rs` requires implementing an `EngineAdapter` trait for attack generation instead of constructing `Position` structs, but our module's public API stays the same.

## 3. Implementation Details

### `crates/engine/src/syzygy/mod.rs` (new file)

**Imports**: `chess_board::Position`, `chess_types::{Color, Piece, PieceKind}`, `chess_search::MATE_SCORE`.

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

The underlying Fathom C library's `tb_probe_wdl` signature is: `tb_probe_wdl(white, black, kings, queens, rooks, bishops, knights, pawns, rule50, castling, ep, turn)`. The `fathom-syzygy` Rust crate wraps this via a `fathom_syzygy::Position` struct.

Key parameter conventions from the Fathom C API:
- **`castling`**: Must be `0`. Passing non-zero values causes `tb_probe_wdl` and `tb_probe_root` to immediately return `TB_RESULT_FAILED`. Syzygy tablebases do not include positions with castling rights, so this field is always zero.
- **`ep`**: The en passant target square index (a1=0, h8=63). A value of `0` means "no en passant available." This is unambiguous because a1 can never be a valid en passant target square (en passant targets are always on rank 3 or rank 6).
- **`turn`**: `true` (or `1`) for white to move, `false` (or `0`) for black to move.

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
        // Fathom requires castling = 0; non-zero causes TB_RESULT_FAILED.
        // Syzygy tablebases do not cover positions with castling rights.
        castling: 0,
        // Fathom uses 0 for "no en passant." This is unambiguous because a1
        // (index 0) is never a valid en passant target square.
        ep: pos.en_passant().map(|sq| sq.index() as u32).unwrap_or(0),
        // Fathom C API: true = white to move, false = black to move.
        // Our Color enum: White = 0, Black = 1.
        turn: pos.side_to_move() == Color::White,
    }
}
```

**Changes from previous plan**:
1. `castling` is now hardcoded to `0` instead of `pos.castling_rights().inner() as u32`. Fathom mandates castling = 0 because Syzygy tablebases do not include castling positions. Passing non-zero causes immediate failure.
2. `ep` uses `unwrap_or(0)` which is correct — `0` is the Fathom sentinel for "no en passant." Square a1 (index 0) can never be a valid en passant target square (en passant targets are always on rank 3 or 6), so this is unambiguous.
3. `turn` now uses a boolean expression `pos.side_to_move() == Color::White` instead of integer mapping. This produces `true` for white (matching Fathom's `turn=true` = white) and `false` for black. The previous plan incorrectly mapped White→1 and Black→0 as integers, which depended on the Rust wrapper's convention; using a `bool` directly matches the Fathom C API's `bool turn` parameter.

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
/// TB win score: MATE_SCORE - 200 = 29800.
/// Places TB wins below actual checkmate scores (MATE_SCORE - ply, ply < MAX_PLY = 128)
/// but clearly in winning territory.
const TB_WIN_SCORE: i32 = MATE_SCORE - 200;

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

`TB_WIN_SCORE` is defined as `MATE_SCORE - 200` using the imported `chess_search::MATE_SCORE` constant (currently `30000`), producing `29800`. Since the engine crate already depends on `chess-search`, no new dependency is introduced. The offset of 200 ensures TB wins don't collide with real mate scores (which are `MATE_SCORE - ply` where ply < `MAX_PLY` = 128).

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
2. Run `cargo doc -p fathom-syzygy --open` to verify the crate's actual API surface matches the assumptions in this plan. If it differs, adapt the wrapper code accordingly (or switch to `pyrrhic-rs` fallback).
3. `crates/engine/src/syzygy/mod.rs` — implement `Wdl`, `Dtz` types, `piece_count`, `to_fathom_position`, `convert_wdl`, `wdl_to_score`, `SyzygyTablebase` struct with `init`, `probe_wdl`, `probe_dtz`, `max_pieces`, and all unit tests
4. `crates/engine/src/lib.rs` — add `pub mod syzygy;`

## 7. Verification Commands

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test -p engine
cargo build -p engine
```

## 8. Design Deviations

**Castling field hardcoded to zero**: The engine's `Position` tracks castling rights via a 4-bit `CastlingRights(u8)` (WK=0b0001, WQ=0b0010, BK=0b0100, BQ=0b1000). The previous plan passed `pos.castling_rights().inner()` to Fathom. However, the Fathom C API mandates `castling = 0` — passing any non-zero value causes `tb_probe_wdl` and `tb_probe_root` to immediately return `TB_RESULT_FAILED`. This is by design: Syzygy tablebases do not index positions with castling rights (castling requires rooks and king on starting squares, which is already encoded by piece placement). The revised plan hardcodes `castling: 0` in `to_fathom_position`.

**Score constant sourced from search crate**: The previous plan defined `TB_WIN_SCORE = 29800` as a local magic number to avoid a dependency on `chess-search`. Since the engine crate already depends on `chess-search` (confirmed in `Cargo.toml`), the revised plan imports `chess_search::MATE_SCORE` and defines `TB_WIN_SCORE = MATE_SCORE - 200`. This keeps the constant tied to its source of truth.

**`&mut self` on probe methods**: The `fathom-syzygy` crate's `get_probers()` method borrows `&mut self` on the `Fathom` instance, so our `probe_wdl` and `probe_dtz` methods take `&mut self` rather than `&self`. This is an API constraint imposed by the underlying library's singleton design and does not affect functionality — the engine holds a single `SyzygyTablebase` instance in its state.