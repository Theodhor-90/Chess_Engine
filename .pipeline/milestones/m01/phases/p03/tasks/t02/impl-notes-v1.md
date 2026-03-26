All verification commands pass. Here's the summary:

- **`crates/board/src/position.rs`** — modified:
  - Added `pub fn unmake_move(&mut self, mv: Move, undo: UndoInfo)` method (steps: toggle side, handle piece restoration by move type, decrement fullmove for Black, restore castling/EP/halfmove/hash from UndoInfo, recompute occupancy)
  - Removed `#[allow(dead_code)]` from `set_castling_rights`, `set_en_passant`, `set_halfmove_clock` (now used by `unmake_move`)
  - Added `assert_round_trip` test helper
  - Added 16 tests: `unmake_quiet`, `unmake_capture`, `unmake_double_pawn_push`, `unmake_white_kingside_castle`, `unmake_white_queenside_castle`, `unmake_black_kingside_castle`, `unmake_black_queenside_castle`, `unmake_en_passant`, `unmake_promotion_knight`, `unmake_promotion_bishop`, `unmake_promotion_rook`, `unmake_promotion_queen`, `unmake_promotion_capture`, `unmake_fullmove_counter`, `unmake_preserves_ep_state`, `unmake_multiple_sequential`