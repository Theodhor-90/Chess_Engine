use chess_board::{CastlingRights, Position};
use chess_types::{Bitboard, Color, Move, MoveFlag, Square};

const WHITE_KS_PATH: Bitboard = Bitboard::new((1u64 << 5) | (1u64 << 6)); // F1, G1
const WHITE_QS_PATH: Bitboard = Bitboard::new((1u64 << 1) | (1u64 << 2) | (1u64 << 3)); // B1, C1, D1
const BLACK_KS_PATH: Bitboard = Bitboard::new((1u64 << 61) | (1u64 << 62)); // F8, G8
const BLACK_QS_PATH: Bitboard = Bitboard::new((1u64 << 57) | (1u64 << 58) | (1u64 << 59)); // B8, C8, D8

const WHITE_KS_KING_TRANSIT: [Square; 3] = [Square::E1, Square::F1, Square::G1];
const WHITE_QS_KING_TRANSIT: [Square; 3] = [Square::E1, Square::D1, Square::C1];
const BLACK_KS_KING_TRANSIT: [Square; 3] = [Square::E8, Square::F8, Square::G8];
const BLACK_QS_KING_TRANSIT: [Square; 3] = [Square::E8, Square::D8, Square::C8];

pub fn generate_castling_moves(pos: &Position, moves: &mut Vec<Move>) {
    match pos.side_to_move() {
        Color::White => {
            if pos
                .castling_rights()
                .contains(CastlingRights::WHITE_KINGSIDE)
                && (pos.occupied() & WHITE_KS_PATH).is_empty()
                && !WHITE_KS_KING_TRANSIT
                    .iter()
                    .any(|&sq| pos.is_square_attacked(sq, Color::Black))
            {
                moves.push(Move::new(Square::E1, Square::G1, MoveFlag::KING_CASTLE));
            }
            if pos
                .castling_rights()
                .contains(CastlingRights::WHITE_QUEENSIDE)
                && (pos.occupied() & WHITE_QS_PATH).is_empty()
                && !WHITE_QS_KING_TRANSIT
                    .iter()
                    .any(|&sq| pos.is_square_attacked(sq, Color::Black))
            {
                moves.push(Move::new(Square::E1, Square::C1, MoveFlag::QUEEN_CASTLE));
            }
        }
        Color::Black => {
            if pos
                .castling_rights()
                .contains(CastlingRights::BLACK_KINGSIDE)
                && (pos.occupied() & BLACK_KS_PATH).is_empty()
                && !BLACK_KS_KING_TRANSIT
                    .iter()
                    .any(|&sq| pos.is_square_attacked(sq, Color::White))
            {
                moves.push(Move::new(Square::E8, Square::G8, MoveFlag::KING_CASTLE));
            }
            if pos
                .castling_rights()
                .contains(CastlingRights::BLACK_QUEENSIDE)
                && (pos.occupied() & BLACK_QS_PATH).is_empty()
                && !BLACK_QS_KING_TRANSIT
                    .iter()
                    .any(|&sq| pos.is_square_attacked(sq, Color::White))
            {
                moves.push(Move::new(Square::E8, Square::C8, MoveFlag::QUEEN_CASTLE));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_board::Position;

    fn has_move(moves: &[Move], from: Square, to: Square, flags: u16) -> bool {
        moves.contains(&Move::new(from, to, flags))
    }

    #[test]
    fn test_white_kingside_castling() {
        let pos = Position::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1")
            .expect("valid fen");
        let mut moves = Vec::new();
        generate_castling_moves(&pos, &mut moves);
        assert!(has_move(
            &moves,
            Square::E1,
            Square::G1,
            MoveFlag::KING_CASTLE
        ));
    }

    #[test]
    fn test_white_queenside_castling() {
        let pos = Position::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1")
            .expect("valid fen");
        let mut moves = Vec::new();
        generate_castling_moves(&pos, &mut moves);
        assert!(has_move(
            &moves,
            Square::E1,
            Square::C1,
            MoveFlag::QUEEN_CASTLE
        ));
    }

    #[test]
    fn test_black_kingside_castling() {
        let pos = Position::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1")
            .expect("valid fen");
        let mut moves = Vec::new();
        generate_castling_moves(&pos, &mut moves);
        assert!(has_move(
            &moves,
            Square::E8,
            Square::G8,
            MoveFlag::KING_CASTLE
        ));
    }

    #[test]
    fn test_black_queenside_castling() {
        let pos = Position::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1")
            .expect("valid fen");
        let mut moves = Vec::new();
        generate_castling_moves(&pos, &mut moves);
        assert!(has_move(
            &moves,
            Square::E8,
            Square::C8,
            MoveFlag::QUEEN_CASTLE
        ));
    }

    #[test]
    fn test_castling_blocked_by_pieces() {
        let pos = Position::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R2QK2R w KQkq - 0 1")
            .expect("valid fen");
        let mut moves = Vec::new();
        generate_castling_moves(&pos, &mut moves);
        assert!(!has_move(
            &moves,
            Square::E1,
            Square::C1,
            MoveFlag::QUEEN_CASTLE
        ));
        assert!(has_move(
            &moves,
            Square::E1,
            Square::G1,
            MoveFlag::KING_CASTLE
        ));
    }

    #[test]
    fn test_castling_blocked_by_attacked_origin() {
        let pos = Position::from_fen("r3k2r/pppppppp/8/8/4r3/8/PPPP1PPP/R3K2R w KQkq - 0 1")
            .expect("valid fen");
        let mut moves = Vec::new();
        generate_castling_moves(&pos, &mut moves);
        assert!(!has_move(
            &moves,
            Square::E1,
            Square::G1,
            MoveFlag::KING_CASTLE
        ));
        assert!(!has_move(
            &moves,
            Square::E1,
            Square::C1,
            MoveFlag::QUEEN_CASTLE
        ));
    }

    #[test]
    fn test_castling_blocked_by_attacked_transit() {
        let pos = Position::from_fen("r3k2r/pppppppp/8/8/8/6n1/PPPPPPPP/R3K2R w KQkq - 0 1")
            .expect("valid fen");
        let mut moves = Vec::new();
        generate_castling_moves(&pos, &mut moves);
        assert!(!has_move(
            &moves,
            Square::E1,
            Square::G1,
            MoveFlag::KING_CASTLE
        ));
    }

    #[test]
    fn test_castling_blocked_by_attacked_destination() {
        let pos = Position::from_fen("r3k2r/pppppppp/8/8/8/5n2/PPPPPPPP/R3K2R w KQkq - 0 1")
            .expect("valid fen");
        let mut moves = Vec::new();
        generate_castling_moves(&pos, &mut moves);
        assert!(!has_move(
            &moves,
            Square::E1,
            Square::G1,
            MoveFlag::KING_CASTLE
        ));
    }

    #[test]
    fn test_castling_rights_revoked() {
        let pos = Position::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w - - 0 1")
            .expect("valid fen");
        let mut moves = Vec::new();
        generate_castling_moves(&pos, &mut moves);
        assert!(moves.is_empty());
    }
}
