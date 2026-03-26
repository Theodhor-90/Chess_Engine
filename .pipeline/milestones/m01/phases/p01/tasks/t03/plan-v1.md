Now I have a complete understanding of the codebase. Let me produce the implementation plan.

## 1. Deliverables

| File | Purpose |
|------|---------|
| `crates/types/src/chess_move.rs` | `Move` newtype wrapping `u16`, `MoveFlag` constants, encode/decode methods, `Display` impl producing UCI notation, and colocated unit tests |
| `crates/types/src/lib.rs` | Modified to declare the `chess_move` module and re-export `Move` and `MoveFlag` |

## 2. Dependencies

No new dependencies. The `Move` type depends only on the standard library and the existing `Square` and `PieceKind` types from sibling modules within the `types` crate.

## 3. Implementation Details

### `crates/types/src/chess_move.rs`

**Purpose:** Implement the packed 16-bit `Move` type with encode/decode for all chess move types.

**Bit layout (16 bits total):**
```
bits 15..10: from square (6 bits, 0..63)
bits  9.. 4: to square   (6 bits, 0..63)
bits  3.. 0: flags       (4 bits, 0..15)
```

Extraction uses shift-and-mask:
- `from = (raw >> 10) & 0x3F`
- `to = (raw >> 4) & 0x3F`
- `flags = raw & 0xF`

Encoding:
- `raw = (from << 10) | (to << 4) | flags`

**`MoveFlag` — associated constants on a unit struct (or a dedicated type):**

Following the standard Chess Programming Wiki / Stockfish 4-bit flag convention:

| Value | Name | Description |
|-------|------|-------------|
| 0 | `QUIET` | Normal quiet move |
| 1 | `DOUBLE_PAWN_PUSH` | Pawn advances two ranks |
| 2 | `KING_CASTLE` | King-side castling (O-O) |
| 3 | `QUEEN_CASTLE` | Queen-side castling (O-O-O) |
| 4 | `CAPTURE` | Normal capture |
| 5 | `EN_PASSANT` | En passant capture |
| 6 | (unused) | Reserved |
| 7 | (unused) | Reserved |
| 8 | `KNIGHT_PROMOTION` | Promote to knight |
| 9 | `BISHOP_PROMOTION` | Promote to bishop |
| 10 | `ROOK_PROMOTION` | Promote to rook |
| 11 | `QUEEN_PROMOTION` | Promote to queen |
| 12 | `KNIGHT_PROMOTION_CAPTURE` | Promote to knight + capture |
| 13 | `BISHOP_PROMOTION_CAPTURE` | Promote to bishop + capture |
| 14 | `ROOK_PROMOTION_CAPTURE` | Promote to rook + capture |
| 15 | `QUEEN_PROMOTION_CAPTURE` | Promote to queen + capture |

**Exports:**

- `Move` — newtype struct wrapping `u16`, derives `Clone, Copy, PartialEq, Eq, Hash, Debug`
- `MoveFlag` — struct with associated `u16` constants for all 16 flag values

**Public methods on `Move`:**

- `Move::new(from: Square, to: Square, flags: u16) -> Move` — encodes a move from components
- `Move::from_raw(raw: u16) -> Move` — wraps a raw `u16` directly
- `Move::raw(self) -> u16` — returns the underlying `u16`
- `Move::from_sq(self) -> Square` — extracts the source square
- `Move::to_sq(self) -> Square` — extracts the destination square
- `Move::flags(self) -> u16` — extracts the 4-bit flags
- `Move::is_capture(self) -> bool` — returns true if the flags indicate a capture (flag == 4, 5, or 12..15)
- `Move::is_promotion(self) -> bool` — returns true if the flags indicate a promotion (flag 8..15)
- `Move::promotion_piece(self) -> Option<PieceKind>` — returns the promotion piece kind if this is a promotion move (Knight for 8/12, Bishop for 9/13, Rook for 10/14, Queen for 11/15)
- `Move::is_castling(self) -> bool` — returns true for king-side or queen-side castling (flag 2 or 3)
- `Move::is_en_passant(self) -> bool` — returns true for en passant (flag 5)
- `Move::is_double_pawn_push(self) -> bool` — returns true for double pawn push (flag 1)

**`Display` implementation:**

Produces UCI long-algebraic notation:
- Normal moves: `"{from}{to}"` e.g. `e2e4`
- Promotion moves: `"{from}{to}{promo}"` where `promo` is one of `n`, `b`, `r`, `q` (lowercase), e.g. `e7e8q`

Uses the existing `Display` impls on `Square` (which outputs `"e2"`, `"e4"`, etc.) and maps `PieceKind` to the lowercase letter for the promotion suffix.

**`Move::NONE` constant:** A sentinel value (`Move(0)`) representing "no move" — useful as a default/placeholder. Since from=0, to=0, flags=0 this represents a1→a1 quiet which is never a legal chess move.

### `crates/types/src/lib.rs`

Add `pub mod chess_move;` declaration and re-export:
```rust
pub use chess_move::{Move, MoveFlag};
```

## 4. API Contracts

**Encoding:**
```rust
let m = Move::new(Square::E2, Square::E4, MoveFlag::DOUBLE_PAWN_PUSH);
assert_eq!(m.from_sq(), Square::E2);
assert_eq!(m.to_sq(), Square::E4);
assert_eq!(m.flags(), MoveFlag::DOUBLE_PAWN_PUSH);
```

**Promotion:**
```rust
let m = Move::new(Square::E7, Square::E8, MoveFlag::QUEEN_PROMOTION);
assert_eq!(m.promotion_piece(), Some(PieceKind::Queen));
assert!(m.is_promotion());
assert!(!m.is_capture());
assert_eq!(format!("{m}"), "e7e8q");
```

**Promotion with capture:**
```rust
let m = Move::new(Square::E7, Square::D8, MoveFlag::QUEEN_PROMOTION_CAPTURE);
assert_eq!(m.promotion_piece(), Some(PieceKind::Queen));
assert!(m.is_promotion());
assert!(m.is_capture());
assert_eq!(format!("{m}"), "e7d8q");
```

**Castling:**
```rust
let m = Move::new(Square::E1, Square::G1, MoveFlag::KING_CASTLE);
assert!(m.is_castling());
assert_eq!(format!("{m}"), "e1g1");
```

**Non-promotion display:**
```rust
let m = Move::new(Square::E2, Square::E4, MoveFlag::QUIET);
assert_eq!(format!("{m}"), "e2e4");
```

## 5. Test Plan

All tests are colocated in `crates/types/src/chess_move.rs` inside a `#[cfg(test)] mod tests` block, following the pattern established by sibling modules.

**Tests (mapped to verification criteria from the task spec):**

1. **`test_encode_decode_quiet`** — Encode a quiet move (e.g., e2→e4 with `QUIET`), decode and verify `from_sq`, `to_sq`, and `flags` match.

2. **`test_encode_decode_double_pawn_push`** — Encode a double pawn push, verify flag extraction.

3. **`test_encode_decode_king_castle`** — Encode king-side castling, verify flag and `is_castling()`.

4. **`test_encode_decode_queen_castle`** — Encode queen-side castling, verify flag and `is_castling()`.

5. **`test_encode_decode_capture`** — Encode a capture, verify `is_capture()`.

6. **`test_encode_decode_en_passant`** — Encode an en passant capture, verify `is_en_passant()` and `is_capture()`.

7. **`test_encode_decode_promotions`** — Loop over all 4 promotion flags (knight, bishop, rook, queen), encode each, verify `is_promotion()`, `promotion_piece()`, and that `is_capture()` is false.

8. **`test_encode_decode_promotion_captures`** — Loop over all 4 promotion-capture flags, encode each, verify `is_promotion()`, `is_capture()`, and `promotion_piece()`.

9. **`test_boundary_squares`** — Encode a move from `Square::A1` (index 0) to `Square::H8` (index 63) and vice versa, verify round-trip.

10. **`test_all_flags_distinct`** — Verify all 16 `MoveFlag` constant values are distinct (0..15).

11. **`test_display_quiet`** — Verify `Display` for a quiet move produces `"e2e4"`.

12. **`test_display_promotion`** — Verify `Display` for each promotion type produces the correct UCI string with suffix (`n`, `b`, `r`, `q`).

13. **`test_display_promotion_capture`** — Verify `Display` for a promotion capture includes the promotion suffix.

## 6. Implementation Order

1. **Create `crates/types/src/chess_move.rs`** — Define `MoveFlag` with all 16 constants, then the `Move` struct with `new`, `from_raw`, `raw`, `from_sq`, `to_sq`, `flags` methods.
2. **Add query methods** — `is_capture`, `is_promotion`, `promotion_piece`, `is_castling`, `is_en_passant`, `is_double_pawn_push`.
3. **Implement `Display`** — UCI long-algebraic notation with promotion suffix.
4. **Add `Move::NONE` constant.**
5. **Write all unit tests** in the `#[cfg(test)] mod tests` block.
6. **Modify `crates/types/src/lib.rs`** — Add `pub mod chess_move;` and `pub use chess_move::{Move, MoveFlag};`.
7. **Run verification commands.**

## 7. Verification Commands

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test -p chess-types
```

## 8. Design Deviations

**Deviation 1: File name `chess_move.rs` instead of `moves.rs`**

- The task spec suggests "`moves.rs` (or `chess_move.rs` to avoid keyword conflicts)" — we choose `chess_move.rs` because `move` is a Rust keyword and using `chess_move` as the module name avoids any confusion or potential issues with `r#move` syntax. The task spec explicitly offers this as an option.

**Deviation 2: `MoveFlag` as a struct with associated `u16` constants instead of an enum**

- The task spec says "A `MoveFlags` enum or constants for all 16 flag combinations." We use a struct with associated constants (e.g., `MoveFlag::QUIET`, `MoveFlag::CAPTURE`) rather than an enum because: (a) two of the 16 values (6, 7) are unused/reserved, which makes an enum unclean — you'd need meaningless variant names; (b) flag values need to participate in bitwise operations and comparisons with `u16`, and associated constants avoid the need for `as u16` casts everywhere; (c) the task spec explicitly offers "enum or constants" as alternatives.