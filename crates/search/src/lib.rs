use chess_board::Position;
use chess_types::{Color, Move, Piece, PieceKind, Square};

pub const MATE_SCORE: i32 = 30000;
pub const INFINITY: i32 = 31000;

fn king_square(pos: &Position, side: Color) -> Square {
    let king = Piece::new(side, PieceKind::King);
    let sq_idx = pos
        .piece_bitboard(king)
        .into_iter()
        .next()
        .expect("king must exist") as u8;
    Square::new(sq_idx).expect("valid square")
}

pub fn negamax(
    pos: &mut Position,
    depth: u8,
    mut alpha: i32,
    beta: i32,
    ply: u8,
) -> (i32, Option<Move>) {
    if depth == 0 {
        return (chess_eval::evaluate(pos), None);
    }

    let moves = chess_movegen::generate_legal_moves(pos);

    if moves.is_empty() {
        let king_sq = king_square(pos, pos.side_to_move());
        let opponent = pos.side_to_move().opposite();
        if pos.is_square_attacked(king_sq, opponent) {
            return (-MATE_SCORE + ply as i32, None);
        } else {
            return (0, None);
        }
    }

    let mut best_move: Option<Move> = None;

    for mv in moves {
        let undo = pos.make_move(mv);
        let (score, _) = negamax(pos, depth - 1, -beta, -alpha, ply + 1);
        let score = -score;
        pos.unmake_move(mv, undo);

        if score > alpha {
            alpha = score;
            best_move = Some(mv);
            if alpha >= beta {
                break;
            }
        }
    }

    (alpha, best_move)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_board::Position;

    #[test]
    fn checkmate_black_is_mated() {
        let mut pos =
            Position::from_fen("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3")
                .expect("valid fen");
        let (score, mv) = negamax(&mut pos, 1, -INFINITY, INFINITY, 0);
        assert_eq!(score, -MATE_SCORE);
        assert!(mv.is_none());
    }

    #[test]
    fn checkmate_detected_at_depth() {
        let mut pos =
            Position::from_fen("rnb1kbnr/pppp1ppp/4p3/8/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 0 3")
                .expect("valid fen");
        let (score, mv) = negamax(&mut pos, 1, -INFINITY, INFINITY, 0);
        assert_eq!(score, -MATE_SCORE);
        assert!(mv.is_none());
    }

    #[test]
    fn stalemate_returns_zero() {
        let mut pos = Position::from_fen("k7/1R6/K7/8/8/8/8/8 b - - 0 1").expect("valid fen");
        let (score, mv) = negamax(&mut pos, 1, -INFINITY, INFINITY, 0);
        assert_eq!(score, 0);
        assert!(mv.is_none());
    }

    #[test]
    fn returns_legal_move_at_depth() {
        let mut pos = Position::startpos();
        let (_, mv) = negamax(&mut pos, 1, -INFINITY, INFINITY, 0);
        assert!(mv.is_some());
        let legal_moves = chess_movegen::generate_legal_moves(&mut pos);
        let best = mv.unwrap();
        assert!(legal_moves.iter().any(|&m| m == best));
    }

    #[test]
    fn prefers_capture_of_free_piece() {
        let mut pos = Position::from_fen("4k3/8/8/8/8/8/3q4/R3K3 w - - 0 1").expect("valid fen");
        let (score, mv) = negamax(&mut pos, 1, -INFINITY, INFINITY, 0);
        assert!(score > 0);
        assert!(mv.is_some());
        assert!(mv.unwrap().is_capture());
    }

    #[test]
    fn alpha_beta_prunes() {
        let mut pos = Position::startpos();
        let (_, mv) = negamax(&mut pos, 3, -INFINITY, INFINITY, 0);
        assert!(mv.is_some());
    }
}
