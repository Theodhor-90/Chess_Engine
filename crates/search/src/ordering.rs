use chess_board::Position;
use chess_types::Move;

use crate::killer::KillerTable;

const KILLER_SCORE: i32 = 50;

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

fn score_move(mv: Move, pos: &Position, killers: &KillerTable, ply: u8) -> i32 {
    if mv.is_capture() {
        score_mvv_lva(mv, pos)
    } else if killers.is_killer(ply, mv) {
        KILLER_SCORE
    } else {
        0
    }
}

pub fn order_moves(moves: &mut [Move], pos: &Position, killers: &KillerTable, ply: u8) {
    moves.sort_unstable_by(|a, b| {
        let sa = score_move(*a, pos, killers, ply);
        let sb = score_move(*b, pos, killers, ply);
        sb.cmp(&sa)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_board::Position;

    #[test]
    fn score_mvv_lva_pxq_higher_than_pxp() {
        let mut pos1 = Position::from_fen("4k3/8/3q4/4P3/8/8/8/4K3 w - - 0 1").expect("valid fen");
        let moves1 = chess_movegen::generate_legal_moves(&mut pos1);
        let pxq = moves1
            .iter()
            .find(|m| m.is_capture())
            .expect("pawn captures queen");
        let score_pxq = score_mvv_lva(*pxq, &pos1);

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
        let mut pos1 = Position::from_fen("4k3/8/4r3/3P4/8/8/8/4K3 w - - 0 1").expect("valid fen");
        let moves1 = chess_movegen::generate_legal_moves(&mut pos1);
        let pxr = moves1
            .iter()
            .find(|m| m.is_capture())
            .expect("pawn captures rook");
        let score_pxr = score_mvv_lva(*pxr, &pos1);

        let mut pos2 = Position::from_fen("4k3/8/4r3/8/3N4/8/8/4K3 w - - 0 1").expect("valid fen");
        let moves2 = chess_movegen::generate_legal_moves(&mut pos2);
        let target = chess_types::Square::new(44).unwrap();
        let nxr = moves2
            .iter()
            .find(|m| m.is_capture() && m.to_sq() == target)
            .expect("knight captures rook");
        let score_nxr = score_mvv_lva(*nxr, &pos2);

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
        let killers = KillerTable::new();

        order_moves(&mut moves, &pos, &killers, 0);

        let first_quiet_idx = moves.iter().position(|m| !m.is_capture());
        let last_capture_idx = moves.iter().rposition(|m| m.is_capture());

        if let (Some(first_quiet), Some(last_capture)) = (first_quiet_idx, last_capture_idx) {
            assert!(
                last_capture < first_quiet,
                "all captures should come before all quiet moves"
            );
        }
    }

    #[test]
    fn killer_moves_ordered_between_captures_and_quiet() {
        let mut pos = Position::from_fen("4k3/8/8/8/8/8/3q4/R3K3 w - - 0 1").expect("valid fen");
        let mut moves = chess_movegen::generate_legal_moves(&mut pos);

        let killer_mv = moves
            .iter()
            .find(|m| !m.is_capture())
            .copied()
            .expect("there must be a quiet move");

        let mut killers = KillerTable::new();
        killers.store(0, killer_mv);

        order_moves(&mut moves, &pos, &killers, 0);

        let killer_idx = moves
            .iter()
            .position(|m| *m == killer_mv)
            .expect("killer must be in list");

        let last_capture_idx = moves.iter().rposition(|m| m.is_capture());
        let first_other_quiet_idx = moves
            .iter()
            .position(|m| !m.is_capture() && *m != killer_mv);

        if let Some(last_cap) = last_capture_idx {
            assert!(
                killer_idx > last_cap,
                "killer should come after all captures"
            );
        }
        if let Some(first_quiet) = first_other_quiet_idx {
            assert!(
                killer_idx < first_quiet,
                "killer should come before other quiet moves"
            );
        }
    }
}
