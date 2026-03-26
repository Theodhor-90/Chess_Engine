## Phase P01 — Primitive Types

### Goal

Define the core value types that the entire chess engine operates on. This phase produces the `types` crate containing `Bitboard`, chess enums (`Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank`), and the `Move` type. These primitives are the substrate that every subsequent phase and milestone builds upon — correctness and ergonomic APIs here are critical.

### Deliverables

- **T01 — Bitboard**: `Bitboard` newtype (`struct Bitboard(u64)`) with `BitAnd`, `BitOr`, `BitXor`, `Not`, `Shl`, `Shr` operator overloads, bit iteration (iterate over set bit indices), population count, and LSB/MSB extraction.
- **T02 — Chess Enums**: `Color`, `PieceKind`, `Piece`, `Square`, `File`, `Rank` enums with conversion traits (e.g., `From`/`Into` between `Square` and `(File, Rank)`) and safe indexing. Square uses little-endian rank-file mapping (a1 = 0, h8 = 63).
- **T03 — Move Encoding**: `Move` as a packed `u16` with encode/decode methods. Bit layout: `from(6) | to(6) | flags(4)`. Flags encode promotion piece, castling, en passant, and double pawn push.

### Technical Constraints

- Language: Rust 2021 edition, stable toolchain
- Workspace structure: this phase produces the `crates/types/` crate
- `Bitboard` must be a newtype wrapper `struct Bitboard(u64)` with operator overloads (not a type alias)
- Move encoding uses a 16-bit packed integer: `from(6) | to(6) | flags(4)`
- Square indexing uses little-endian rank-file mapping (a1 = 0, h8 = 63)
- No `unsafe` unless provably necessary (with `// SAFETY:` comment)
- No `unwrap()` in library crates — use `Result`/`Option` propagation
- Naming: `snake_case` functions/variables, `PascalCase` types, `SCREAMING_SNAKE` constants
- All public items must have `///` doc comments
- Must pass `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`
- Minimize external dependencies — core types should depend only on the standard library
- Error handling: `thiserror` for library errors

### Dependencies

- No prior phases required (this is the first phase of the first milestone)
- Rust stable toolchain must be installed