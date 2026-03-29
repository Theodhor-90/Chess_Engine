pub mod material;
pub mod pawn;
pub mod phase;
pub mod pst;

pub use material::{BISHOP_VALUE, KNIGHT_VALUE, PAWN_VALUE, QUEEN_VALUE, ROOK_VALUE};

use chess_board::Position;
use chess_types::{Color, Piece, PieceKind};
use phase::{compute_phase, MAX_PHASE};

pub fn evaluate(pos: &Position) -> i32 {
    let mut mg_score: i32 = 0;
    let mut eg_score: i32 = 0;

    let kinds = [
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ];

    for kind in kinds {
        let mg_table = pst::mg_table(kind);
        let eg_table = pst::eg_table(kind);
        let mat_value = material::piece_value(kind);

        let white_bb = pos.piece_bitboard(Piece::new(Color::White, kind));
        for sq_idx in white_bb {
            let idx = sq_idx as usize;
            mg_score += mat_value + mg_table[idx];
            eg_score += mat_value + eg_table[idx];
        }

        let black_bb = pos.piece_bitboard(Piece::new(Color::Black, kind));
        for sq_idx in black_bb {
            let idx = pst::mirror_square(sq_idx as u8);
            mg_score -= mat_value + mg_table[idx];
            eg_score -= mat_value + eg_table[idx];
        }
    }

    let phase = compute_phase(pos);
    let score = ((mg_score * phase) + (eg_score * (MAX_PHASE - phase))) / MAX_PHASE;

    if pos.side_to_move() == Color::White {
        score
    } else {
        -score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startpos_eval_is_zero() {
        let pos =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(evaluate(&pos), 0);
    }

    #[test]
    fn white_advantage_positive_for_white() {
        let pos =
            Position::from_fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert!(evaluate(&pos) > 0);
    }

    #[test]
    fn white_advantage_negative_for_black() {
        let pos =
            Position::from_fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1").unwrap();
        assert!(evaluate(&pos) < 0);
    }

    #[test]
    fn centralized_knight_scores_higher_than_rim_knight() {
        // White knight on d4 (centralized) vs White knight on a1 (rim)
        // Both positions: White has king + knight, Black has king only
        let centralized = Position::from_fen("4k3/8/8/8/3N4/8/8/4K3 w - - 0 1").unwrap();
        let rim = Position::from_fen("4k3/8/8/8/8/8/8/N3K3 w - - 0 1").unwrap();
        assert!(evaluate(&centralized) > evaluate(&rim));
    }

    #[test]
    fn tapered_eval_interpolation() {
        // Verify the tapered formula produces correct intermediate values.
        // Position has one white knight → phase = 1 (knight weight = 1).
        let pos = Position::from_fen("4k3/8/8/8/3N4/8/8/4K3 w - - 0 1").unwrap();

        // mg_score = 333, eg_score = 345, phase = 1
        // score = (333 * 1 + 345 * 23) / 24 = 8268 / 24 = 344
        assert_eq!(evaluate(&pos), 344);
    }

    #[test]
    fn pst_bonuses_are_additive_with_material() {
        // Position with extra material AND better placement should score higher
        // than a position with only extra material.
        // Both have White extra knight vs Black, but different knight placement.
        let good_placement =
            Position::from_fen("rnbqkb1r/pppppppp/8/8/3N4/8/PPPPPPPP/RNBQKB1R w KQkq - 0 1")
                .unwrap();
        let bad_placement =
            Position::from_fen("rnbqkb1r/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();

        // Both have material advantage (missing Black knight), but centralized
        // knight should produce a higher score than one on g1.
        let good_score = evaluate(&good_placement);
        let bad_score = evaluate(&bad_placement);
        assert!(good_score > 0);
        assert!(bad_score > 0);
        assert!(good_score > bad_score);
    }
}
