use chess_board::Position;
use chess_types::Move;

pub fn score_mvv_lva(mv: Move, pos: &Position) -> i32 {
    if !mv.is_capture() {
        return 0;
    }

    let attacker_kind = pos
        .piece_on(mv.from_sq())
        .expect("attacker must exist on from_sq")
        .kind;

    let victim_kind = if mv.is_en_passant() {
        chess_types::PieceKind::Pawn
    } else {
        pos.piece_on(mv.to_sq())
            .expect("capture target must exist on to_sq")
            .kind
    };

    chess_eval::material::piece_value(victim_kind) * 100
        - chess_eval::material::piece_value(attacker_kind)
}

pub fn order_moves(moves: &mut [Move], pos: &Position) {
    moves.sort_unstable_by(|a, b| {
        let sa = score_mvv_lva(*a, pos);
        let sb = score_mvv_lva(*b, pos);
        sb.cmp(&sa)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_board::Position;

    #[test]
    fn score_mvv_lva_pxq_higher_than_pxp() {
        // Position 1: white pawn on e5 captures black queen on d6
        let mut pos1 = Position::from_fen("4k3/8/3q4/4P3/8/8/8/4K3 w - - 0 1").expect("valid fen");
        let moves1 = chess_movegen::generate_legal_moves(&mut pos1);
        let pxq = moves1
            .iter()
            .find(|m| m.is_capture())
            .expect("pawn captures queen");
        let score_pxq = score_mvv_lva(*pxq, &pos1);

        // Position 2: white pawn on e5 captures black pawn on d6
        let mut pos2 = Position::from_fen("4k3/8/3p4/4P3/8/8/8/4K3 w - - 0 1").expect("valid fen");
        let moves2 = chess_movegen::generate_legal_moves(&mut pos2);
        let pxp = moves2
            .iter()
            .find(|m| m.is_capture())
            .expect("pawn captures pawn");
        let score_pxp = score_mvv_lva(*pxp, &pos2);

        assert!(score_pxq > score_pxp);
    }

    #[test]
    fn score_mvv_lva_lower_attacker_scores_higher() {
        // Position 1: white pawn on d5 captures black rook on e6
        let mut pos1 = Position::from_fen("4k3/8/4r3/3P4/8/8/8/4K3 w - - 0 1").expect("valid fen");
        let moves1 = chess_movegen::generate_legal_moves(&mut pos1);
        let pxr = moves1
            .iter()
            .find(|m| m.is_capture())
            .expect("pawn captures rook");
        let score_pxr = score_mvv_lva(*pxr, &pos1);

        // Position 2: white knight on d4 captures black rook on e6
        let mut pos2 = Position::from_fen("4k3/8/4r3/8/3N4/8/8/4K3 w - - 0 1").expect("valid fen");
        let moves2 = chess_movegen::generate_legal_moves(&mut pos2);
        let target = chess_types::Square::new(44).unwrap(); // e6
        let nxr = moves2
            .iter()
            .find(|m| m.is_capture() && m.to_sq() == target)
            .expect("knight captures rook");
        let score_nxr = score_mvv_lva(*nxr, &pos2);

        // PxR should score higher than NxR (same victim, lower-value attacker preferred)
        assert!(score_pxr > score_nxr);
    }

    #[test]
    fn score_mvv_lva_quiet_returns_zero() {
        let mut pos = Position::startpos();
        let moves = chess_movegen::generate_legal_moves(&mut pos);

        let quiet = moves
            .iter()
            .find(|m| !m.is_capture())
            .expect("there are quiet moves from startpos");

        assert_eq!(score_mvv_lva(*quiet, &pos), 0);
    }

    #[test]
    fn order_moves_captures_before_quiet() {
        let mut pos = Position::from_fen("4k3/8/8/8/8/8/3q4/R3K3 w - - 0 1").expect("valid fen");
        let mut moves = chess_movegen::generate_legal_moves(&mut pos);

        order_moves(&mut moves, &pos);

        let first_quiet_idx = moves.iter().position(|m| !m.is_capture());
        let last_capture_idx = moves.iter().rposition(|m| m.is_capture());

        if let (Some(first_quiet), Some(last_capture)) = (first_quiet_idx, last_capture_idx) {
            assert!(
                last_capture < first_quiet,
                "all captures should come before all quiet moves"
            );
        }
    }
}
