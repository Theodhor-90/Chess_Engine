pub mod endgame;
pub mod king_centralization;
pub mod king_safety;
pub mod material;
pub mod mobility;
pub mod pawn;
pub mod phase;
pub mod pieces;
pub mod pst;

pub use material::{BISHOP_VALUE, KNIGHT_VALUE, PAWN_VALUE, QUEEN_VALUE, ROOK_VALUE};
pub use pawn::PawnHashTable;

use chess_board::Position;
use chess_types::{Color, Piece, PieceKind};
use phase::{compute_phase, MAX_PHASE};

pub fn evaluate(pos: &Position, pawn_table: &mut PawnHashTable) -> i32 {
    if let Some(eg_score) = endgame::probe_endgame(pos) {
        return if pos.side_to_move() == Color::White {
            eg_score
        } else {
            -eg_score
        };
    }

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

    let pawn_key = pawn::pawn_zobrist_hash(pos);
    let (pawn_mg, pawn_eg) = match pawn_table.probe(pawn_key) {
        Some(scores) => scores,
        None => {
            let scores = pawn::evaluate_pawns(pos);
            pawn_table.store(pawn_key, scores.0, scores.1);
            scores
        }
    };

    let (pp_extra_mg, pp_extra_eg) = pawn::evaluate_passed_pawn_extras(pos);

    mg_score += pawn_mg + pp_extra_mg;
    eg_score += pawn_eg + pp_extra_eg;

    let (w_ks_mg, w_ks_eg) = king_safety::evaluate_king_safety(pos, Color::White);
    let (b_ks_mg, b_ks_eg) = king_safety::evaluate_king_safety(pos, Color::Black);
    mg_score += w_ks_mg - b_ks_mg;
    eg_score += w_ks_eg - b_ks_eg;

    let (w_mob_mg, w_mob_eg) = mobility::evaluate_mobility(pos, Color::White);
    let (b_mob_mg, b_mob_eg) = mobility::evaluate_mobility(pos, Color::Black);
    mg_score += w_mob_mg - b_mob_mg;
    eg_score += w_mob_eg - b_mob_eg;

    let (w_pc_mg, w_pc_eg) = pieces::evaluate_piece_bonuses(pos, Color::White);
    let (b_pc_mg, b_pc_eg) = pieces::evaluate_piece_bonuses(pos, Color::Black);
    mg_score += w_pc_mg - b_pc_mg;
    eg_score += w_pc_eg - b_pc_eg;

    let (kc_mg, kc_eg) = king_centralization::evaluate_king_centralization(pos);
    mg_score += kc_mg;
    eg_score += kc_eg;

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
        assert_eq!(evaluate(&pos, &mut PawnHashTable::new()), 0);
    }

    #[test]
    fn white_advantage_positive_for_white() {
        let pos =
            Position::from_fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert!(evaluate(&pos, &mut PawnHashTable::new()) > 0);
    }

    #[test]
    fn white_advantage_negative_for_black() {
        let pos =
            Position::from_fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1").unwrap();
        assert!(evaluate(&pos, &mut PawnHashTable::new()) < 0);
    }

    #[test]
    fn centralized_knight_scores_higher_than_rim_knight() {
        let centralized = Position::from_fen("4k3/8/8/8/3N4/8/4P3/4K3 w - - 0 1").unwrap();
        let rim = Position::from_fen("4k3/8/8/8/8/8/4P3/N3K3 w - - 0 1").unwrap();
        let mut pt = PawnHashTable::new();
        assert!(evaluate(&centralized, &mut pt) > evaluate(&rim, &mut pt));
    }

    #[test]
    fn tapered_eval_interpolation() {
        let pos = Position::from_fen("4k3/8/8/8/3N4/8/4P3/4K3 w - - 0 1").unwrap();
        let score = evaluate(&pos, &mut PawnHashTable::new());
        assert!(
            score > 0,
            "KNP vs K should be positive for white, got {score}"
        );
    }

    #[test]
    fn pst_bonuses_are_additive_with_material() {
        let good_placement =
            Position::from_fen("rnbqkb1r/pppppppp/8/8/3N4/8/PPPPPPPP/RNBQKB1R w KQkq - 0 1")
                .unwrap();
        let bad_placement =
            Position::from_fen("rnbqkb1r/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let mut pt = PawnHashTable::new();
        let good_score = evaluate(&good_placement, &mut pt);
        let bad_score = evaluate(&bad_placement, &mut pt);
        assert!(good_score > 0);
        assert!(bad_score > 0);
        assert!(good_score > bad_score);
    }

    #[test]
    fn passed_pawn_rank6_scores_higher_than_rank4() {
        let rank6 = Position::from_fen("4k3/8/3P4/8/8/8/8/4K3 w - - 0 1").unwrap();
        let rank4 = Position::from_fen("4k3/8/8/8/3P4/8/8/4K3 w - - 0 1").unwrap();
        let mut pt = PawnHashTable::new();
        let score6 = evaluate(&rank6, &mut pt);
        let score4 = evaluate(&rank4, &mut pt);
        assert!(
            score6 > score4,
            "rank6 score {score6} should exceed rank4 score {score4}"
        );
    }

    #[test]
    fn pawn_hash_end_to_end() {
        let pos = Position::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1")
            .unwrap();
        let mut pt = PawnHashTable::new();
        let score1 = evaluate(&pos, &mut pt);
        let score2 = evaluate(&pos, &mut pt);
        assert_eq!(score1, score2);
    }
}
