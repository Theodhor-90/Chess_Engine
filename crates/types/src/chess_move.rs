use crate::piece::PieceKind;
use crate::square::Square;

/// Constants for the 4-bit move flag field.
pub struct MoveFlag;

impl MoveFlag {
    /// Normal quiet move.
    pub const QUIET: u16 = 0;
    /// Pawn advances two ranks.
    pub const DOUBLE_PAWN_PUSH: u16 = 1;
    /// King-side castling (O-O).
    pub const KING_CASTLE: u16 = 2;
    /// Queen-side castling (O-O-O).
    pub const QUEEN_CASTLE: u16 = 3;
    /// Normal capture.
    pub const CAPTURE: u16 = 4;
    /// En passant capture.
    pub const EN_PASSANT: u16 = 5;
    /// Promote to knight.
    pub const KNIGHT_PROMOTION: u16 = 8;
    /// Promote to bishop.
    pub const BISHOP_PROMOTION: u16 = 9;
    /// Promote to rook.
    pub const ROOK_PROMOTION: u16 = 10;
    /// Promote to queen.
    pub const QUEEN_PROMOTION: u16 = 11;
    /// Promote to knight + capture.
    pub const KNIGHT_PROMOTION_CAPTURE: u16 = 12;
    /// Promote to bishop + capture.
    pub const BISHOP_PROMOTION_CAPTURE: u16 = 13;
    /// Promote to rook + capture.
    pub const ROOK_PROMOTION_CAPTURE: u16 = 14;
    /// Promote to queen + capture.
    pub const QUEEN_PROMOTION_CAPTURE: u16 = 15;
}

/// A packed 16-bit chess move.
///
/// Bit layout: `from(6) | to(6) | flags(4)` — bits 15..10 source square,
/// bits 9..4 destination square, bits 3..0 flags.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Move(u16);

impl Move {
    /// Encodes a move from components.
    pub fn new(from: Square, to: Square, flags: u16) -> Move {
        Move((from.index() as u16) << 10 | (to.index() as u16) << 4 | (flags & 0xF))
    }

    /// Wraps a raw `u16` directly.
    pub fn from_raw(raw: u16) -> Move {
        Move(raw)
    }

    /// Returns the underlying `u16`.
    pub fn raw(self) -> u16 {
        self.0
    }

    /// Extracts the source square.
    pub fn from_sq(self) -> Square {
        Square::new(((self.0 >> 10) & 0x3F) as u8).expect("from_sq always valid in a valid Move")
    }

    /// Extracts the destination square.
    pub fn to_sq(self) -> Square {
        Square::new(((self.0 >> 4) & 0x3F) as u8).expect("to_sq always valid in a valid Move")
    }

    /// Extracts the 4-bit flags.
    pub fn flags(self) -> u16 {
        self.0 & 0xF
    }

    /// Returns true if the flags indicate a capture.
    pub fn is_capture(self) -> bool {
        let f = self.flags();
        f == MoveFlag::CAPTURE || f == MoveFlag::EN_PASSANT || f >= 12
    }

    /// Returns true if the flags indicate a promotion.
    pub fn is_promotion(self) -> bool {
        self.flags() >= 8
    }

    /// Returns the promotion piece kind if this is a promotion move.
    pub fn promotion_piece(self) -> Option<PieceKind> {
        match self.flags() {
            8 | 12 => Some(PieceKind::Knight),
            9 | 13 => Some(PieceKind::Bishop),
            10 | 14 => Some(PieceKind::Rook),
            11 | 15 => Some(PieceKind::Queen),
            _ => None,
        }
    }

    /// Returns true for king-side or queen-side castling.
    pub fn is_castling(self) -> bool {
        let f = self.flags();
        f == MoveFlag::KING_CASTLE || f == MoveFlag::QUEEN_CASTLE
    }

    /// Returns true for en passant.
    pub fn is_en_passant(self) -> bool {
        self.flags() == MoveFlag::EN_PASSANT
    }

    /// Returns true for double pawn push.
    pub fn is_double_pawn_push(self) -> bool {
        self.flags() == MoveFlag::DOUBLE_PAWN_PUSH
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.from_sq(), self.to_sq())?;
        if let Some(kind) = self.promotion_piece() {
            let ch = match kind {
                PieceKind::Knight => 'n',
                PieceKind::Bishop => 'b',
                PieceKind::Rook => 'r',
                PieceKind::Queen => 'q',
                _ => unreachable!(),
            };
            write!(f, "{ch}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_quiet() {
        let m = Move::new(Square::E2, Square::E4, MoveFlag::QUIET);
        assert_eq!(m.from_sq(), Square::E2);
        assert_eq!(m.to_sq(), Square::E4);
        assert_eq!(m.flags(), MoveFlag::QUIET);
    }

    #[test]
    fn test_encode_decode_double_pawn_push() {
        let m = Move::new(Square::E2, Square::E4, MoveFlag::DOUBLE_PAWN_PUSH);
        assert_eq!(m.from_sq(), Square::E2);
        assert_eq!(m.to_sq(), Square::E4);
        assert_eq!(m.flags(), MoveFlag::DOUBLE_PAWN_PUSH);
        assert!(m.is_double_pawn_push());
    }

    #[test]
    fn test_encode_decode_king_castle() {
        let m = Move::new(Square::E1, Square::G1, MoveFlag::KING_CASTLE);
        assert_eq!(m.from_sq(), Square::E1);
        assert_eq!(m.to_sq(), Square::G1);
        assert_eq!(m.flags(), MoveFlag::KING_CASTLE);
        assert!(m.is_castling());
    }

    #[test]
    fn test_encode_decode_queen_castle() {
        let m = Move::new(Square::E1, Square::C1, MoveFlag::QUEEN_CASTLE);
        assert_eq!(m.from_sq(), Square::E1);
        assert_eq!(m.to_sq(), Square::C1);
        assert_eq!(m.flags(), MoveFlag::QUEEN_CASTLE);
        assert!(m.is_castling());
    }

    #[test]
    fn test_encode_decode_capture() {
        let m = Move::new(Square::D4, Square::E5, MoveFlag::CAPTURE);
        assert_eq!(m.from_sq(), Square::D4);
        assert_eq!(m.to_sq(), Square::E5);
        assert_eq!(m.flags(), MoveFlag::CAPTURE);
        assert!(m.is_capture());
    }

    #[test]
    fn test_encode_decode_en_passant() {
        let m = Move::new(Square::E5, Square::D6, MoveFlag::EN_PASSANT);
        assert_eq!(m.from_sq(), Square::E5);
        assert_eq!(m.to_sq(), Square::D6);
        assert_eq!(m.flags(), MoveFlag::EN_PASSANT);
        assert!(m.is_en_passant());
        assert!(m.is_capture());
    }

    #[test]
    fn test_encode_decode_promotions() {
        let cases = [
            (MoveFlag::KNIGHT_PROMOTION, PieceKind::Knight),
            (MoveFlag::BISHOP_PROMOTION, PieceKind::Bishop),
            (MoveFlag::ROOK_PROMOTION, PieceKind::Rook),
            (MoveFlag::QUEEN_PROMOTION, PieceKind::Queen),
        ];
        for (flag, expected_kind) in cases {
            let m = Move::new(Square::E7, Square::E8, flag);
            assert!(m.is_promotion());
            assert_eq!(m.promotion_piece(), Some(expected_kind));
            assert!(!m.is_capture());
        }
    }

    #[test]
    fn test_encode_decode_promotion_captures() {
        let cases = [
            (MoveFlag::KNIGHT_PROMOTION_CAPTURE, PieceKind::Knight),
            (MoveFlag::BISHOP_PROMOTION_CAPTURE, PieceKind::Bishop),
            (MoveFlag::ROOK_PROMOTION_CAPTURE, PieceKind::Rook),
            (MoveFlag::QUEEN_PROMOTION_CAPTURE, PieceKind::Queen),
        ];
        for (flag, expected_kind) in cases {
            let m = Move::new(Square::E7, Square::D8, flag);
            assert!(m.is_promotion());
            assert!(m.is_capture());
            assert_eq!(m.promotion_piece(), Some(expected_kind));
        }
    }

    #[test]
    fn test_boundary_squares() {
        let m = Move::new(Square::A1, Square::H8, MoveFlag::QUIET);
        assert_eq!(m.from_sq(), Square::A1);
        assert_eq!(m.to_sq(), Square::H8);

        let m = Move::new(Square::H8, Square::A1, MoveFlag::QUIET);
        assert_eq!(m.from_sq(), Square::H8);
        assert_eq!(m.to_sq(), Square::A1);
    }

    #[test]
    fn test_all_flags_distinct() {
        let flags = [
            MoveFlag::QUIET,
            MoveFlag::DOUBLE_PAWN_PUSH,
            MoveFlag::KING_CASTLE,
            MoveFlag::QUEEN_CASTLE,
            MoveFlag::CAPTURE,
            MoveFlag::EN_PASSANT,
            6, // reserved
            7, // reserved
            MoveFlag::KNIGHT_PROMOTION,
            MoveFlag::BISHOP_PROMOTION,
            MoveFlag::ROOK_PROMOTION,
            MoveFlag::QUEEN_PROMOTION,
            MoveFlag::KNIGHT_PROMOTION_CAPTURE,
            MoveFlag::BISHOP_PROMOTION_CAPTURE,
            MoveFlag::ROOK_PROMOTION_CAPTURE,
            MoveFlag::QUEEN_PROMOTION_CAPTURE,
        ];
        for i in 0..flags.len() {
            assert_eq!(flags[i], i as u16);
            for j in (i + 1)..flags.len() {
                assert_ne!(flags[i], flags[j]);
            }
        }
    }

    #[test]
    fn test_display_quiet() {
        let m = Move::new(Square::E2, Square::E4, MoveFlag::QUIET);
        assert_eq!(format!("{m}"), "e2e4");
    }

    #[test]
    fn test_display_promotion() {
        let cases = [
            (MoveFlag::KNIGHT_PROMOTION, "e7e8n"),
            (MoveFlag::BISHOP_PROMOTION, "e7e8b"),
            (MoveFlag::ROOK_PROMOTION, "e7e8r"),
            (MoveFlag::QUEEN_PROMOTION, "e7e8q"),
        ];
        for (flag, expected) in cases {
            let m = Move::new(Square::E7, Square::E8, flag);
            assert_eq!(format!("{m}"), expected);
        }
    }

    #[test]
    fn test_display_promotion_capture() {
        let m = Move::new(Square::E7, Square::D8, MoveFlag::QUEEN_PROMOTION_CAPTURE);
        assert_eq!(format!("{m}"), "e7d8q");
    }
}
