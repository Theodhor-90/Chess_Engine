use chess_board::Position;
use chess_types::{Color, Piece, PieceKind};

pub const PAWN_VALUE: i32 = 93;
pub const KNIGHT_VALUE: i32 = 316;
pub const BISHOP_VALUE: i32 = 344;
pub const ROOK_VALUE: i32 = 510;
pub const QUEEN_VALUE: i32 = 936;

pub fn piece_value(kind: PieceKind) -> i32 {
    match kind {
        PieceKind::Pawn => PAWN_VALUE,
        PieceKind::Knight => KNIGHT_VALUE,
        PieceKind::Bishop => BISHOP_VALUE,
        PieceKind::Rook => ROOK_VALUE,
        PieceKind::Queen => QUEEN_VALUE,
        PieceKind::King => 0,
    }
}

pub fn material_score(pos: &Position) -> i32 {
    let mut score = 0i32;
    let kinds = [
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
    ];
    for kind in kinds {
        let white_count = pos
            .piece_bitboard(Piece::new(Color::White, kind))
            .pop_count() as i32;
        let black_count = pos
            .piece_bitboard(Piece::new(Color::Black, kind))
            .pop_count() as i32;
        score += (white_count - black_count) * piece_value(kind);
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_material_is_zero() {
        let pos =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(material_score(&pos), 0);
    }

    #[test]
    fn white_extra_queen() {
        let pos =
            Position::from_fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(material_score(&pos), QUEEN_VALUE);
    }

    #[test]
    fn black_extra_knight() {
        let pos = Position::from_fen("rnbqkbnr/pppppppp/8/3n4/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap();
        assert_eq!(material_score(&pos), -KNIGHT_VALUE);
    }
}
