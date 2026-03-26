use chess_types::{Bitboard, Color, File, Piece, PieceKind, Square};

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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
