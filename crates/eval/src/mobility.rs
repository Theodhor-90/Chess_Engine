use chess_board::Position;
use chess_movegen::knights::KNIGHT_ATTACKS;
use chess_movegen::magic::{bishop_attacks, rook_attacks};
use chess_types::{Bitboard, Color, Piece, PieceKind, Square};

const KNIGHT_MOBILITY_MG: i32 = 4;
const KNIGHT_MOBILITY_EG: i32 = 4;
const BISHOP_MOBILITY_MG: i32 = 5;
const BISHOP_MOBILITY_EG: i32 = 5;
const ROOK_MOBILITY_MG: i32 = 2;
const ROOK_MOBILITY_EG: i32 = 3;
const QUEEN_MOBILITY_MG: i32 = 1;
const QUEEN_MOBILITY_EG: i32 = 2;

const NOT_A_FILE: Bitboard = Bitboard::new(0xFEFE_FEFE_FEFE_FEFE);
const NOT_H_FILE: Bitboard = Bitboard::new(0x7F7F_7F7F_7F7F_7F7F);

fn pawn_attacks(pawns: Bitboard, color: Color) -> Bitboard {
    match color {
        Color::White => ((pawns & NOT_A_FILE) << 7) | ((pawns & NOT_H_FILE) << 9),
        Color::Black => ((pawns & NOT_H_FILE) >> 7) | ((pawns & NOT_A_FILE) >> 9),
    }
}

pub fn evaluate_mobility(pos: &Position, color: Color) -> (i32, i32) {
    let opponent = color.opposite();
    let opponent_pawns = pos.piece_bitboard(Piece::new(opponent, PieceKind::Pawn));
    let opp_pawn_attacks = pawn_attacks(opponent_pawns, opponent);
    let safe_squares = !opp_pawn_attacks;
    let occupied = pos.occupied();

    let mut mg_score: i32 = 0;
    let mut eg_score: i32 = 0;

    let knights = pos.piece_bitboard(Piece::new(color, PieceKind::Knight));
    for sq_idx in knights {
        let mobility = (KNIGHT_ATTACKS[sq_idx as usize] & safe_squares).pop_count() as i32;
        mg_score += mobility * KNIGHT_MOBILITY_MG;
        eg_score += mobility * KNIGHT_MOBILITY_EG;
    }

    let bishops = pos.piece_bitboard(Piece::new(color, PieceKind::Bishop));
    for sq_idx in bishops {
        let sq = Square::new(sq_idx as u8).unwrap();
        let attacks = bishop_attacks(sq, occupied);
        let mobility = (attacks & safe_squares).pop_count() as i32;
        mg_score += mobility * BISHOP_MOBILITY_MG;
        eg_score += mobility * BISHOP_MOBILITY_EG;
    }

    let rooks = pos.piece_bitboard(Piece::new(color, PieceKind::Rook));
    for sq_idx in rooks {
        let sq = Square::new(sq_idx as u8).unwrap();
        let attacks = rook_attacks(sq, occupied);
        let mobility = (attacks & safe_squares).pop_count() as i32;
        mg_score += mobility * ROOK_MOBILITY_MG;
        eg_score += mobility * ROOK_MOBILITY_EG;
    }

    let queens = pos.piece_bitboard(Piece::new(color, PieceKind::Queen));
    for sq_idx in queens {
        let sq = Square::new(sq_idx as u8).unwrap();
        let b_attacks = bishop_attacks(sq, occupied);
        let r_attacks = rook_attacks(sq, occupied);
        let mobility = ((b_attacks | r_attacks) & safe_squares).pop_count() as i32;
        mg_score += mobility * QUEEN_MOBILITY_MG;
        eg_score += mobility * QUEEN_MOBILITY_EG;
    }

    (mg_score, eg_score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startpos_mobility_symmetric() {
        let pos =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(
            evaluate_mobility(&pos, Color::White),
            evaluate_mobility(&pos, Color::Black)
        );
    }

    #[test]
    fn open_position_higher_mobility_than_closed() {
        let open = Position::from_fen("4k3/8/8/8/8/8/8/4K2R w - - 0 1").unwrap();
        let closed = Position::from_fen("4k3/8/8/8/8/8/6PP/4K2R w - - 0 1").unwrap();
        let (open_mg, _) = evaluate_mobility(&open, Color::White);
        let (closed_mg, _) = evaluate_mobility(&closed, Color::White);
        assert!(
            open_mg > closed_mg,
            "open mg {open_mg} should exceed closed mg {closed_mg}"
        );
    }

    #[test]
    fn opponent_pawn_attacks_excluded() {
        let with_pawns = Position::from_fen("4k3/8/8/3pp3/8/4N3/8/4K3 w - - 0 1").unwrap();
        let without_pawns = Position::from_fen("4k3/8/8/8/8/4N3/8/4K3 w - - 0 1").unwrap();
        let (with_mg, _) = evaluate_mobility(&with_pawns, Color::White);
        let (without_mg, _) = evaluate_mobility(&without_pawns, Color::White);
        assert!(
            with_mg < without_mg,
            "with_pawns mg {with_mg} should be less than without_pawns mg {without_mg}"
        );
    }

    #[test]
    fn queen_mobility_scaled_less_than_knight() {
        let knight_pos = Position::from_fen("4k3/8/8/8/4N3/8/8/4K3 w - - 0 1").unwrap();
        let queen_pos = Position::from_fen("4k3/8/8/8/4Q3/8/8/4K3 w - - 0 1").unwrap();
        let (knight_mg, _) = evaluate_mobility(&knight_pos, Color::White);
        let (queen_mg, _) = evaluate_mobility(&queen_pos, Color::White);
        assert!(
            knight_mg > queen_mg,
            "knight mg {knight_mg} should exceed queen mg {queen_mg}"
        );
    }

    #[test]
    fn mobility_produces_midgame_and_endgame_scores() {
        let pos = Position::from_fen("4k3/8/8/8/4N3/8/8/4K3 w - - 0 1").unwrap();
        let (mg, eg) = evaluate_mobility(&pos, Color::White);
        assert_eq!(mg, 32);
        assert_eq!(eg, 32);
    }
}
