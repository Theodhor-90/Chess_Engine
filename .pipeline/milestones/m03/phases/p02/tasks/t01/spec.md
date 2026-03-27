# T01 — Negamax Search with Alpha-Beta Pruning

## Objective

Create the `search` crate (`crates/search/`) and implement the core negamax search function with alpha-beta pruning. This is the foundational search algorithm that the engine uses to select moves.

## Deliverables

- `crates/search/Cargo.toml` — new crate manifest with dependencies on `types`, `board`, `movegen`, `eval`
- `crates/search/src/lib.rs` — negamax function, score constants, and unit tests
- `Cargo.toml` (workspace root) — updated to include `crates/search` as a workspace member

## Implementation Details

### Crate Setup

- Create `crates/search/` as a library crate following the project's workspace structure
- Add dependencies on `types`, `board`, `movegen`, and `eval` crates
- Register `crates/search` in the workspace root `Cargo.toml`

### Negamax Function

- **Signature**: Takes `&mut Position`, depth (`u8`), alpha (`i32`), beta (`i32`), and ply (`u8`). Returns `(i32, Option<Move>)`.
- **Convention**: Scores are returned from the perspective of the side to move. Negation at each recursive level implements the minimax property.
- **At depth 0**: Call `eval::evaluate` directly as a placeholder (quiescence search will replace this in T02).
- **Legal move generation**: Use `movegen::generate_legal_moves` to get all legal moves.
- **Checkmate detection**: No legal moves AND king is in check (via `pos.is_square_attacked(king_square, opponent_color)`) → return `-MATE_SCORE + ply`.
- **Stalemate detection**: No legal moves AND king is NOT in check → return `0` (draw).
- **Move loop**: For each legal move, call `pos.make_move(mv)` to get `UndoInfo`, recursively search with `(-beta, -alpha)` and `depth - 1`, then call `pos.unmake_move(mv, undo)`. Negate the returned score.
- **Alpha-beta pruning**: Update alpha and best move when score improves. Prune (return beta) when `score >= beta` (beta cutoff).

### Score Constants

- `MATE_SCORE: i32 = 30000` — adjusted by ply distance so the search prefers shorter mates
- `INFINITY: i32 = 31000` — used for initial alpha/beta bounds

### Score Type

- Use `i32` for all scores, consistent with the `eval` crate's return type

### Check Detection

- Use `pos.is_square_attacked(king_square, opponent_color)` to determine if the side to move is in check. There is no dedicated `is_in_check` method on `Position`.

## Dependencies

- **M01 types crate**: `Move`, `Square`, `Piece`, `PieceKind`, `Color`, `Bitboard`, `MoveFlag`
- **M01 board crate**: `Position` with `make_move(&mut self, mv: Move) -> UndoInfo`, `unmake_move(&mut self, mv: Move, undo: UndoInfo)`, `is_square_attacked(&self, square: Square, by_side: Color) -> bool`, `side_to_move(&self) -> Color`, FEN parsing
- **M02 movegen crate**: `generate_legal_moves(pos: &mut Position) -> Vec<Move>`
- **M03/P01 eval crate**: `evaluate(pos: &Position) -> i32`
- No dependencies on other tasks in this phase (this is the first task)

## Verification Criteria

1. The `search` crate compiles and is registered as a workspace member
2. Negamax correctly detects checkmate positions and returns `-MATE_SCORE + ply`
3. Negamax correctly detects stalemate positions and returns `0`
4. At depth > 0, the search returns a legal move (not `None`) when legal moves exist
5. Alpha-beta pruning reduces node count compared to a brute-force minimax (verifiable by comparing node counts)
6. Unit tests verify mate/stalemate detection and basic search behavior
7. All code passes `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`