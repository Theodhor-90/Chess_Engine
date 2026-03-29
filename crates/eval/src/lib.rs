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

    struct TermBreakdown {
        material: i32,
        pawn_structure: i32,
        king_safety: i32,
        mobility: i32,
        piece_bonuses: i32,
        king_centralization: i32,
    }

    fn eval_term_contributions(pos: &Position, pawn_table: &mut PawnHashTable) -> TermBreakdown {
        let phase = compute_phase(pos);

        let mut mat_mg: i32 = 0;
        let mut mat_eg: i32 = 0;
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
                mat_mg += mat_value + mg_table[idx];
                mat_eg += mat_value + eg_table[idx];
            }
            let black_bb = pos.piece_bitboard(Piece::new(Color::Black, kind));
            for sq_idx in black_bb {
                let idx = pst::mirror_square(sq_idx as u8);
                mat_mg -= mat_value + mg_table[idx];
                mat_eg -= mat_value + eg_table[idx];
            }
        }
        let material = ((mat_mg * phase) + (mat_eg * (MAX_PHASE - phase))) / MAX_PHASE;

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
        let ps_mg = pawn_mg + pp_extra_mg;
        let ps_eg = pawn_eg + pp_extra_eg;
        let pawn_structure = ((ps_mg * phase) + (ps_eg * (MAX_PHASE - phase))) / MAX_PHASE;

        let (w_ks_mg, w_ks_eg) = king_safety::evaluate_king_safety(pos, Color::White);
        let (b_ks_mg, b_ks_eg) = king_safety::evaluate_king_safety(pos, Color::Black);
        let ks_mg = w_ks_mg - b_ks_mg;
        let ks_eg = w_ks_eg - b_ks_eg;
        let king_safety = ((ks_mg * phase) + (ks_eg * (MAX_PHASE - phase))) / MAX_PHASE;

        let (w_mob_mg, w_mob_eg) = mobility::evaluate_mobility(pos, Color::White);
        let (b_mob_mg, b_mob_eg) = mobility::evaluate_mobility(pos, Color::Black);
        let mob_mg = w_mob_mg - b_mob_mg;
        let mob_eg = w_mob_eg - b_mob_eg;
        let mobility = ((mob_mg * phase) + (mob_eg * (MAX_PHASE - phase))) / MAX_PHASE;

        let (w_pc_mg, w_pc_eg) = pieces::evaluate_piece_bonuses(pos, Color::White);
        let (b_pc_mg, b_pc_eg) = pieces::evaluate_piece_bonuses(pos, Color::Black);
        let pc_mg = w_pc_mg - b_pc_mg;
        let pc_eg = w_pc_eg - b_pc_eg;
        let piece_bonuses = ((pc_mg * phase) + (pc_eg * (MAX_PHASE - phase))) / MAX_PHASE;

        let (kc_mg, kc_eg) = king_centralization::evaluate_king_centralization(pos);
        let king_centralization = ((kc_mg * phase) + (kc_eg * (MAX_PHASE - phase))) / MAX_PHASE;

        TermBreakdown {
            material,
            pawn_structure,
            king_safety,
            mobility,
            piece_bonuses,
            king_centralization,
        }
    }

    const REFERENCE_FENS: [&str; 6] = [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r1bqkb1r/pppppppp/2n2n2/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3",
        "r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
        "rnbqkb1r/pp2pppp/2p2n2/3p4/2PP4/5N2/PP2PPPP/RNBQKB1R w KQkq - 0 4",
        "rnbqk2r/ppppbppp/4pn2/8/2PP4/5N2/PP2PPPP/RNBQKB1R w KQkq - 2 4",
        "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3",
    ];

    #[test]
    fn weight_balance_no_term_exceeds_50_percent() {
        let mut pt = PawnHashTable::new();
        for fen in REFERENCE_FENS {
            let pos = Position::from_fen(fen).unwrap();
            let b = eval_term_contributions(&pos, &mut pt);
            let terms = [
                ("pawn_structure", b.pawn_structure),
                ("king_safety", b.king_safety),
                ("mobility", b.mobility),
                ("piece_bonuses", b.piece_bonuses),
                ("king_centralization", b.king_centralization),
            ];
            let non_zero_count = terms.iter().filter(|(_, v)| *v != 0).count();
            if non_zero_count < 2 {
                continue;
            }
            let total_abs: i32 = terms.iter().map(|(_, v)| v.abs()).sum();
            if total_abs < 20 {
                continue;
            }
            for (name, val) in &terms {
                assert!(
                    val.abs() * 2 <= total_abs,
                    "FEN: {fen}\n{name} abs {} exceeds 50% of total {total_abs}",
                    val.abs()
                );
            }
        }
    }

    #[test]
    fn king_safety_less_than_material_in_quiet_positions() {
        let mut pt = PawnHashTable::new();
        for fen in REFERENCE_FENS {
            let pos = Position::from_fen(fen).unwrap();
            let b = eval_term_contributions(&pos, &mut pt);
            if b.king_safety == 0 || b.material.abs() < 10 {
                continue;
            }
            assert!(
                b.king_safety.abs() < b.material.abs(),
                "FEN: {fen}\nking_safety abs {} >= material abs {}",
                b.king_safety.abs(),
                b.material.abs()
            );
        }
    }

    #[test]
    fn phase_interpolation_opening() {
        let pos = Position::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1")
            .unwrap();
        let phase = compute_phase(&pos);
        assert!(phase >= 20, "Opening phase should be >= 20, got {phase}");
    }

    #[test]
    fn phase_interpolation_middlegame() {
        let pos = Position::from_fen("r1b1k3/pppp1ppp/8/8/8/8/PPPP1PPP/R1B1K3 w - - 0 1").unwrap();
        let phase = compute_phase(&pos);
        assert!(
            phase > 2 && phase < 20,
            "Middlegame phase should be between 2 and 20 exclusive, got {phase}"
        );
    }

    #[test]
    fn phase_interpolation_endgame() {
        let pos = Position::from_fen("4k3/pppp4/8/8/8/8/PPPP4/4K3 w - - 0 1").unwrap();
        let phase = compute_phase(&pos);
        assert_eq!(phase, 0, "Endgame phase should be 0, got {phase}");
    }

    #[test]
    fn all_m07_terms_contribute_mg_and_eg() {
        // Use Slav Defense — asymmetric pawn structure ensures non-zero pawn eval
        let pos =
            Position::from_fen("rnbqkb1r/pp2pppp/2p2n2/3p4/2PP4/5N2/PP2PPPP/RNBQKB1R w KQkq - 0 4")
                .unwrap();

        let (pawn_mg, pawn_eg) = pawn::evaluate_pawns(&pos);
        let (pp_mg, pp_eg) = pawn::evaluate_passed_pawn_extras(&pos);
        assert!(
            pawn_mg != 0 || pawn_eg != 0 || pp_mg != 0 || pp_eg != 0,
            "pawn eval should produce non-zero MG or EG: mg={pawn_mg} eg={pawn_eg} pp_mg={pp_mg} pp_eg={pp_eg}"
        );

        let (w_ks_mg, w_ks_eg) = king_safety::evaluate_king_safety(&pos, Color::White);
        assert!(
            w_ks_mg != 0 || w_ks_eg != 0,
            "king_safety should produce non-zero MG or EG per color"
        );

        let (w_mob_mg, w_mob_eg) = mobility::evaluate_mobility(&pos, Color::White);
        assert!(
            w_mob_mg != 0 || w_mob_eg != 0,
            "mobility should produce non-zero MG or EG per color"
        );

        let (w_pc_mg, w_pc_eg) = pieces::evaluate_piece_bonuses(&pos, Color::White);
        let (b_pc_mg, b_pc_eg) = pieces::evaluate_piece_bonuses(&pos, Color::Black);
        assert!(
            w_pc_mg != 0 || w_pc_eg != 0 || b_pc_mg != 0 || b_pc_eg != 0,
            "piece_bonuses should produce non-zero MG or EG for at least one color"
        );
    }

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
