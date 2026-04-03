Now I have all the information needed to write the task plan. Here it is:

## 1. Deliverables

- **New**: `crates/uci/src/time.rs` — Time allocation module containing a pure function that computes a per-move `Duration` from UCI clock parameters and the side to move.
- **Modified**: `crates/uci/src/lib.rs` — Add `pub mod time;` declaration to expose the new module.
- **Modified**: `crates/uci/Cargo.toml` — Add `chess-types` dependency if not already present (already present — no change needed).

## 2. Dependencies

- `chess-types` (already a dependency of `chess-uci` — provides `Color` for side-to-move parameter)
- `std::time::Duration` (standard library)
- No new external dependencies required.

## 3. Implementation Details

### `crates/uci/src/time.rs`

**Purpose**: Provide a pure, unit-testable function that computes the per-move time budget from UCI clock parameters.

**Public API**:

```rust
use std::time::Duration;
use chess_types::Color;
use crate::GoParams;

/// Computes the time budget for the current move.
///
/// Formula: `time_left / moves_to_go + increment`
/// - `moves_to_go` defaults to 25 when `GoParams::movestogo` is `None`
/// - `time_left` is `wtime` for `Color::White`, `btime` for `Color::Black`
/// - `increment` is `winc` for `Color::White`, `binc` for `Color::Black`
pub fn allocate_time(params: &GoParams, side: Color) -> Duration
```

**Key logic**:

1. Extract `time_left` from `params.wtime` or `params.btime` based on `side`. If the relevant field is `None`, return a fallback of `Duration::from_secs(1)` (minimal safe budget when no time info is available for this side).
2. Extract `increment` from `params.winc` or `params.binc` based on `side`. Default to `0` if `None`.
3. Determine `moves_to_go`: use `params.movestogo` if `Some`, otherwise default to `25`.
4. Compute: `budget_ms = time_left_ms / moves_to_go as u64 + increment_ms`.
5. Clamp the budget to a minimum of 1 millisecond to avoid zero-duration budgets.
6. Return `Duration::from_millis(budget_ms)`.

**Inline tests** (`#[cfg(test)] mod tests`):

Six tests covering the verification criteria from the task spec (see Test Plan below).

### `crates/uci/src/lib.rs`

**Change**: Add one line near the top alongside the existing `pub mod output;`:

```rust
pub mod time;
```

No other changes.

## 4. API Contracts

```rust
use chess_uci::time::allocate_time;
use chess_uci::GoParams;
use chess_types::Color;
use std::time::Duration;

// Example: sudden death, 5 min remaining, white to move, no increment
let params = GoParams {
    wtime: Some(300_000),
    btime: Some(300_000),
    ..GoParams::default()
};
let budget = allocate_time(&params, Color::White);
// budget == Duration::from_millis(300_000 / 25 + 0) == Duration::from_millis(12_000)
```

N/A for REST/HTTP.

## 5. Test Plan

All tests are colocated in `crates/uci/src/time.rs` as `#[cfg(test)] mod tests`, following the project convention.

| Test | Inputs | Expected Budget | What it verifies |
|------|--------|----------------|------------------|
| `sudden_death_no_increment` | wtime=300000, btime=300000, no movestogo, no inc, White | 300000/25 = 12000ms | Correct default movestogo (25) with no increment |
| `sudden_death_with_increment` | wtime=300000, btime=300000, winc=3000, binc=3000, no movestogo, White | 300000/25 + 3000 = 15000ms | Increment is added to base allocation |
| `movestogo_with_increment` | wtime=120000, btime=120000, winc=5000, binc=5000, movestogo=20, White | 120000/20 + 5000 = 11000ms | Explicit movestogo overrides default of 25 |
| `movestogo_without_increment` | wtime=120000, btime=120000, movestogo=20, no inc, White | 120000/20 = 6000ms | movestogo works without increment |
| `very_low_time` | wtime=500, btime=800, winc=100, binc=100, no movestogo, Black | 800/25 + 100 = 132ms | Handles sub-second time remaining (Black side) |
| `zero_increment_no_movestogo` | wtime=300000, btime=300000, no movestogo, no inc, Black | 300000/25 = 12000ms | Pure sudden death from Black's perspective |

## 6. Implementation Order

1. `crates/uci/src/time.rs` — implement `allocate_time` function and all six unit tests
2. `crates/uci/src/lib.rs` — add `pub mod time;` declaration

## 7. Verification Commands

```bash
cargo test -p chess-uci
cargo clippy -p chess-uci -- -D warnings
cargo fmt --check
```

## 8. Design Deviations

None.