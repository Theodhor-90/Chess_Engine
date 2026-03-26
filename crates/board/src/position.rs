use chess_types::{Bitboard, Color, File, Move, MoveFlag, Piece, PieceKind, Square};

use crate::fen::{self, FenError};

/// Compact representation of the four castling flags as a `u8` bitfield.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct CastlingRights(u8);

impl CastlingRights {
    pub const WHITE_KINGSIDE: u8 = 0b0001;
    pub const WHITE_QUEENSIDE: u8 = 0b0010;
    pub const BLACK_KINGSIDE: u8 = 0b0100;
    pub const BLACK_QUEENSIDE: u8 = 0b1000;

    /// All four castling rights set.
    pub const ALL: CastlingRights = CastlingRights(0b1111);
    /// No castling rights set.
    pub const NONE: CastlingRights = CastlingRights(0);

    /// Check if a specific right is set.
    pub fn contains(self, flag: u8) -> bool {
        self.0 & flag != 0
    }

    /// Enable a right.
    pub fn set(&mut self, flag: u8) {
        self.0 |= flag;
    }

    /// Disable a right.
    pub fn remove(&mut self, flag: u8) {
        self.0 &= !flag;
    }

    /// Returns the raw `u8` bitfield value.
    pub fn inner(self) -> u8 {
        self.0
    }

    /// Clears rights not present in the mask (AND operation on the raw bitfield).
    pub fn mask(&mut self, mask: u8) {
        self.0 &= mask;
    }
}

/// AND-mask table indexed by square index for updating castling rights.
/// When a piece moves from or to a square, `castling_rights.mask(CASTLING_RIGHTS_MASK[sq])`.
const CASTLING_RIGHTS_MASK: [u8; 64] = {
    let mut table = [0b1111u8; 64];
    table[0] = 0b1101; // A1: clears WHITE_QUEENSIDE
    table[4] = 0b1100; // E1: clears both white rights
    table[7] = 0b1110; // H1: clears WHITE_KINGSIDE
    table[56] = 0b0111; // A8: clears BLACK_QUEENSIDE
    table[60] = 0b0011; // E8: clears both black rights
    table[63] = 0b1011; // H8: clears BLACK_KINGSIDE
    table
};

/// State saved by `make_move` that is needed by `unmake_move` to restore the position.
#[derive(Clone, Copy, Debug)]
pub struct UndoInfo {
    pub captured: Option<Piece>,
    pub castling_rights: CastlingRights,
    pub en_passant: Option<Square>,
    pub halfmove_clock: u8,
    pub hash: u64,
}

/// Complete state of a chess game.
#[derive(Clone, Debug)]
pub struct Position {
    piece_bb: [Bitboard; 12],
    occupied_by: [Bitboard; 2],
    occupied: Bitboard,
    side_to_move: Color,
    castling_rights: CastlingRights,
    en_passant: Option<Square>,
    halfmove_clock: u8,
    fullmove_counter: u16,
    hash: u64,
}

impl Position {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        piece_bb: [Bitboard; 12],
        occupied_by: [Bitboard; 2],
        occupied: Bitboard,
        side_to_move: Color,
        castling_rights: CastlingRights,
        en_passant: Option<Square>,
        halfmove_clock: u8,
        fullmove_counter: u16,
    ) -> Position {
        Position {
            piece_bb,
            occupied_by,
            occupied,
            side_to_move,
            castling_rights,
            en_passant,
            halfmove_clock,
            fullmove_counter,
            hash: 0,
        }
    }

    pub fn from_fen(fen: &str) -> Result<Position, FenError> {
        fen::parse_fen(fen)
    }

    pub fn to_fen(&self) -> String {
        fen::format_fen(self)
    }

    /// Returns the standard chess starting position.
    pub fn startpos() -> Position {
        let mut piece_bb = [Bitboard::EMPTY; 12];

        // White pieces
        piece_bb[Piece::new(Color::White, PieceKind::Pawn).index()] =
            Bitboard::new(0x0000_0000_0000_FF00);
        piece_bb[Piece::new(Color::White, PieceKind::Knight).index()] =
            Bitboard::new(0x0000_0000_0000_0042);
        piece_bb[Piece::new(Color::White, PieceKind::Bishop).index()] =
            Bitboard::new(0x0000_0000_0000_0024);
        piece_bb[Piece::new(Color::White, PieceKind::Rook).index()] =
            Bitboard::new(0x0000_0000_0000_0081);
        piece_bb[Piece::new(Color::White, PieceKind::Queen).index()] =
            Bitboard::new(0x0000_0000_0000_0008);
        piece_bb[Piece::new(Color::White, PieceKind::King).index()] =
            Bitboard::new(0x0000_0000_0000_0010);

        // Black pieces
        piece_bb[Piece::new(Color::Black, PieceKind::Pawn).index()] =
            Bitboard::new(0x00FF_0000_0000_0000);
        piece_bb[Piece::new(Color::Black, PieceKind::Knight).index()] =
            Bitboard::new(0x4200_0000_0000_0000);
        piece_bb[Piece::new(Color::Black, PieceKind::Bishop).index()] =
            Bitboard::new(0x2400_0000_0000_0000);
        piece_bb[Piece::new(Color::Black, PieceKind::Rook).index()] =
            Bitboard::new(0x8100_0000_0000_0000);
        piece_bb[Piece::new(Color::Black, PieceKind::Queen).index()] =
            Bitboard::new(0x0800_0000_0000_0000);
        piece_bb[Piece::new(Color::Black, PieceKind::King).index()] =
            Bitboard::new(0x1000_0000_0000_0000);

        let mut white_occ = Bitboard::EMPTY;
        for bb in &piece_bb[..6] {
            white_occ |= *bb;
        }
        let mut black_occ = Bitboard::EMPTY;
        for bb in &piece_bb[6..] {
            black_occ |= *bb;
        }
        let occupied = white_occ | black_occ;

        let mut pos = Position {
            piece_bb,
            occupied_by: [white_occ, black_occ],
            occupied,
            side_to_move: Color::White,
            castling_rights: CastlingRights::ALL,
            en_passant: None,
            halfmove_clock: 0,
            fullmove_counter: 1,
            hash: 0,
        };
        pos.hash = crate::zobrist::compute_hash(&pos);
        pos
    }

    /// Returns the bitboard for a given piece.
    pub fn piece_bitboard(&self, piece: Piece) -> Bitboard {
        self.piece_bb[piece.index()]
    }

    /// Returns the occupancy bitboard for a given color.
    pub fn occupied_by(&self, color: Color) -> Bitboard {
        self.occupied_by[color as usize]
    }

    /// Returns the total occupancy bitboard.
    pub fn occupied(&self) -> Bitboard {
        self.occupied
    }

    /// Returns the piece on a given square, or `None` if empty.
    pub fn piece_on(&self, square: Square) -> Option<Piece> {
        let sq_bb = Bitboard::new(1u64 << square.index());
        if (self.occupied & sq_bb).is_empty() {
            return None;
        }
        for i in 0..12 {
            if !(self.piece_bb[i] & sq_bb).is_empty() {
                return Piece::from_index(i);
            }
        }
        None
    }

    /// Returns the side to move.
    pub fn side_to_move(&self) -> Color {
        self.side_to_move
    }

    /// Returns the castling rights.
    pub fn castling_rights(&self) -> CastlingRights {
        self.castling_rights
    }

    /// Returns the en passant target square, if any.
    pub fn en_passant(&self) -> Option<Square> {
        self.en_passant
    }

    /// Returns the halfmove clock.
    pub fn halfmove_clock(&self) -> u8 {
        self.halfmove_clock
    }

    /// Returns the fullmove counter.
    pub fn fullmove_counter(&self) -> u16 {
        self.fullmove_counter
    }

    pub fn hash(&self) -> u64 {
        self.hash
    }

    pub(crate) fn set_hash(&mut self, hash: u64) {
        self.hash = hash;
    }

    pub fn toggle_piece_hash(&mut self, piece: Piece, square: Square) {
        self.hash ^= crate::zobrist::piece_square_key(piece, square);
    }

    pub fn toggle_side_to_move_hash(&mut self) {
        self.hash ^= crate::zobrist::side_to_move_key();
    }

    pub fn toggle_castling_hash(&mut self, rights: CastlingRights) {
        self.hash ^= crate::zobrist::castling_key(rights);
    }

    pub fn toggle_en_passant_hash(&mut self, file: File) {
        self.hash ^= crate::zobrist::en_passant_key(file);
    }

    /// Applies a move to the position, updating all state, and returns the info needed to undo it.
    pub fn make_move(&mut self, mv: Move) -> UndoInfo {
        use crate::zobrist;

        // 1. Save undo state
        let mut undo = UndoInfo {
            captured: None,
            castling_rights: self.castling_rights,
            en_passant: self.en_passant,
            halfmove_clock: self.halfmove_clock,
            hash: self.hash,
        };

        // 2. Extract move info
        let from = mv.from_sq();
        let to = mv.to_sq();
        let flags = mv.flags();
        let moving_piece = self.piece_on(from).expect("no piece on source square");
        let us = self.side_to_move;
        let them = us.opposite();

        // 3. Clear en passant unconditionally
        if let Some(ep_sq) = self.en_passant {
            self.hash ^= zobrist::en_passant_key(ep_sq.file());
            self.en_passant = None;
        }

        // 4. Hash out old castling rights
        self.hash ^= zobrist::castling_key(self.castling_rights);

        // 5 & 6. Handle by move type
        let from_bb = Bitboard::new(1u64 << from.index());
        let to_bb = Bitboard::new(1u64 << to.index());

        match flags {
            MoveFlag::QUIET => {
                // Remove piece from `from`, place on `to`
                *self.piece_bb_mut(moving_piece) ^= from_bb | to_bb;
                *self.occupied_by_mut(us) ^= from_bb | to_bb;
                self.hash ^= zobrist::piece_square_key(moving_piece, from);
                self.hash ^= zobrist::piece_square_key(moving_piece, to);
            }
            MoveFlag::CAPTURE => {
                let captured = self.piece_on(to).expect("no piece on capture square");
                undo.captured = Some(captured);
                // Remove captured piece
                *self.piece_bb_mut(captured) ^= to_bb;
                *self.occupied_by_mut(them) ^= to_bb;
                // Move our piece
                *self.piece_bb_mut(moving_piece) ^= from_bb | to_bb;
                *self.occupied_by_mut(us) ^= from_bb | to_bb;
                self.hash ^= zobrist::piece_square_key(captured, to);
                self.hash ^= zobrist::piece_square_key(moving_piece, from);
                self.hash ^= zobrist::piece_square_key(moving_piece, to);
            }
            MoveFlag::DOUBLE_PAWN_PUSH => {
                *self.piece_bb_mut(moving_piece) ^= from_bb | to_bb;
                *self.occupied_by_mut(us) ^= from_bb | to_bb;
                self.hash ^= zobrist::piece_square_key(moving_piece, from);
                self.hash ^= zobrist::piece_square_key(moving_piece, to);
                // Set en passant square
                let ep_idx = if us == Color::White {
                    to.index() - 8
                } else {
                    to.index() + 8
                };
                let ep_sq = Square::new(ep_idx).expect("invalid en passant square");
                self.en_passant = Some(ep_sq);
                self.hash ^= zobrist::en_passant_key(ep_sq.file());
            }
            MoveFlag::KING_CASTLE => {
                // Move king
                *self.piece_bb_mut(moving_piece) ^= from_bb | to_bb;
                *self.occupied_by_mut(us) ^= from_bb | to_bb;
                self.hash ^= zobrist::piece_square_key(moving_piece, from);
                self.hash ^= zobrist::piece_square_key(moving_piece, to);
                // Move rook
                let rook = Piece::new(us, PieceKind::Rook);
                let (rook_from, rook_to) = if us == Color::White {
                    (Square::H1, Square::F1)
                } else {
                    (Square::H8, Square::F8)
                };
                let rook_from_bb = Bitboard::new(1u64 << rook_from.index());
                let rook_to_bb = Bitboard::new(1u64 << rook_to.index());
                *self.piece_bb_mut(rook) ^= rook_from_bb | rook_to_bb;
                *self.occupied_by_mut(us) ^= rook_from_bb | rook_to_bb;
                self.hash ^= zobrist::piece_square_key(rook, rook_from);
                self.hash ^= zobrist::piece_square_key(rook, rook_to);
            }
            MoveFlag::QUEEN_CASTLE => {
                // Move king
                *self.piece_bb_mut(moving_piece) ^= from_bb | to_bb;
                *self.occupied_by_mut(us) ^= from_bb | to_bb;
                self.hash ^= zobrist::piece_square_key(moving_piece, from);
                self.hash ^= zobrist::piece_square_key(moving_piece, to);
                // Move rook
                let rook = Piece::new(us, PieceKind::Rook);
                let (rook_from, rook_to) = if us == Color::White {
                    (Square::A1, Square::D1)
                } else {
                    (Square::A8, Square::D8)
                };
                let rook_from_bb = Bitboard::new(1u64 << rook_from.index());
                let rook_to_bb = Bitboard::new(1u64 << rook_to.index());
                *self.piece_bb_mut(rook) ^= rook_from_bb | rook_to_bb;
                *self.occupied_by_mut(us) ^= rook_from_bb | rook_to_bb;
                self.hash ^= zobrist::piece_square_key(rook, rook_from);
                self.hash ^= zobrist::piece_square_key(rook, rook_to);
            }
            MoveFlag::EN_PASSANT => {
                // Determine captured pawn square
                let cap_idx = if us == Color::White {
                    to.index() - 8
                } else {
                    to.index() + 8
                };
                let cap_sq = Square::new(cap_idx).expect("invalid en passant capture square");
                let cap_bb = Bitboard::new(1u64 << cap_sq.index());
                let captured_pawn = Piece::new(them, PieceKind::Pawn);
                undo.captured = Some(captured_pawn);
                // Remove captured pawn
                *self.piece_bb_mut(captured_pawn) ^= cap_bb;
                *self.occupied_by_mut(them) ^= cap_bb;
                // Move our pawn
                *self.piece_bb_mut(moving_piece) ^= from_bb | to_bb;
                *self.occupied_by_mut(us) ^= from_bb | to_bb;
                self.hash ^= zobrist::piece_square_key(captured_pawn, cap_sq);
                self.hash ^= zobrist::piece_square_key(moving_piece, from);
                self.hash ^= zobrist::piece_square_key(moving_piece, to);
            }
            _ => {
                // Promotions and promotion-captures
                let is_capture = mv.is_capture();
                if is_capture {
                    let captured = self
                        .piece_on(to)
                        .expect("no piece on promotion-capture square");
                    undo.captured = Some(captured);
                    *self.piece_bb_mut(captured) ^= to_bb;
                    *self.occupied_by_mut(them) ^= to_bb;
                    self.hash ^= zobrist::piece_square_key(captured, to);
                }
                let promo_kind = mv
                    .promotion_piece()
                    .expect("promotion flag without promotion piece");
                let promoted = Piece::new(us, promo_kind);
                // Remove pawn from source
                *self.piece_bb_mut(moving_piece) ^= from_bb;
                *self.occupied_by_mut(us) ^= from_bb;
                // Place promoted piece on destination
                *self.piece_bb_mut(promoted) ^= to_bb;
                *self.occupied_by_mut(us) ^= to_bb;
                self.hash ^= zobrist::piece_square_key(moving_piece, from);
                self.hash ^= zobrist::piece_square_key(promoted, to);
            }
        }

        // 7. Update castling rights
        self.castling_rights
            .mask(CASTLING_RIGHTS_MASK[from.index() as usize]);
        self.castling_rights
            .mask(CASTLING_RIGHTS_MASK[to.index() as usize]);

        // 8. Hash in new castling rights
        self.hash ^= zobrist::castling_key(self.castling_rights);

        // 9. Update halfmove clock
        if moving_piece.kind == PieceKind::Pawn || undo.captured.is_some() {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }

        // 10. Update fullmove counter
        if us == Color::Black {
            self.fullmove_counter += 1;
        }

        // 11. Toggle side to move
        self.side_to_move = them;
        self.hash ^= zobrist::side_to_move_key();

        // 12. Update total occupancy
        self.occupied = self.occupied_by[0] | self.occupied_by[1];

        undo
    }

    /// Restores the position to its exact state before the corresponding `make_move` call.
    pub fn unmake_move(&mut self, mv: Move, undo: UndoInfo) {
        // 1. Toggle side to move back
        self.side_to_move = self.side_to_move.opposite();
        let us = self.side_to_move;
        let them = us.opposite();

        // 2. Extract move info
        let from = mv.from_sq();
        let to = mv.to_sq();
        let flags = mv.flags();

        let from_bb = Bitboard::new(1u64 << from.index());
        let to_bb = Bitboard::new(1u64 << to.index());

        // 3. Handle by move type
        match flags {
            MoveFlag::QUIET => {
                let piece = self.piece_on(to).expect("no piece on destination square");
                *self.piece_bb_mut(piece) ^= from_bb | to_bb;
                *self.occupied_by_mut(us) ^= from_bb | to_bb;
            }
            MoveFlag::CAPTURE => {
                let piece = self.piece_on(to).expect("no piece on destination square");
                *self.piece_bb_mut(piece) ^= from_bb | to_bb;
                *self.occupied_by_mut(us) ^= from_bb | to_bb;
                let captured = undo.captured.expect("capture move without captured piece");
                *self.piece_bb_mut(captured) ^= to_bb;
                *self.occupied_by_mut(them) ^= to_bb;
            }
            MoveFlag::DOUBLE_PAWN_PUSH => {
                let piece = self.piece_on(to).expect("no piece on destination square");
                *self.piece_bb_mut(piece) ^= from_bb | to_bb;
                *self.occupied_by_mut(us) ^= from_bb | to_bb;
            }
            MoveFlag::KING_CASTLE => {
                // Move king back
                let king = self.piece_on(to).expect("no king on destination square");
                *self.piece_bb_mut(king) ^= from_bb | to_bb;
                *self.occupied_by_mut(us) ^= from_bb | to_bb;
                // Move rook back
                let rook = Piece::new(us, PieceKind::Rook);
                let (rook_from, rook_to) = if us == Color::White {
                    (Square::H1, Square::F1)
                } else {
                    (Square::H8, Square::F8)
                };
                let rook_from_bb = Bitboard::new(1u64 << rook_from.index());
                let rook_to_bb = Bitboard::new(1u64 << rook_to.index());
                *self.piece_bb_mut(rook) ^= rook_from_bb | rook_to_bb;
                *self.occupied_by_mut(us) ^= rook_from_bb | rook_to_bb;
            }
            MoveFlag::QUEEN_CASTLE => {
                // Move king back
                let king = self.piece_on(to).expect("no king on destination square");
                *self.piece_bb_mut(king) ^= from_bb | to_bb;
                *self.occupied_by_mut(us) ^= from_bb | to_bb;
                // Move rook back
                let rook = Piece::new(us, PieceKind::Rook);
                let (rook_from, rook_to) = if us == Color::White {
                    (Square::A1, Square::D1)
                } else {
                    (Square::A8, Square::D8)
                };
                let rook_from_bb = Bitboard::new(1u64 << rook_from.index());
                let rook_to_bb = Bitboard::new(1u64 << rook_to.index());
                *self.piece_bb_mut(rook) ^= rook_from_bb | rook_to_bb;
                *self.occupied_by_mut(us) ^= rook_from_bb | rook_to_bb;
            }
            MoveFlag::EN_PASSANT => {
                let piece = self.piece_on(to).expect("no piece on destination square");
                *self.piece_bb_mut(piece) ^= from_bb | to_bb;
                *self.occupied_by_mut(us) ^= from_bb | to_bb;
                // Restore captured pawn
                let cap_idx = if us == Color::White {
                    to.index() - 8
                } else {
                    to.index() + 8
                };
                let cap_sq = Square::new(cap_idx).expect("invalid en passant capture square");
                let cap_bb = Bitboard::new(1u64 << cap_sq.index());
                let captured = undo.captured.expect("en passant without captured piece");
                *self.piece_bb_mut(captured) ^= cap_bb;
                *self.occupied_by_mut(them) ^= cap_bb;
            }
            _ => {
                // Promotions and promotion-captures
                let promo_kind = mv
                    .promotion_piece()
                    .expect("promotion flag without promotion piece");
                let promoted = Piece::new(us, promo_kind);
                let pawn = Piece::new(us, PieceKind::Pawn);
                // Remove promoted piece from destination
                *self.piece_bb_mut(promoted) ^= to_bb;
                *self.occupied_by_mut(us) ^= to_bb;
                // Place pawn back on source
                *self.piece_bb_mut(pawn) ^= from_bb;
                *self.occupied_by_mut(us) ^= from_bb;
                // Restore captured piece if promotion-capture
                if mv.is_capture() {
                    let captured = undo
                        .captured
                        .expect("promotion-capture without captured piece");
                    *self.piece_bb_mut(captured) ^= to_bb;
                    *self.occupied_by_mut(them) ^= to_bb;
                }
            }
        }

        // 4. Update fullmove counter
        if us == Color::Black {
            self.fullmove_counter -= 1;
        }

        // 5. Restore saved state from UndoInfo
        self.set_castling_rights(undo.castling_rights);
        self.set_en_passant(undo.en_passant);
        self.set_halfmove_clock(undo.halfmove_clock);

        // 6. Restore Zobrist hash
        self.set_hash(undo.hash);

        // 7. Recompute total occupancy
        self.occupied = self.occupied_by[0] | self.occupied_by[1];
    }

    pub(crate) fn piece_bb_mut(&mut self, piece: Piece) -> &mut Bitboard {
        &mut self.piece_bb[piece.index()]
    }

    pub(crate) fn occupied_by_mut(&mut self, color: Color) -> &mut Bitboard {
        &mut self.occupied_by[color as usize]
    }

    #[allow(dead_code)]
    pub(crate) fn occupied_mut(&mut self) -> &mut Bitboard {
        &mut self.occupied
    }

    #[allow(dead_code)]
    pub(crate) fn set_side_to_move(&mut self, color: Color) {
        self.side_to_move = color;
    }

    pub(crate) fn set_castling_rights(&mut self, rights: CastlingRights) {
        self.castling_rights = rights;
    }

    pub(crate) fn set_en_passant(&mut self, sq: Option<Square>) {
        self.en_passant = sq;
    }

    pub(crate) fn set_halfmove_clock(&mut self, clock: u8) {
        self.halfmove_clock = clock;
    }

    #[allow(dead_code)]
    pub(crate) fn set_fullmove_counter(&mut self, counter: u16) {
        self.fullmove_counter = counter;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_types::{Move, MoveFlag};

    #[test]
    fn startpos_piece_placement() {
        let pos = Position::startpos();

        // White pawns: 8 pawns on rank 2
        let wp = pos.piece_bitboard(Piece::new(Color::White, PieceKind::Pawn));
        assert_eq!(wp.pop_count(), 8);
        assert_eq!(wp.inner(), 0x0000_0000_0000_FF00);

        // White knights: b1, g1
        let wn = pos.piece_bitboard(Piece::new(Color::White, PieceKind::Knight));
        assert_eq!(wn.pop_count(), 2);
        assert_eq!(wn.inner(), 0x0000_0000_0000_0042);

        // White bishops: c1, f1
        let wb = pos.piece_bitboard(Piece::new(Color::White, PieceKind::Bishop));
        assert_eq!(wb.pop_count(), 2);
        assert_eq!(wb.inner(), 0x0000_0000_0000_0024);

        // White rooks: a1, h1
        let wr = pos.piece_bitboard(Piece::new(Color::White, PieceKind::Rook));
        assert_eq!(wr.pop_count(), 2);
        assert_eq!(wr.inner(), 0x0000_0000_0000_0081);

        // White queen: d1
        let wq = pos.piece_bitboard(Piece::new(Color::White, PieceKind::Queen));
        assert_eq!(wq.pop_count(), 1);
        assert_eq!(wq.inner(), 0x0000_0000_0000_0008);

        // White king: e1
        let wk = pos.piece_bitboard(Piece::new(Color::White, PieceKind::King));
        assert_eq!(wk.pop_count(), 1);
        assert_eq!(wk.inner(), 0x0000_0000_0000_0010);

        // Black pawns: 8 pawns on rank 7
        let bp = pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Pawn));
        assert_eq!(bp.pop_count(), 8);
        assert_eq!(bp.inner(), 0x00FF_0000_0000_0000);

        // Black knights: b8, g8
        let bn = pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Knight));
        assert_eq!(bn.pop_count(), 2);
        assert_eq!(bn.inner(), 0x4200_0000_0000_0000);

        // Black bishops: c8, f8
        let bb = pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Bishop));
        assert_eq!(bb.pop_count(), 2);
        assert_eq!(bb.inner(), 0x2400_0000_0000_0000);

        // Black rooks: a8, h8
        let br = pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Rook));
        assert_eq!(br.pop_count(), 2);
        assert_eq!(br.inner(), 0x8100_0000_0000_0000);

        // Black queen: d8
        let bq = pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Queen));
        assert_eq!(bq.pop_count(), 1);
        assert_eq!(bq.inner(), 0x0800_0000_0000_0000);

        // Black king: e8
        let bk = pos.piece_bitboard(Piece::new(Color::Black, PieceKind::King));
        assert_eq!(bk.pop_count(), 1);
        assert_eq!(bk.inner(), 0x1000_0000_0000_0000);
    }

    #[test]
    fn startpos_occupancy() {
        let pos = Position::startpos();

        let white_occ = pos.occupied_by(Color::White);
        assert_eq!(white_occ.pop_count(), 16);
        assert_eq!(white_occ.inner(), 0x0000_0000_0000_FFFF);

        let black_occ = pos.occupied_by(Color::Black);
        assert_eq!(black_occ.pop_count(), 16);
        assert_eq!(black_occ.inner(), 0xFFFF_0000_0000_0000);

        let all_occ = pos.occupied();
        assert_eq!(all_occ.pop_count(), 32);
    }

    #[test]
    fn startpos_game_state() {
        let pos = Position::startpos();

        assert_eq!(pos.side_to_move(), Color::White);
        assert_eq!(pos.castling_rights(), CastlingRights::ALL);
        assert!(pos
            .castling_rights()
            .contains(CastlingRights::WHITE_KINGSIDE));
        assert!(pos
            .castling_rights()
            .contains(CastlingRights::WHITE_QUEENSIDE));
        assert!(pos
            .castling_rights()
            .contains(CastlingRights::BLACK_KINGSIDE));
        assert!(pos
            .castling_rights()
            .contains(CastlingRights::BLACK_QUEENSIDE));
        assert_eq!(pos.en_passant(), None);
        assert_eq!(pos.halfmove_clock(), 0);
        assert_eq!(pos.fullmove_counter(), 1);
    }

    #[test]
    fn piece_on_startpos() {
        let pos = Position::startpos();

        assert_eq!(
            pos.piece_on(Square::E1),
            Some(Piece::new(Color::White, PieceKind::King))
        );
        assert_eq!(
            pos.piece_on(Square::D8),
            Some(Piece::new(Color::Black, PieceKind::Queen))
        );
        assert_eq!(
            pos.piece_on(Square::A2),
            Some(Piece::new(Color::White, PieceKind::Pawn))
        );
        assert_eq!(pos.piece_on(Square::E4), None);
    }

    #[test]
    fn occupancy_consistency() {
        let pos = Position::startpos();

        let white_occ = pos.occupied_by(Color::White);
        let black_occ = pos.occupied_by(Color::Black);
        let all_occ = pos.occupied();

        assert_eq!((white_occ | black_occ).inner(), all_occ.inner());
        assert!((white_occ & black_occ).is_empty());
    }

    fn assert_hash_matches_recomputation(pos: &Position) {
        let recomputed = crate::zobrist::compute_hash(pos);
        assert_eq!(
            pos.hash(),
            recomputed,
            "incremental hash {:#018x} != recomputed {:#018x}",
            pos.hash(),
            recomputed
        );
    }

    #[test]
    fn make_move_quiet() {
        // Ng1-f3 from startpos
        let mut pos = Position::startpos();
        let mv = Move::new(Square::G1, Square::F3, MoveFlag::QUIET);
        let undo = pos.make_move(mv);

        let wn = pos.piece_bitboard(Piece::new(Color::White, PieceKind::Knight));
        assert!(!(wn & Bitboard::new(1u64 << Square::G1.index())).is_empty() == false);
        assert!(!(wn & Bitboard::new(1u64 << Square::F3.index())).is_empty());
        assert_eq!(pos.piece_on(Square::G1), None);
        assert_eq!(
            pos.piece_on(Square::F3),
            Some(Piece::new(Color::White, PieceKind::Knight))
        );
        assert_eq!(pos.side_to_move(), Color::Black);
        assert_eq!(pos.halfmove_clock(), 1);
        assert_eq!(pos.en_passant(), None);
        assert!(undo.captured.is_none());
        assert_hash_matches_recomputation(&pos);
    }

    #[test]
    fn make_move_capture() {
        // Position where white knight on f3 can capture black pawn on e5
        let fen = "rnbqkbnr/pppp1ppp/8/4p3/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 2";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::F3, Square::E5, MoveFlag::CAPTURE);
        let undo = pos.make_move(mv);

        assert_eq!(
            pos.piece_on(Square::E5),
            Some(Piece::new(Color::White, PieceKind::Knight))
        );
        assert_eq!(pos.piece_on(Square::F3), None);
        assert_eq!(
            undo.captured,
            Some(Piece::new(Color::Black, PieceKind::Pawn))
        );
        assert_eq!(pos.halfmove_clock(), 0);
        assert_eq!(pos.side_to_move(), Color::Black);
        assert_hash_matches_recomputation(&pos);
    }

    #[test]
    fn make_move_double_pawn_push() {
        let mut pos = Position::startpos();
        let mv = Move::new(Square::E2, Square::E4, MoveFlag::DOUBLE_PAWN_PUSH);
        let _undo = pos.make_move(mv);

        assert_eq!(
            pos.piece_on(Square::E4),
            Some(Piece::new(Color::White, PieceKind::Pawn))
        );
        assert_eq!(pos.piece_on(Square::E2), None);
        assert_eq!(pos.en_passant(), Some(Square::E3));
        assert_eq!(pos.halfmove_clock(), 0);
        assert_eq!(pos.side_to_move(), Color::Black);
        assert_hash_matches_recomputation(&pos);
    }

    #[test]
    fn make_move_white_kingside_castle() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/5NP1/PPPPPPBP/RNBQK2R w KQkq - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E1, Square::G1, MoveFlag::KING_CASTLE);
        let _undo = pos.make_move(mv);

        assert_eq!(
            pos.piece_on(Square::G1),
            Some(Piece::new(Color::White, PieceKind::King))
        );
        assert_eq!(
            pos.piece_on(Square::F1),
            Some(Piece::new(Color::White, PieceKind::Rook))
        );
        assert_eq!(pos.piece_on(Square::E1), None);
        assert_eq!(pos.piece_on(Square::H1), None);
        assert!(!pos
            .castling_rights()
            .contains(CastlingRights::WHITE_KINGSIDE));
        assert!(!pos
            .castling_rights()
            .contains(CastlingRights::WHITE_QUEENSIDE));
        assert_hash_matches_recomputation(&pos);
    }

    #[test]
    fn make_move_white_queenside_castle() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/2NQ4/PPPPPPPP/R3KBNR w KQkq - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E1, Square::C1, MoveFlag::QUEEN_CASTLE);
        let _undo = pos.make_move(mv);

        assert_eq!(
            pos.piece_on(Square::C1),
            Some(Piece::new(Color::White, PieceKind::King))
        );
        assert_eq!(
            pos.piece_on(Square::D1),
            Some(Piece::new(Color::White, PieceKind::Rook))
        );
        assert_eq!(pos.piece_on(Square::E1), None);
        assert_eq!(pos.piece_on(Square::A1), None);
        assert!(!pos
            .castling_rights()
            .contains(CastlingRights::WHITE_KINGSIDE));
        assert!(!pos
            .castling_rights()
            .contains(CastlingRights::WHITE_QUEENSIDE));
        assert_hash_matches_recomputation(&pos);
    }

    #[test]
    fn make_move_black_kingside_castle() {
        let fen = "rnbqk2r/ppppppbp/5np1/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E8, Square::G8, MoveFlag::KING_CASTLE);
        let _undo = pos.make_move(mv);

        assert_eq!(
            pos.piece_on(Square::G8),
            Some(Piece::new(Color::Black, PieceKind::King))
        );
        assert_eq!(
            pos.piece_on(Square::F8),
            Some(Piece::new(Color::Black, PieceKind::Rook))
        );
        assert_eq!(pos.piece_on(Square::E8), None);
        assert_eq!(pos.piece_on(Square::H8), None);
        assert!(!pos
            .castling_rights()
            .contains(CastlingRights::BLACK_KINGSIDE));
        assert!(!pos
            .castling_rights()
            .contains(CastlingRights::BLACK_QUEENSIDE));
        assert_hash_matches_recomputation(&pos);
    }

    #[test]
    fn make_move_black_queenside_castle() {
        let fen = "r3kbnr/ppqppppp/2n5/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E8, Square::C8, MoveFlag::QUEEN_CASTLE);
        let _undo = pos.make_move(mv);

        assert_eq!(
            pos.piece_on(Square::C8),
            Some(Piece::new(Color::Black, PieceKind::King))
        );
        assert_eq!(
            pos.piece_on(Square::D8),
            Some(Piece::new(Color::Black, PieceKind::Rook))
        );
        assert_eq!(pos.piece_on(Square::E8), None);
        assert_eq!(pos.piece_on(Square::A8), None);
        assert!(!pos
            .castling_rights()
            .contains(CastlingRights::BLACK_KINGSIDE));
        assert!(!pos
            .castling_rights()
            .contains(CastlingRights::BLACK_QUEENSIDE));
        assert_hash_matches_recomputation(&pos);
    }

    #[test]
    fn make_move_en_passant() {
        // White pawn on e5, black pawn on d5 (just double-pushed), EP square d6
        let fen = "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E5, Square::D6, MoveFlag::EN_PASSANT);
        let undo = pos.make_move(mv);

        assert_eq!(
            pos.piece_on(Square::D6),
            Some(Piece::new(Color::White, PieceKind::Pawn))
        );
        assert_eq!(pos.piece_on(Square::E5), None);
        assert_eq!(pos.piece_on(Square::D5), None); // captured pawn removed
        assert_eq!(
            undo.captured,
            Some(Piece::new(Color::Black, PieceKind::Pawn))
        );
        assert_eq!(pos.halfmove_clock(), 0);
        assert_hash_matches_recomputation(&pos);
    }

    #[test]
    fn make_move_promotion_knight() {
        let fen = "8/4P3/8/8/8/8/4k3/4K3 w - - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E7, Square::E8, MoveFlag::KNIGHT_PROMOTION);
        let _undo = pos.make_move(mv);

        assert_eq!(
            pos.piece_on(Square::E8),
            Some(Piece::new(Color::White, PieceKind::Knight))
        );
        assert_eq!(pos.piece_on(Square::E7), None);
        assert_hash_matches_recomputation(&pos);
    }

    #[test]
    fn make_move_promotion_bishop() {
        let fen = "8/4P3/8/8/8/8/4k3/4K3 w - - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E7, Square::E8, MoveFlag::BISHOP_PROMOTION);
        let _undo = pos.make_move(mv);

        assert_eq!(
            pos.piece_on(Square::E8),
            Some(Piece::new(Color::White, PieceKind::Bishop))
        );
        assert_eq!(pos.piece_on(Square::E7), None);
        assert_hash_matches_recomputation(&pos);
    }

    #[test]
    fn make_move_promotion_rook() {
        let fen = "8/4P3/8/8/8/8/4k3/4K3 w - - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E7, Square::E8, MoveFlag::ROOK_PROMOTION);
        let _undo = pos.make_move(mv);

        assert_eq!(
            pos.piece_on(Square::E8),
            Some(Piece::new(Color::White, PieceKind::Rook))
        );
        assert_eq!(pos.piece_on(Square::E7), None);
        assert_hash_matches_recomputation(&pos);
    }

    #[test]
    fn make_move_promotion_queen() {
        let fen = "8/4P3/8/8/8/8/4k3/4K3 w - - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E7, Square::E8, MoveFlag::QUEEN_PROMOTION);
        let _undo = pos.make_move(mv);

        assert_eq!(
            pos.piece_on(Square::E8),
            Some(Piece::new(Color::White, PieceKind::Queen))
        );
        assert_eq!(pos.piece_on(Square::E7), None);
        assert_hash_matches_recomputation(&pos);
    }

    #[test]
    fn make_move_promotion_capture() {
        // White pawn on e7, black rook on d8
        let fen = "3r4/4P3/8/8/8/8/4k3/4K3 w - - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E7, Square::D8, MoveFlag::QUEEN_PROMOTION_CAPTURE);
        let undo = pos.make_move(mv);

        assert_eq!(
            pos.piece_on(Square::D8),
            Some(Piece::new(Color::White, PieceKind::Queen))
        );
        assert_eq!(pos.piece_on(Square::E7), None);
        assert_eq!(
            undo.captured,
            Some(Piece::new(Color::Black, PieceKind::Rook))
        );
        assert_hash_matches_recomputation(&pos);
    }

    #[test]
    fn make_move_fullmove_counter() {
        let mut pos = Position::startpos();
        // White moves: fullmove should stay 1
        let mv_w = Move::new(Square::E2, Square::E4, MoveFlag::DOUBLE_PAWN_PUSH);
        let _undo = pos.make_move(mv_w);
        assert_eq!(pos.fullmove_counter(), 1);

        // Black moves: fullmove should increment to 2
        let mv_b = Move::new(Square::E7, Square::E5, MoveFlag::DOUBLE_PAWN_PUSH);
        let _undo = pos.make_move(mv_b);
        assert_eq!(pos.fullmove_counter(), 2);
    }

    #[test]
    fn make_move_castling_rights_rook_capture() {
        // White queen on h4 captures black rook on h8, should clear BLACK_KINGSIDE
        let fen = "rnbqk2r/pppppppp/8/8/7Q/8/PPPPPPPP/RNB1KBNR w KQkq - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        assert!(pos
            .castling_rights()
            .contains(CastlingRights::BLACK_KINGSIDE));

        let mv = Move::new(Square::H4, Square::H8, MoveFlag::CAPTURE);
        let _undo = pos.make_move(mv);

        assert!(!pos
            .castling_rights()
            .contains(CastlingRights::BLACK_KINGSIDE));
        assert_hash_matches_recomputation(&pos);
    }

    #[test]
    fn make_move_clears_prior_ep() {
        // Position with en passant square set, make a non-double-pawn-push move
        let fen = "rnbqkbnr/pppp1ppp/8/4pP2/8/8/PPPPP1PP/RNBQKBNR w KQkq e6 0 3";
        let mut pos = Position::from_fen(fen).unwrap();
        assert_eq!(pos.en_passant(), Some(Square::E6));

        // Quiet knight move: should clear EP
        let mv = Move::new(Square::G1, Square::F3, MoveFlag::QUIET);
        let _undo = pos.make_move(mv);

        assert_eq!(pos.en_passant(), None);
        assert_hash_matches_recomputation(&pos);
    }

    fn assert_round_trip(pos: &mut Position, mv: Move) {
        let original_fen = pos.to_fen();
        let original_hash = pos.hash();
        let original_side = pos.side_to_move();
        let original_castling = pos.castling_rights();
        let original_ep = pos.en_passant();
        let original_halfmove = pos.halfmove_clock();
        let original_fullmove = pos.fullmove_counter();
        let original_occupied = pos.occupied();
        let original_white_occ = pos.occupied_by(Color::White);
        let original_black_occ = pos.occupied_by(Color::Black);

        let undo = pos.make_move(mv);
        pos.unmake_move(mv, undo);

        assert_eq!(pos.to_fen(), original_fen, "FEN mismatch after round-trip");
        assert_eq!(pos.hash(), original_hash, "hash mismatch after round-trip");
        assert_eq!(pos.side_to_move(), original_side);
        assert_eq!(pos.castling_rights(), original_castling);
        assert_eq!(pos.en_passant(), original_ep);
        assert_eq!(pos.halfmove_clock(), original_halfmove);
        assert_eq!(pos.fullmove_counter(), original_fullmove);
        assert_eq!(pos.occupied(), original_occupied);
        assert_eq!(pos.occupied_by(Color::White), original_white_occ);
        assert_eq!(pos.occupied_by(Color::Black), original_black_occ);
    }

    #[test]
    fn unmake_quiet() {
        let mut pos = Position::startpos();
        let mv = Move::new(Square::G1, Square::F3, MoveFlag::QUIET);
        assert_round_trip(&mut pos, mv);
        assert_eq!(
            pos.piece_on(Square::G1),
            Some(Piece::new(Color::White, PieceKind::Knight))
        );
        assert_eq!(pos.piece_on(Square::F3), None);
    }

    #[test]
    fn unmake_capture() {
        let fen = "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E4, Square::D5, MoveFlag::CAPTURE);
        assert_round_trip(&mut pos, mv);
        assert_eq!(
            pos.piece_on(Square::E4),
            Some(Piece::new(Color::White, PieceKind::Pawn))
        );
        assert_eq!(
            pos.piece_on(Square::D5),
            Some(Piece::new(Color::Black, PieceKind::Pawn))
        );
    }

    #[test]
    fn unmake_double_pawn_push() {
        let mut pos = Position::startpos();
        let mv = Move::new(Square::E2, Square::E4, MoveFlag::DOUBLE_PAWN_PUSH);
        assert_round_trip(&mut pos, mv);
        assert_eq!(
            pos.piece_on(Square::E2),
            Some(Piece::new(Color::White, PieceKind::Pawn))
        );
        assert_eq!(pos.piece_on(Square::E4), None);
        assert_eq!(pos.en_passant(), None);
    }

    #[test]
    fn unmake_white_kingside_castle() {
        let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E1, Square::G1, MoveFlag::KING_CASTLE);
        assert_round_trip(&mut pos, mv);
        assert_eq!(
            pos.piece_on(Square::E1),
            Some(Piece::new(Color::White, PieceKind::King))
        );
        assert_eq!(
            pos.piece_on(Square::H1),
            Some(Piece::new(Color::White, PieceKind::Rook))
        );
        assert_eq!(pos.piece_on(Square::G1), None);
        assert_eq!(pos.piece_on(Square::F1), None);
    }

    #[test]
    fn unmake_white_queenside_castle() {
        let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E1, Square::C1, MoveFlag::QUEEN_CASTLE);
        assert_round_trip(&mut pos, mv);
        assert_eq!(
            pos.piece_on(Square::E1),
            Some(Piece::new(Color::White, PieceKind::King))
        );
        assert_eq!(
            pos.piece_on(Square::A1),
            Some(Piece::new(Color::White, PieceKind::Rook))
        );
        assert_eq!(pos.piece_on(Square::C1), None);
        assert_eq!(pos.piece_on(Square::D1), None);
    }

    #[test]
    fn unmake_black_kingside_castle() {
        let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E8, Square::G8, MoveFlag::KING_CASTLE);
        assert_round_trip(&mut pos, mv);
        assert_eq!(
            pos.piece_on(Square::E8),
            Some(Piece::new(Color::Black, PieceKind::King))
        );
        assert_eq!(
            pos.piece_on(Square::H8),
            Some(Piece::new(Color::Black, PieceKind::Rook))
        );
        assert_eq!(pos.piece_on(Square::G8), None);
        assert_eq!(pos.piece_on(Square::F8), None);
    }

    #[test]
    fn unmake_black_queenside_castle() {
        let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E8, Square::C8, MoveFlag::QUEEN_CASTLE);
        assert_round_trip(&mut pos, mv);
        assert_eq!(
            pos.piece_on(Square::E8),
            Some(Piece::new(Color::Black, PieceKind::King))
        );
        assert_eq!(
            pos.piece_on(Square::A8),
            Some(Piece::new(Color::Black, PieceKind::Rook))
        );
        assert_eq!(pos.piece_on(Square::C8), None);
        assert_eq!(pos.piece_on(Square::D8), None);
    }

    #[test]
    fn unmake_en_passant() {
        let fen = "rnbqkbnr/pppp1ppp/8/4pP2/8/8/PPPPP1PP/RNBQKBNR w KQkq e6 0 3";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::F5, Square::E6, MoveFlag::EN_PASSANT);
        assert_round_trip(&mut pos, mv);
        assert_eq!(
            pos.piece_on(Square::F5),
            Some(Piece::new(Color::White, PieceKind::Pawn))
        );
        assert_eq!(
            pos.piece_on(Square::E5),
            Some(Piece::new(Color::Black, PieceKind::Pawn))
        );
        assert_eq!(pos.piece_on(Square::E6), None);
        assert_eq!(pos.en_passant(), Some(Square::E6));
    }

    #[test]
    fn unmake_promotion_knight() {
        let fen = "4k3/4P3/8/8/8/8/8/4K3 w - - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E7, Square::E8, MoveFlag::KNIGHT_PROMOTION);
        assert_round_trip(&mut pos, mv);
        assert_eq!(
            pos.piece_on(Square::E7),
            Some(Piece::new(Color::White, PieceKind::Pawn))
        );
        assert_eq!(
            pos.piece_on(Square::E8),
            Some(Piece::new(Color::Black, PieceKind::King))
        );
    }

    #[test]
    fn unmake_promotion_bishop() {
        let fen = "4k3/4P3/8/8/8/8/8/4K3 w - - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E7, Square::E8, MoveFlag::BISHOP_PROMOTION);
        assert_round_trip(&mut pos, mv);
        assert_eq!(
            pos.piece_on(Square::E7),
            Some(Piece::new(Color::White, PieceKind::Pawn))
        );
    }

    #[test]
    fn unmake_promotion_rook() {
        let fen = "4k3/4P3/8/8/8/8/8/4K3 w - - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E7, Square::E8, MoveFlag::ROOK_PROMOTION);
        assert_round_trip(&mut pos, mv);
        assert_eq!(
            pos.piece_on(Square::E7),
            Some(Piece::new(Color::White, PieceKind::Pawn))
        );
    }

    #[test]
    fn unmake_promotion_queen() {
        let fen = "4k3/4P3/8/8/8/8/8/4K3 w - - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E7, Square::E8, MoveFlag::QUEEN_PROMOTION);
        assert_round_trip(&mut pos, mv);
        assert_eq!(
            pos.piece_on(Square::E7),
            Some(Piece::new(Color::White, PieceKind::Pawn))
        );
    }

    #[test]
    fn unmake_promotion_capture() {
        let fen = "3nk3/4P3/8/8/8/8/8/4K3 w - - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E7, Square::D8, MoveFlag::QUEEN_PROMOTION_CAPTURE);
        assert_round_trip(&mut pos, mv);
        assert_eq!(
            pos.piece_on(Square::E7),
            Some(Piece::new(Color::White, PieceKind::Pawn))
        );
        assert_eq!(
            pos.piece_on(Square::D8),
            Some(Piece::new(Color::Black, PieceKind::Knight))
        );
    }

    #[test]
    fn unmake_fullmove_counter() {
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::E7, Square::E5, MoveFlag::DOUBLE_PAWN_PUSH);
        assert_round_trip(&mut pos, mv);
        assert_eq!(pos.fullmove_counter(), 1);
    }

    #[test]
    fn unmake_preserves_ep_state() {
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let mut pos = Position::from_fen(fen).unwrap();
        let mv = Move::new(Square::B8, Square::C6, MoveFlag::QUIET);
        assert_round_trip(&mut pos, mv);
        assert_eq!(pos.en_passant(), Some(Square::E3));
    }

    #[test]
    fn unmake_multiple_sequential() {
        let mut pos = Position::startpos();
        let original_fen = pos.to_fen();
        let original_hash = pos.hash();

        let mv1 = Move::new(Square::E2, Square::E4, MoveFlag::DOUBLE_PAWN_PUSH);
        let undo1 = pos.make_move(mv1);

        let mv2 = Move::new(Square::D7, Square::D5, MoveFlag::DOUBLE_PAWN_PUSH);
        let undo2 = pos.make_move(mv2);

        let mv3 = Move::new(Square::E4, Square::D5, MoveFlag::CAPTURE);
        let undo3 = pos.make_move(mv3);

        // Unmake in reverse order
        pos.unmake_move(mv3, undo3);
        pos.unmake_move(mv2, undo2);
        pos.unmake_move(mv1, undo1);

        assert_eq!(pos.to_fen(), original_fen);
        assert_eq!(pos.hash(), original_hash);
        assert_eq!(pos.side_to_move(), Color::White);
        assert_eq!(pos.fullmove_counter(), 1);
        assert_eq!(pos.halfmove_clock(), 0);
        assert_eq!(pos.en_passant(), None);
        assert_eq!(pos.castling_rights(), CastlingRights::ALL);
    }
}
