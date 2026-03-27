Now I have all the context I need. Let me produce the complete revised plan.

## Task T02 — UCI Output: Implementation Plan (v2)

### 1. Deliverables

| # | File | Action | Description |
|---|------|--------|-------------|
| 1 | `crates/uci/src/lib.rs` | Modify | Add `pub mod output;` declaration |
| 2 | `crates/uci/src/output.rs` | Create | UCI output formatting types and functions |
| 3 | `crates/uci/Cargo.toml` | Modify | Add `chess-types` dependency (needed for `Move` in `bestmove`/`pv`) |

### 2. Dependencies

| Dependency | Crate | What it provides |
|------------|-------|-----------------|
| `chess-types` | `crates/types` | `Move` type with `Display` impl producing UCI coordinate notation (e.g., `e2e4`, `a7a8q`) |
| `thiserror` | external | Already present in `crates/uci/Cargo.toml` — no change needed |

No runtime dependencies beyond these. The output module produces `String` values with no I/O — the engine binary (T03) handles actual stdout writing.

### 3. Implementation Details

All code goes in `crates/uci/src/output.rs`. The module contains:

#### 3.1 `Score` enum

Represents the two UCI score formats:

```rust
use chess_types::Move;

/// UCI score representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Score {
    /// Centipawn score.
    Cp(i32),
    /// Mate in N moves (positive = engine mates, negative = engine is mated).
    Mate(i32),
}
```

#### 3.2 `InfoFields` struct

A struct with all-optional fields for constructing `info` lines. Uses a builder pattern to set fields, then `to_uci_string()` to produce the formatted output.

```rust
/// Builder for UCI `info` output lines.
#[derive(Debug, Clone, Default)]
pub struct InfoFields {
    pub depth: Option<u32>,
    pub score: Option<Score>,
    pub nodes: Option<u64>,
    pub nps: Option<u64>,
    pub time: Option<u64>,
    pub pv: Vec<Move>,
}

impl InfoFields {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn depth(mut self, depth: u32) -> Self {
        self.depth = Some(depth);
        self
    }

    pub fn score(mut self, score: Score) -> Self {
        self.score = Some(score);
        self
    }

    pub fn nodes(mut self, nodes: u64) -> Self {
        self.nodes = Some(nodes);
        self
    }

    pub fn nps(mut self, nps: u64) -> Self {
        self.nps = Some(nps);
        self
    }

    pub fn time(mut self, time: u64) -> Self {
        self.time = Some(time);
        self
    }

    pub fn pv(mut self, pv: Vec<Move>) -> Self {
        self.pv = pv;
        self
    }

    /// Formats the info line. Returns `None` if no fields are set.
    pub fn to_uci_string(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();

        if let Some(d) = self.depth {
            parts.push(format!("depth {d}"));
        }
        if let Some(ref s) = self.score {
            match s {
                Score::Cp(cp) => parts.push(format!("score cp {cp}")),
                Score::Mate(m) => parts.push(format!("score mate {m}")),
            }
        }
        if let Some(n) = self.nodes {
            parts.push(format!("nodes {n}"));
        }
        if let Some(n) = self.nps {
            parts.push(format!("nps {n}"));
        }
        if let Some(t) = self.time {
            parts.push(format!("time {t}"));
        }
        if !self.pv.is_empty() {
            let pv_str: Vec<String> = self.pv.iter().map(|m| m.to_string()).collect();
            parts.push(format!("pv {}", pv_str.join(" ")));
        }

        if parts.is_empty() {
            return None;
        }

        Some(format!("info {}", parts.join(" ")))
    }
}
```

#### 3.3 Free functions

Simple functions that return formatted UCI strings:

```rust
/// Returns `"id name <name>"`.
pub fn id_name(name: &str) -> String {
    format!("id name {name}")
}

/// Returns `"id author <author>"`.
pub fn id_author(author: &str) -> String {
    format!("id author {author}")
}

/// Returns `"uciok"`.
pub fn uciok() -> String {
    "uciok".to_string()
}

/// Returns `"readyok"`.
pub fn readyok() -> String {
    "readyok".to_string()
}

/// Returns `"bestmove <move>"` or `"bestmove <move> ponder <move>"`.
pub fn bestmove(mv: Move, ponder: Option<Move>) -> String {
    match ponder {
        Some(p) => format!("bestmove {mv} ponder {p}"),
        None => format!("bestmove {mv}"),
    }
}
```

All functions produce `String` — no I/O. Moves are formatted via `Move`'s `Display` impl which produces UCI coordinate notation (e.g., `e2e4`, `a7a8q`).

### 4. API Contracts

#### Public types exported from `crates::uci::output`

| Item | Kind | Signature |
|------|------|-----------|
| `Score` | enum | `Cp(i32)`, `Mate(i32)` |
| `InfoFields` | struct | All fields `pub`, implements builder methods, `to_uci_string() -> Option<String>` |
| `id_name` | fn | `fn id_name(name: &str) -> String` |
| `id_author` | fn | `fn id_author(author: &str) -> String` |
| `uciok` | fn | `fn uciok() -> String` |
| `readyok` | fn | `fn readyok() -> String` |
| `bestmove` | fn | `fn bestmove(mv: Move, ponder: Option<Move>) -> String` |

#### Output format contracts

| Function | Example output |
|----------|---------------|
| `id_name("MyEngine")` | `id name MyEngine` |
| `id_author("Author")` | `id author Author` |
| `uciok()` | `uciok` |
| `readyok()` | `readyok` |
| `bestmove(e2e4, None)` | `bestmove e2e4` |
| `bestmove(e2e4, Some(e7e5))` | `bestmove e2e4 ponder e7e5` |
| `InfoFields` depth only | `info depth 5` |
| `InfoFields` depth+score+nodes | `info depth 10 score cp 35 nodes 12345` |
| `InfoFields` full | `info depth 15 score mate 3 nodes 50000 nps 1000000 time 50 pv e2e4 e7e5 g1f3` |
| `InfoFields` no fields set | `None` (no output) |

Info field order is fixed: `depth`, `score`, `nodes`, `nps`, `time`, `pv`. This matches standard UCI conventions.

### 5. Test Plan

All tests go in `crates/uci/src/output.rs` inside a `#[cfg(test)] mod tests` block. Tests use `chess_types::{Move, Square, MoveFlag}` to construct moves.

| # | Test name | Verifies |
|---|-----------|----------|
| 1 | `test_id_name` | `id_name("Foo")` → `"id name Foo"` |
| 2 | `test_id_author` | `id_author("Bar")` → `"id author Bar"` |
| 3 | `test_uciok` | `uciok()` → `"uciok"` |
| 4 | `test_readyok` | `readyok()` → `"readyok"` |
| 5 | `test_bestmove_no_ponder` | `bestmove(Move::new(E2, E4, QUIET), None)` → `"bestmove e2e4"` |
| 6 | `test_bestmove_with_ponder` | `bestmove(Move::new(E2, E4, QUIET), Some(Move::new(E7, E5, QUIET)))` → `"bestmove e2e4 ponder e7e5"` |
| 7 | `test_bestmove_promotion` | `bestmove(Move::new(A7, A8, QUEEN_PROMOTION), None)` → `"bestmove a7a8q"` |
| 8 | `test_info_depth_only` | `InfoFields::new().depth(5).to_uci_string()` → `Some("info depth 5")` |
| 9 | `test_info_depth_score_nodes` | depth=10, score=Cp(35), nodes=12345 → `Some("info depth 10 score cp 35 nodes 12345")` |
| 10 | `test_info_score_cp_negative` | score=Cp(-150) → contains `"score cp -150"` |
| 11 | `test_info_score_mate_positive` | score=Mate(3) → contains `"score mate 3"` |
| 12 | `test_info_score_mate_negative` | score=Mate(-2) → contains `"score mate -2"` |
| 13 | `test_info_full_line` | All fields set including 3-move pv → `Some("info depth 15 score cp 100 nodes 50000 nps 1000000 time 50 pv e2e4 e7e5 g1f3")` |
| 14 | `test_info_pv_multiple_moves` | pv with 3 moves → `"pv e2e4 e7e5 d2d4"` substring present |
| 15 | `test_info_no_fields` | `InfoFields::new().to_uci_string()` → `None` |
| 16 | `test_info_nps_only` | `InfoFields::new().nps(500000).to_uci_string()` → `Some("info nps 500000")` |
| 17 | `test_info_time_only` | `InfoFields::new().time(1500).to_uci_string()` → `Some("info time 1500")` |

### 6. Implementation Order

| Step | Action | Details |
|------|--------|---------|
| 1 | Update `crates/uci/Cargo.toml` | Add `chess-types = { path = "../types" }` under `[dependencies]` |
| 2 | Create `crates/uci/src/output.rs` | Add `Score` enum, `InfoFields` struct with builder methods and `to_uci_string()`, free functions `id_name`, `id_author`, `uciok`, `readyok`, `bestmove`, and the `#[cfg(test)] mod tests` block with all 17 tests |
| 3 | Update `crates/uci/src/lib.rs` | Add `pub mod output;` at the top of the file (before the existing `UciError` definition) |
| 4 | Run verification commands | `cargo fmt -p chess-uci --check`, `cargo clippy -p chess-uci -- -D warnings`, `cargo test -p chess-uci`, `cargo check --workspace` |

### 7. Verification Commands

```bash
# Format check
cargo fmt -p chess-uci --check

# Lint check
cargo clippy -p chess-uci -- -D warnings

# Run all uci crate tests (includes both T01 parser tests and new T02 output tests)
cargo test -p chess-uci

# Verify the full workspace still compiles
cargo check --workspace
```

### 8. Design Deviations

None.