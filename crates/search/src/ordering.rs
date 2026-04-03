use chess_board::Position;
use chess_types::{Color, Move, PieceKind};

use crate::countermove::CounterMoveTable;
use crate::history::HistoryTable;
use crate::killer::KillerTable;
use crate::see;

const TT_SCORE: i32 = 300_000;
const PV_SCORE: i32 = 200_000;
const CAPTURE_BASE: i32 = 100_000;
const KILLER_SCORE: i32 = 20_000;
const COUNTER_MOVE_SCORE: i32 = 15_000;
const BAD_CAPTURE_BASE: i32 = -20_000;

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

#[allow(clippy::too_many_arguments)]
fn score_move(
    mv: Move,
    pos: &Position,
    killers: &KillerTable,
    history: &HistoryTable,
    countermoves: &CounterMoveTable,
    ply: u8,
    pv_move: Option<Move>,
    tt_move: Option<Move>,
    side: Color,
    prev_move: Option<Move>,
    prev_piece: Option<PieceKind>,
) -> i32 {
    if tt_move == Some(mv) {
        TT_SCORE
    } else if pv_move == Some(mv) {
        PV_SCORE
    } else if mv.is_capture() {
        let see_score = see::see(pos, mv);
        if see_score >= 0 {
            CAPTURE_BASE + score_mvv_lva(mv, pos)
        } else {
            BAD_CAPTURE_BASE + see_score
        }
    } else if killers.is_killer(ply, mv) {
        KILLER_SCORE
    } else if let (Some(pm_piece), Some(pm)) = (prev_piece, prev_move) {
        if countermoves.lookup(pm_piece, pm.to_sq()) == Some(mv) {
            COUNTER_MOVE_SCORE
        } else {
            history.score(side, mv.from_sq(), mv.to_sq())
        }
    } else {
        history.score(side, mv.from_sq(), mv.to_sq())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn order_moves(
    moves: &mut [Move],
    pos: &Position,
    killers: &KillerTable,
    history: &HistoryTable,
    countermoves: &CounterMoveTable,
    ply: u8,
    pv_move: Option<Move>,
    tt_move: Option<Move>,
    side: Color,
    prev_move: Option<Move>,
    prev_piece: Option<PieceKind>,
) {
    moves.sort_unstable_by(|a, b| {
        let sa = score_move(
            *a,
            pos,
            killers,
            history,
            countermoves,
            ply,
            pv_move,
            tt_move,
            side,
            prev_move,
            prev_piece,
        );
        let sb = score_move(
            *b,
            pos,
            killers,
            history,
            countermoves,
            ply,
            pv_move,
            tt_move,
            side,
            prev_move,
            prev_piece,
        );
        sb.cmp(&sa)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::countermove::CounterMoveTable;
    use crate::history::HistoryTable;
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
        let history = HistoryTable::new();
        let countermoves = CounterMoveTable::new();

        order_moves(
            &mut moves,
            &pos,
            &killers,
            &history,
            &countermoves,
            0,
            None,
            None,
            Color::White,
            None,
            None,
        );

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
        let history = HistoryTable::new();
        let countermoves = CounterMoveTable::new();

        order_moves(
            &mut moves,
            &pos,
            &killers,
            &history,
            &countermoves,
            0,
            None,
            None,
            Color::White,
            None,
            None,
        );

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

    #[test]
    fn pv_move_ordered_first() {
        let mut pos = Position::from_fen("4k3/8/8/8/8/8/3q4/R3K3 w - - 0 1").expect("valid fen");
        let mut moves = chess_movegen::generate_legal_moves(&mut pos);

        let pv_mv = moves
            .iter()
            .find(|m| !m.is_capture())
            .copied()
            .expect("there must be a quiet move");

        let killers = KillerTable::new();
        let history = HistoryTable::new();
        let countermoves = CounterMoveTable::new();
        order_moves(
            &mut moves,
            &pos,
            &killers,
            &history,
            &countermoves,
            0,
            Some(pv_mv),
            None,
            Color::White,
            None,
            None,
        );

        assert_eq!(moves[0], pv_mv, "PV move should be at index 0");
    }

    #[test]
    fn tt_move_ordered_before_pv() {
        let mut pos = Position::startpos();
        let mut moves = chess_movegen::generate_legal_moves(&mut pos);
        assert!(moves.len() >= 2, "need at least two moves");

        let quiet_moves: Vec<Move> = moves.iter().filter(|m| !m.is_capture()).copied().collect();
        assert!(quiet_moves.len() >= 2, "need at least two quiet moves");

        let pv_mv = quiet_moves[0];
        let tt_mv = quiet_moves[1];

        let killers = KillerTable::new();
        let history = HistoryTable::new();
        let countermoves = CounterMoveTable::new();
        order_moves(
            &mut moves,
            &pos,
            &killers,
            &history,
            &countermoves,
            0,
            Some(pv_mv),
            Some(tt_mv),
            Color::White,
            None,
            None,
        );

        assert_eq!(moves[0], tt_mv, "TT move should be at index 0");
        assert_eq!(moves[1], pv_mv, "PV move should be at index 1");
    }

    #[test]
    fn counter_move_ordered_between_killers_and_quiet() {
        let mut pos = Position::from_fen("4k3/8/8/8/8/8/8/R3K3 w - - 0 1").expect("valid fen");
        let mut moves = chess_movegen::generate_legal_moves(&mut pos);

        let quiet_moves: Vec<Move> = moves.iter().filter(|m| !m.is_capture()).copied().collect();
        assert!(
            quiet_moves.len() >= 2,
            "need at least two quiet moves for this test"
        );

        let killer_mv = quiet_moves[0];
        let counter_mv = quiet_moves[1];

        let mut killers = KillerTable::new();
        killers.store(0, killer_mv);
        let history = HistoryTable::new();
        let mut countermoves = CounterMoveTable::new();
        let prev_mv = Move::new(
            chess_types::Square::D7,
            chess_types::Square::D2,
            chess_types::MoveFlag::QUIET,
        );
        countermoves.store(PieceKind::Queen, prev_mv.to_sq(), counter_mv);

        order_moves(
            &mut moves,
            &pos,
            &killers,
            &history,
            &countermoves,
            0,
            None,
            None,
            Color::White,
            Some(prev_mv),
            Some(PieceKind::Queen),
        );

        let killer_idx = moves
            .iter()
            .position(|m| *m == killer_mv)
            .expect("killer must be in list");
        let counter_idx = moves
            .iter()
            .position(|m| *m == counter_mv)
            .expect("counter-move must be in list");
        let first_plain_quiet_idx = moves
            .iter()
            .position(|m| !m.is_capture() && *m != killer_mv && *m != counter_mv);

        assert!(
            counter_idx > killer_idx,
            "counter-move should come after killer move"
        );
        if let Some(first_quiet) = first_plain_quiet_idx {
            assert!(
                counter_idx < first_quiet,
                "counter-move should come before plain quiet moves"
            );
        }
    }
}
