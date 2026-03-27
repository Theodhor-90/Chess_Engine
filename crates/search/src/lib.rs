pub mod killer;
pub mod ordering;

use std::time::{Duration, Instant};

use chess_board::Position;
use chess_types::{Color, Move, Piece, PieceKind, Square};

use killer::KillerTable;

pub const MATE_SCORE: i32 = 30000;
pub const INFINITY: i32 = 31000;

pub struct SearchContext {
    start: Instant,
    time_budget: Duration,
    nodes: u64,
    aborted: bool,
    killers: KillerTable,
}

impl SearchContext {
    fn check_time(&mut self) {
        if self.start.elapsed() >= self.time_budget {
            self.aborted = true;
        }
    }
}

fn king_square(pos: &Position, side: Color) -> Square {
    let king = Piece::new(side, PieceKind::King);
    let sq_idx = pos
        .piece_bitboard(king)
        .into_iter()
        .next()
        .expect("king must exist") as u8;
    Square::new(sq_idx).expect("valid square")
}

#[allow(clippy::only_used_in_recursion)]
pub fn quiescence(
    pos: &mut Position,
    mut alpha: i32,
    beta: i32,
    ply: u8,
    ctx: &mut SearchContext,
) -> i32 {
    ctx.nodes += 1;
    if ctx.nodes & 1023 == 0 {
        ctx.check_time();
    }
    if ctx.aborted {
        return 0;
    }

    let stand_pat = chess_eval::evaluate(pos);
    if stand_pat >= beta {
        return beta;
    }
    if stand_pat > alpha {
        alpha = stand_pat;
    }

    let moves = chess_movegen::generate_legal_moves(pos);
    let mut tactical: Vec<Move> = moves
        .into_iter()
        .filter(|mv| mv.is_capture() || mv.is_promotion())
        .collect();
    ordering::order_moves(&mut tactical, pos, &ctx.killers, ply);
    for mv in tactical {
        let undo = pos.make_move(mv);
        let score = -quiescence(pos, -beta, -alpha, ply + 1, ctx);
        pos.unmake_move(mv, undo);

        if ctx.aborted {
            return 0;
        }

        if score >= beta {
            return beta;
        }
        if score > alpha {
            alpha = score;
        }
    }

    alpha
}

pub fn negamax(
    pos: &mut Position,
    depth: u8,
    mut alpha: i32,
    beta: i32,
    ply: u8,
    ctx: &mut SearchContext,
) -> (i32, Option<Move>) {
    ctx.nodes += 1;
    if ctx.nodes & 1023 == 0 {
        ctx.check_time();
    }
    if ctx.aborted {
        return (0, None);
    }

    if depth == 0 {
        return (quiescence(pos, alpha, beta, ply, ctx), None);
    }

    let mut moves = chess_movegen::generate_legal_moves(pos);
    ordering::order_moves(&mut moves, pos, &ctx.killers, ply);

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
        let (score, _) = negamax(pos, depth - 1, -beta, -alpha, ply + 1, ctx);
        let score = -score;
        pos.unmake_move(mv, undo);

        if ctx.aborted {
            return (0, None);
        }

        if score > alpha {
            alpha = score;
            best_move = Some(mv);
            if alpha >= beta {
                if !mv.is_capture() {
                    ctx.killers.store(ply, mv);
                }
                break;
            }
        }
    }

    (alpha, best_move)
}

pub fn search(pos: &mut Position, time_budget: Duration) -> Option<Move> {
    let mut ctx = SearchContext {
        start: Instant::now(),
        time_budget,
        nodes: 0,
        aborted: false,
        killers: KillerTable::new(),
    };

    let mut best_move: Option<Move> = None;
    let mut depth: u8 = 1;

    loop {
        ctx.aborted = false;
        let (score, mv) = negamax(pos, depth, -INFINITY, INFINITY, 0, &mut ctx);

        if ctx.aborted {
            break;
        }

        if mv.is_some() {
            best_move = mv;
        }

        if score.abs() >= MATE_SCORE - 100 {
            break;
        }

        if depth == u8::MAX {
            break;
        }

        depth += 1;
    }

    best_move
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_board::Position;

    fn test_ctx() -> SearchContext {
        SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
        }
    }

    #[test]
    fn checkmate_black_is_mated() {
        let mut pos =
            Position::from_fen("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3")
                .expect("valid fen");
        let mut ctx = test_ctx();
        let (score, mv) = negamax(&mut pos, 1, -INFINITY, INFINITY, 0, &mut ctx);
        assert_eq!(score, -MATE_SCORE);
        assert!(mv.is_none());
    }

    #[test]
    fn checkmate_detected_at_depth() {
        let mut pos =
            Position::from_fen("rnb1kbnr/pppp1ppp/4p3/8/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 0 3")
                .expect("valid fen");
        let mut ctx = test_ctx();
        let (score, mv) = negamax(&mut pos, 1, -INFINITY, INFINITY, 0, &mut ctx);
        assert_eq!(score, -MATE_SCORE);
        assert!(mv.is_none());
    }

    #[test]
    fn stalemate_returns_zero() {
        let mut pos = Position::from_fen("k7/1R6/K7/8/8/8/8/8 b - - 0 1").expect("valid fen");
        let mut ctx = test_ctx();
        let (score, mv) = negamax(&mut pos, 1, -INFINITY, INFINITY, 0, &mut ctx);
        assert_eq!(score, 0);
        assert!(mv.is_none());
    }

    #[test]
    fn returns_legal_move_at_depth() {
        let mut pos = Position::startpos();
        let mut ctx = test_ctx();
        let (_, mv) = negamax(&mut pos, 1, -INFINITY, INFINITY, 0, &mut ctx);
        assert!(mv.is_some());
        let legal_moves = chess_movegen::generate_legal_moves(&mut pos);
        let best = mv.unwrap();
        assert!(legal_moves.iter().any(|&m| m == best));
    }

    #[test]
    fn prefers_capture_of_free_piece() {
        let mut pos = Position::from_fen("4k3/8/8/8/8/8/3q4/R3K3 w - - 0 1").expect("valid fen");
        let mut ctx = test_ctx();
        let (score, mv) = negamax(&mut pos, 1, -INFINITY, INFINITY, 0, &mut ctx);
        assert!(score > 0);
        assert!(mv.is_some());
        assert!(mv.unwrap().is_capture());
    }

    #[test]
    fn alpha_beta_prunes() {
        let mut pos = Position::startpos();
        let mut ctx = test_ctx();
        let (_, mv) = negamax(&mut pos, 3, -INFINITY, INFINITY, 0, &mut ctx);
        assert!(mv.is_some());
    }

    #[test]
    fn qsearch_stand_pat_cutoff() {
        let mut pos = Position::from_fen("4k3/8/8/8/8/8/8/Q3K3 w - - 0 1").expect("valid fen");
        let static_eval = chess_eval::evaluate(&pos);
        let mut ctx = test_ctx();
        let score = quiescence(&mut pos, -INFINITY, INFINITY, 0, &mut ctx);
        assert_eq!(score, static_eval);
    }

    #[test]
    fn qsearch_finds_hanging_piece() {
        let mut pos = Position::from_fen("4k3/8/8/8/8/8/3q4/R3K3 w - - 0 1").expect("valid fen");
        let mut ctx = test_ctx();
        let score = quiescence(&mut pos, -INFINITY, INFINITY, 0, &mut ctx);
        assert!(score > 0);
    }

    #[test]
    fn negamax_uses_quiescence_for_tactics() {
        let mut pos = Position::from_fen("4k3/8/8/R2b4/8/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut ctx = test_ctx();
        let (score, mv) = negamax(&mut pos, 1, -INFINITY, INFINITY, 0, &mut ctx);
        assert!(score > 0);
        assert!(mv.is_some());
        assert!(mv.unwrap().is_capture());
    }

    #[test]
    fn qsearch_beta_cutoff_in_captures() {
        let mut pos = Position::from_fen("4k3/8/8/8/8/8/3q4/R3K3 w - - 0 1").expect("valid fen");
        let beta = 50;
        let mut ctx = test_ctx();
        let score = quiescence(&mut pos, -INFINITY, beta, 0, &mut ctx);
        assert_eq!(score, beta);
    }

    #[test]
    fn qsearch_only_searches_captures_and_promotions() {
        let mut pos = Position::from_fen("4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1").expect("valid fen");
        let static_eval = chess_eval::evaluate(&pos);
        let mut ctx = test_ctx();
        let score = quiescence(&mut pos, -INFINITY, INFINITY, 0, &mut ctx);
        assert!(score >= static_eval);
    }

    #[test]
    fn iterative_deepening_returns_legal_move() {
        let mut pos = Position::startpos();
        let mv = search(&mut pos, Duration::from_secs(5));
        assert!(mv.is_some());
        let legal_moves = chess_movegen::generate_legal_moves(&mut pos);
        assert!(legal_moves.iter().any(|&m| m == mv.unwrap()));
    }

    #[test]
    fn iterative_deepening_finds_better_moves_at_depth() {
        let mut pos = Position::from_fen(
            "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
        )
        .expect("valid fen");
        let mv = search(&mut pos, Duration::from_secs(5));
        assert!(mv.is_some());
        let best = mv.unwrap();
        assert_eq!(best.to_sq().index(), Square::new(53).unwrap().index());
    }

    #[test]
    fn search_respects_time_budget() {
        let mut pos = Position::startpos();
        let start = Instant::now();
        let mv = search(&mut pos, Duration::from_millis(50));
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(200));
        assert!(mv.is_some());
    }

    #[test]
    fn search_returns_none_for_checkmate() {
        let mut pos =
            Position::from_fen("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3")
                .expect("valid fen");
        let mv = search(&mut pos, Duration::from_secs(5));
        assert!(mv.is_none());
    }

    #[test]
    fn search_returns_none_for_stalemate() {
        let mut pos = Position::from_fen("k7/1R6/K7/8/8/8/8/8 b - - 0 1").expect("valid fen");
        let mv = search(&mut pos, Duration::from_secs(5));
        assert!(mv.is_none());
    }

    #[test]
    fn node_counter_increments() {
        let mut pos = Position::startpos();
        let mut ctx = test_ctx();
        negamax(&mut pos, 2, -INFINITY, INFINITY, 0, &mut ctx);
        assert!(ctx.nodes > 0);
    }
}
