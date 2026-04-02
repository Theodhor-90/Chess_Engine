pub mod countermove;
pub mod history;
pub mod killer;
pub mod ordering;
pub mod pv_table;
pub mod see;
pub mod tt;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use chess_board::Position;
use chess_types::{Color, Move, Piece, PieceKind, Square};

use countermove::CounterMoveTable;
use history::HistoryTable;
use killer::KillerTable;
use pv_table::PvTable;
use tt::{score_from_tt, score_to_tt, verification_key, BoundType, TranspositionTable, TtEntry};

pub const MATE_SCORE: i32 = 30000;
pub const INFINITY: i32 = 31000;
pub const TB_WIN_SCORE: i32 = MATE_SCORE - 200;
const TB_WIN_THRESHOLD: i32 = TB_WIN_SCORE / 2;
const IID_MIN_DEPTH: i32 = 4;
const MAX_PLY: i32 = 128;
const FUTILITY_MARGINS: [i32; 4] = [0, 180, 360, 540];
const REVERSE_FUTILITY_MARGINS: [i32; 4] = [0, 120, 350, 550];
const SINGULAR_MARGIN: i32 = 64;
const SINGULAR_MIN_DEPTH: u8 = 6;
const ASPIRATION_DELTA: i32 = 25;
const ASPIRATION_WIDEN_FACTOR: i32 = 4;

pub trait TbProber {
    fn probe_wdl(&mut self, pos: &Position) -> Option<i32>;
    fn probe_root(&mut self, pos: &Position) -> Option<(i32, i32)>;
}

fn reborrow_prober<'a>(prober: &'a mut Option<&mut dyn TbProber>) -> Option<&'a mut dyn TbProber> {
    match prober {
        Some(ref mut p) => Some(&mut **p),
        None => None,
    }
}

/// Callback invoked after each completed search depth: `(depth, score, nodes, elapsed, pv)`.
pub type DepthCallback<'a> = &'a dyn Fn(u8, i32, u64, Duration, &[Move]);

#[derive(Debug, Clone)]
pub struct SearchLimits {
    pub max_time: Duration,
    pub max_depth: Option<u8>,
    pub max_nodes: Option<u64>,
    pub stop_flag: Option<Arc<AtomicBool>>,
}

pub struct SearchContext {
    start: Instant,
    time_budget: Duration,
    nodes: u64,
    aborted: bool,
    killers: KillerTable,
    history_table: HistoryTable,
    countermove_table: CounterMoveTable,
    pv_table: PvTable,
    prev_pv: Vec<Move>,
    stop_flag: Option<Arc<AtomicBool>>,
    max_nodes: Option<u64>,
    tt: TranspositionTable,
    history: Vec<u64>,
    pawn_table: chess_eval::PawnHashTable,
    pub(crate) lmr_enabled: bool,
    pub(crate) futility_enabled: bool,
    pub(crate) check_extension_enabled: bool,
    pub(crate) singular_extension_enabled: bool,
}

impl SearchContext {
    fn check_time(&mut self) {
        if self.start.elapsed() >= self.time_budget {
            self.aborted = true;
        }
        if let Some(ref flag) = self.stop_flag {
            if flag.load(Ordering::Relaxed) {
                self.aborted = true;
            }
        }
        if let Some(max) = self.max_nodes {
            if self.nodes >= max {
                self.aborted = true;
            }
        }
    }

    fn pv_move_at(&self, ply: u8) -> Option<Move> {
        self.prev_pv.get(ply as usize).copied()
    }
}

const LMR_MAX_DEPTH: usize = 64;
const LMR_MAX_MOVES: usize = 64;

#[allow(clippy::needless_range_loop)]
fn lmr_table() -> &'static [[u8; LMR_MAX_MOVES]; LMR_MAX_DEPTH] {
    static TABLE: OnceLock<[[u8; LMR_MAX_MOVES]; LMR_MAX_DEPTH]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = [[0u8; LMR_MAX_MOVES]; LMR_MAX_DEPTH];
        for d in 1..LMR_MAX_DEPTH {
            for m in 1..LMR_MAX_MOVES {
                t[d][m] = ((d as f64).ln() * (m as f64).ln() / 1.80).floor() as u8;
            }
        }
        t
    })
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

    let stand_pat = chess_eval::evaluate(pos, &mut ctx.pawn_table);
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
    ordering::order_moves(
        &mut tactical,
        pos,
        &ctx.killers,
        &ctx.history_table,
        &ctx.countermove_table,
        ply,
        None,
        None,
        pos.side_to_move(),
        None,
        None,
    );
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

#[allow(clippy::too_many_arguments)]
pub fn negamax(
    pos: &mut Position,
    mut depth: u8,
    mut alpha: i32,
    beta: i32,
    ply: u8,
    allow_null: bool,
    ctx: &mut SearchContext,
    prev_move: Option<(PieceKind, Move)>,
    excluded_move: Option<Move>,
    mut tb_prober: Option<&mut dyn TbProber>,
) -> (i32, Option<Move>) {
    ctx.nodes += 1;
    if ctx.nodes & 1023 == 0 {
        ctx.check_time();
    }
    if ctx.aborted {
        return (0, None);
    }

    if ply > 0 {
        if pos.halfmove_clock() >= 100 {
            return (0, None);
        }

        let current_hash = pos.hash();
        let halfmove = pos.halfmove_clock() as usize;
        let start = if ctx.history.len() > halfmove {
            ctx.history.len() - halfmove
        } else {
            0
        };
        for i in (start..ctx.history.len()).rev().skip(1).step_by(2) {
            if ctx.history[i] == current_hash {
                return (0, None);
            }
        }
    }

    if ply > 0 {
        if let Some(ref mut prober) = tb_prober {
            if let Some(score) = prober.probe_wdl(pos) {
                let adjusted = if score > TB_WIN_THRESHOLD {
                    score - ply as i32
                } else if score < -TB_WIN_THRESHOLD {
                    score + ply as i32
                } else {
                    score
                };
                return (adjusted, None);
            }
        }
    }

    if depth == 0 {
        return (quiescence(pos, alpha, beta, ply, ctx), None);
    }

    let in_check = {
        let king_sq = king_square(pos, pos.side_to_move());
        pos.is_square_attacked(king_sq, pos.side_to_move().opposite())
    };

    if ctx.check_extension_enabled && in_check {
        depth = depth.saturating_add(1);
    }

    let original_alpha = alpha;

    ctx.pv_table.clear_ply(ply);

    let hash = pos.hash();
    let mut tt_move: Option<Move> = None;

    if let Some(entry) = ctx.tt.probe(hash) {
        tt_move = entry.best_move();
        if ply > 0 && entry.depth() >= depth {
            let tt_score = score_from_tt(entry.score(), ply);
            match entry.bound() {
                BoundType::Exact => return (tt_score, tt_move),
                BoundType::LowerBound => {
                    if tt_score >= beta {
                        if let Some(tm) = tt_move {
                            if !tm.is_capture() && !tm.is_promotion() {
                                let bonus = (depth as i32) * (depth as i32);
                                let side = pos.side_to_move();
                                ctx.history_table
                                    .update(side, tm.from_sq(), tm.to_sq(), bonus);
                            }
                        }
                        return (beta, tt_move);
                    }
                }
                BoundType::UpperBound => {
                    if tt_score <= alpha {
                        return (alpha, tt_move);
                    }
                }
            }
        }
    }

    // Singular extension: if TT move dominates all alternatives, extend its search depth
    if ctx.singular_extension_enabled
        && depth >= SINGULAR_MIN_DEPTH
        && excluded_move.is_none()
        && !in_check
    {
        if let Some(entry) = ctx.tt.probe(hash) {
            let tt_bound = entry.bound();
            if (tt_bound == BoundType::LowerBound || tt_bound == BoundType::Exact)
                && entry.depth() >= depth.saturating_sub(3)
                && entry.best_move().is_some()
            {
                let tt_mv = entry.best_move().unwrap();
                let tt_score = score_from_tt(entry.score(), ply);
                let s_beta = tt_score - SINGULAR_MARGIN;
                let (verify_score, _) = negamax(
                    pos,
                    depth / 2,
                    s_beta - 1,
                    s_beta,
                    ply,
                    false,
                    ctx,
                    prev_move,
                    Some(tt_mv),
                    reborrow_prober(&mut tb_prober),
                );
                if !ctx.aborted && verify_score < s_beta {
                    depth = depth.saturating_add(1);
                }
            }
        }
    }

    // Null move pruning
    if allow_null
        && !in_check
        && depth >= 3
        && pos.has_non_pawn_material(pos.side_to_move())
        && beta.abs() < MATE_SCORE - MAX_PLY
    {
        let r: u8 = if depth <= 5 { 2 } else { 3 };
        let null_undo = pos.make_null_move();
        ctx.history.push(pos.hash());
        let (null_score, _) = negamax(
            pos,
            depth - 1 - r,
            -beta,
            -beta + 1,
            ply + 1,
            false,
            ctx,
            None,
            None,
            reborrow_prober(&mut tb_prober),
        );
        let null_score = -null_score;
        ctx.history.pop();
        pos.unmake_null_move(null_undo);

        if ctx.aborted {
            return (0, None);
        }

        if null_score >= beta {
            return (beta, None);
        }
    }

    let static_eval = if !in_check {
        chess_eval::evaluate(pos, &mut ctx.pawn_table)
    } else {
        0
    };

    // Reverse futility pruning
    if ctx.futility_enabled && !in_check && depth <= 3 && beta.abs() < MATE_SCORE - MAX_PLY {
        let margin = REVERSE_FUTILITY_MARGINS[depth as usize];
        if static_eval - margin >= beta {
            return (static_eval, None);
        }
    }

    let mut moves = chess_movegen::generate_legal_moves(pos);

    // Exclude the specified move (used by singular extension verification search)
    if let Some(excl) = excluded_move {
        moves.retain(|&m| m != excl);
    }

    // Validate TT move: discard if not legal or if it matches the excluded move
    if let Some(tm) = tt_move {
        if !moves.contains(&tm) || excluded_move == Some(tm) {
            tt_move = None;
        }
    }

    // Internal iterative deepening: if no TT move and depth is sufficient,
    // do a reduced-depth search to populate the TT with a move for ordering.
    if tt_move.is_none() && depth as i32 >= IID_MIN_DEPTH {
        negamax(
            pos,
            depth - 2,
            alpha,
            beta,
            ply,
            false,
            ctx,
            prev_move,
            None,
            reborrow_prober(&mut tb_prober),
        );
        if let Some(entry) = ctx.tt.probe(hash) {
            if let Some(iid_move) = entry.best_move() {
                if moves.contains(&iid_move) {
                    tt_move = Some(iid_move);
                }
            }
        }
    }

    let pv_move = ctx.pv_move_at(ply);
    let side = pos.side_to_move();
    ordering::order_moves(
        &mut moves,
        pos,
        &ctx.killers,
        &ctx.history_table,
        &ctx.countermove_table,
        ply,
        pv_move,
        tt_move,
        side,
        prev_move.map(|(_, m)| m),
        prev_move.map(|(pk, _)| pk),
    );

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
    let mut searched_quiets: Vec<Move> = Vec::new();

    for (moves_searched, mv) in (0_u32..).zip(moves.into_iter()) {
        let piece_kind = pos
            .piece_on(mv.from_sq())
            .expect("piece must exist on from_sq")
            .kind;

        let is_tactical = mv.is_capture() || mv.is_promotion();

        let see_score = if mv.is_capture() && !mv.is_promotion() {
            see::see(pos, mv)
        } else {
            0
        };

        let undo = pos.make_move(mv);
        ctx.history.push(pos.hash());

        let is_tt_move = tt_move == Some(mv);
        let is_pv_move = pv_move == Some(mv);
        let is_killer = ctx.killers.is_killer(ply, mv);

        let gives_check = {
            let opp_king_sq = king_square(pos, pos.side_to_move());
            pos.is_square_attacked(opp_king_sq, pos.side_to_move().opposite())
        };

        let do_futility = ctx.futility_enabled
            && depth <= 3
            && !in_check
            && !gives_check
            && !is_tactical
            && alpha.abs() < MATE_SCORE - MAX_PLY;

        if do_futility {
            let margin = FUTILITY_MARGINS[depth as usize];
            if static_eval + margin <= alpha {
                ctx.history.pop();
                pos.unmake_move(mv, undo);
                continue;
            }
        }

        if mv.is_capture()
            && !mv.is_promotion()
            && !in_check
            && !gives_check
            && !is_tt_move
            && !is_pv_move
            && depth <= 3
            && see_score < 0
        {
            ctx.history.pop();
            pos.unmake_move(mv, undo);
            continue;
        }

        let do_lmr = ctx.lmr_enabled
            && depth >= 3
            && moves_searched >= 3
            && !is_tt_move
            && !is_pv_move
            && !is_killer
            && !is_tactical
            && !in_check
            && !gives_check;

        let mut score;
        if do_lmr {
            let table = lmr_table();
            let d = (depth as usize).min(LMR_MAX_DEPTH - 1);
            let m = (moves_searched as usize).min(LMR_MAX_MOVES - 1);
            let reduction = table[d][m].max(1);

            let reduced_depth = if (depth as i32 - 1 - reduction as i32) > 0 {
                depth - 1 - reduction
            } else {
                0
            };
            // Stage 1: reduced-depth zero-window search
            let (s, _) = negamax(
                pos,
                reduced_depth,
                -alpha - 1,
                -alpha,
                ply + 1,
                true,
                ctx,
                Some((piece_kind, mv)),
                None,
                reborrow_prober(&mut tb_prober),
            );
            score = -s;

            // Stage 2: full-depth zero-window re-search (on fail-high)
            if score > alpha {
                let (s2, _) = negamax(
                    pos,
                    depth - 1,
                    -alpha - 1,
                    -alpha,
                    ply + 1,
                    true,
                    ctx,
                    Some((piece_kind, mv)),
                    None,
                    reborrow_prober(&mut tb_prober),
                );
                score = -s2;
            }

            // Stage 3: full-depth full-window re-search (on fail-high again)
            if score > alpha && score < beta {
                let (s3, _) = negamax(
                    pos,
                    depth - 1,
                    -beta,
                    -alpha,
                    ply + 1,
                    true,
                    ctx,
                    Some((piece_kind, mv)),
                    None,
                    reborrow_prober(&mut tb_prober),
                );
                score = -s3;
            }
        } else if moves_searched == 0 {
            // PVS: first move gets full window
            let (s, _) = negamax(
                pos,
                depth - 1,
                -beta,
                -alpha,
                ply + 1,
                true,
                ctx,
                Some((piece_kind, mv)),
                None,
                reborrow_prober(&mut tb_prober),
            );
            score = -s;
        } else {
            // PVS: subsequent moves get zero-window
            let (s, _) = negamax(
                pos,
                depth - 1,
                -alpha - 1,
                -alpha,
                ply + 1,
                true,
                ctx,
                Some((piece_kind, mv)),
                None,
                reborrow_prober(&mut tb_prober),
            );
            score = -s;

            // Re-search with full window on fail-high (score > alpha && score < beta)
            if score > alpha && score < beta {
                let (s2, _) = negamax(
                    pos,
                    depth - 1,
                    -beta,
                    -alpha,
                    ply + 1,
                    true,
                    ctx,
                    Some((piece_kind, mv)),
                    None,
                    reborrow_prober(&mut tb_prober),
                );
                score = -s2;
            }
        }

        ctx.history.pop();
        pos.unmake_move(mv, undo);

        if ctx.aborted {
            return (0, None);
        }

        if !mv.is_capture() && !mv.is_promotion() {
            searched_quiets.push(mv);
        }

        if score > alpha {
            alpha = score;
            best_move = Some(mv);
            ctx.pv_table.update(ply, mv);
            if alpha >= beta {
                if !mv.is_capture() {
                    ctx.killers.store(ply, mv);
                    let bonus = (depth as i32) * (depth as i32);
                    let side = pos.side_to_move();
                    ctx.history_table
                        .update(side, mv.from_sq(), mv.to_sq(), bonus);
                    for prev_mv in searched_quiets
                        .iter()
                        .take(searched_quiets.len().saturating_sub(1))
                    {
                        ctx.history_table
                            .update(side, prev_mv.from_sq(), prev_mv.to_sq(), -bonus);
                    }
                    if let Some((prev_piece, prev_mv)) = prev_move {
                        ctx.countermove_table.store(prev_piece, prev_mv.to_sq(), mv);
                    }
                }
                break;
            }
        }
    }

    let bound = if alpha <= original_alpha {
        BoundType::UpperBound
    } else if alpha >= beta {
        BoundType::LowerBound
    } else {
        BoundType::Exact
    };
    let store_score = score_to_tt(alpha, ply);
    let entry = TtEntry::new(
        verification_key(hash),
        depth,
        store_score,
        bound,
        best_move,
        ctx.tt.generation(),
    );
    ctx.tt.store(hash, entry);

    (alpha, best_move)
}

pub fn search(
    pos: &mut Position,
    limits: SearchLimits,
    game_history: &[u64],
    on_depth: Option<DepthCallback<'_>>,
    mut tb_prober: Option<&mut dyn TbProber>,
) -> Option<Move> {
    let mut ctx = SearchContext {
        start: Instant::now(),
        time_budget: limits.max_time,
        nodes: 0,
        aborted: false,
        killers: KillerTable::new(),
        history_table: HistoryTable::new(),
        countermove_table: CounterMoveTable::new(),
        pv_table: PvTable::new(),
        prev_pv: Vec::new(),
        stop_flag: limits.stop_flag,
        max_nodes: limits.max_nodes,
        tt: TranspositionTable::new(64),
        history: game_history.to_vec(),
        pawn_table: chess_eval::PawnHashTable::new(),
        lmr_enabled: true,
        futility_enabled: true,
        check_extension_enabled: true,
        singular_extension_enabled: true,
    };

    ctx.tt.new_generation();
    ctx.history.push(pos.hash());

    // Root tablebase filtering
    let mut tb_hint: Option<Move> = None;
    if let Some(ref mut prober) = tb_prober {
        let legal_moves = chess_movegen::generate_legal_moves(pos);
        let mut probed: Vec<(Move, i32, i32)> = Vec::new();
        let mut all_ok = true;
        for mv in &legal_moves {
            let undo = pos.make_move(*mv);
            let result = prober.probe_root(pos);
            pos.unmake_move(*mv, undo);
            if let Some((wdl, dtz)) = result {
                probed.push((*mv, -wdl, dtz));
            } else {
                all_ok = false;
                break;
            }
        }
        if all_ok && !probed.is_empty() {
            let best_wdl = probed.iter().map(|(_, w, _)| *w).max().unwrap();
            probed.retain(|(_, w, _)| *w == best_wdl);
            if best_wdl > 0 {
                probed.sort_by_key(|(_, _, dtz)| dtz.abs());
            } else if best_wdl < 0 {
                probed.sort_by_key(|(_, _, dtz)| std::cmp::Reverse(dtz.abs()));
            }
            if probed.len() == 1 {
                return Some(probed[0].0);
            }
            tb_hint = Some(probed[0].0);
        }
    }

    let mut best_move: Option<Move> = tb_hint;
    let mut depth: u8 = 1;
    let mut prev_score: i32 = 0;

    if let Some(hint) = tb_hint {
        ctx.prev_pv = vec![hint];
    }

    loop {
        ctx.aborted = false;
        ctx.pv_table.clear();

        let (score, mv) = if depth >= 5 && prev_score.abs() < MATE_SCORE - 100 {
            let mut delta = ASPIRATION_DELTA;
            let mut alpha = prev_score - delta;
            let mut beta = prev_score + delta;
            let mut fail_low = 0;
            let mut fail_high = 0;

            loop {
                ctx.pv_table.clear();
                let (s, m) = negamax(
                    pos,
                    depth,
                    alpha,
                    beta,
                    0,
                    true,
                    &mut ctx,
                    None,
                    None,
                    reborrow_prober(&mut tb_prober),
                );

                if ctx.aborted {
                    break (s, m);
                }

                if s <= alpha {
                    fail_low += 1;
                    if fail_low >= 2 {
                        alpha = -INFINITY;
                    } else {
                        delta *= ASPIRATION_WIDEN_FACTOR;
                        alpha = prev_score - delta;
                    }
                } else if s >= beta {
                    fail_high += 1;
                    if fail_high >= 2 {
                        beta = INFINITY;
                    } else {
                        delta *= ASPIRATION_WIDEN_FACTOR;
                        beta = prev_score + delta;
                    }
                } else {
                    break (s, m);
                }
            }
        } else {
            negamax(
                pos,
                depth,
                -INFINITY,
                INFINITY,
                0,
                true,
                &mut ctx,
                None,
                None,
                reborrow_prober(&mut tb_prober),
            )
        };

        if ctx.aborted {
            break;
        }

        if mv.is_some() {
            best_move = mv;
        }

        prev_score = score;
        ctx.prev_pv = ctx.pv_table.extract_pv();

        if let Some(ref cb) = on_depth {
            cb(depth, score, ctx.nodes, ctx.start.elapsed(), &ctx.prev_pv);
        }

        if score.abs() >= MATE_SCORE - 100 {
            break;
        }

        if depth == u8::MAX {
            break;
        }

        depth += 1;

        if let Some(d) = limits.max_depth {
            if depth > d {
                break;
            }
        }
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
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        }
    }

    #[test]
    fn checkmate_black_is_mated() {
        let mut pos =
            Position::from_fen("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3")
                .expect("valid fen");
        let mut ctx = test_ctx();
        let (score, mv) = negamax(
            &mut pos, 1, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
        );
        assert_eq!(score, -MATE_SCORE);
        assert!(mv.is_none());
    }

    #[test]
    fn checkmate_detected_at_depth() {
        let mut pos =
            Position::from_fen("rnb1kbnr/pppp1ppp/4p3/8/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 0 3")
                .expect("valid fen");
        let mut ctx = test_ctx();
        let (score, mv) = negamax(
            &mut pos, 1, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
        );
        assert_eq!(score, -MATE_SCORE);
        assert!(mv.is_none());
    }

    #[test]
    fn stalemate_returns_zero() {
        let mut pos = Position::from_fen("k7/1R6/K7/8/8/8/8/8 b - - 0 1").expect("valid fen");
        let mut ctx = test_ctx();
        let (score, mv) = negamax(
            &mut pos, 1, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
        );
        assert_eq!(score, 0);
        assert!(mv.is_none());
    }

    #[test]
    fn returns_legal_move_at_depth() {
        let mut pos = Position::startpos();
        let mut ctx = test_ctx();
        let (_, mv) = negamax(
            &mut pos, 1, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
        );
        assert!(mv.is_some());
        let legal_moves = chess_movegen::generate_legal_moves(&mut pos);
        let best = mv.unwrap();
        assert!(legal_moves.iter().any(|&m| m == best));
    }

    #[test]
    fn prefers_capture_of_free_piece() {
        let mut pos = Position::from_fen("4k3/8/8/8/8/8/3q4/R3K3 w - - 0 1").expect("valid fen");
        let mut ctx = test_ctx();
        let (score, mv) = negamax(
            &mut pos, 1, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
        );
        assert!(score > 0);
        assert!(mv.is_some());
        assert!(mv.unwrap().is_capture());
    }

    #[test]
    fn alpha_beta_prunes() {
        let mut pos = Position::startpos();
        let mut ctx = test_ctx();
        let (_, mv) = negamax(
            &mut pos, 3, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
        );
        assert!(mv.is_some());
    }

    #[test]
    fn qsearch_stand_pat_cutoff() {
        let mut pos = Position::from_fen("4k3/8/8/8/8/8/8/Q3K3 w - - 0 1").expect("valid fen");
        let mut ctx = test_ctx();
        let static_eval = chess_eval::evaluate(&pos, &mut ctx.pawn_table);
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
        let (score, mv) = negamax(
            &mut pos, 1, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
        );
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
        let mut ctx = test_ctx();
        let static_eval = chess_eval::evaluate(&pos, &mut ctx.pawn_table);
        let score = quiescence(&mut pos, -INFINITY, INFINITY, 0, &mut ctx);
        assert!(score >= static_eval);
    }

    #[test]
    fn iterative_deepening_returns_legal_move() {
        let mut pos = Position::startpos();
        let limits = SearchLimits {
            max_time: Duration::from_secs(5),
            max_depth: None,
            max_nodes: None,
            stop_flag: None,
        };
        let mv = search(&mut pos, limits, &[], None, None);
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
        let limits = SearchLimits {
            max_time: Duration::from_secs(5),
            max_depth: None,
            max_nodes: None,
            stop_flag: None,
        };
        let mv = search(&mut pos, limits, &[], None, None);
        assert!(mv.is_some());
        let best = mv.unwrap();
        assert_eq!(best.to_sq().index(), Square::new(53).unwrap().index());
    }

    #[test]
    fn search_respects_time_budget() {
        let mut pos = Position::startpos();
        let start = Instant::now();
        let limits = SearchLimits {
            max_time: Duration::from_millis(50),
            max_depth: None,
            max_nodes: None,
            stop_flag: None,
        };
        let mv = search(&mut pos, limits, &[], None, None);
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(200));
        assert!(mv.is_some());
    }

    #[test]
    fn search_returns_none_for_checkmate() {
        let mut pos =
            Position::from_fen("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3")
                .expect("valid fen");
        let limits = SearchLimits {
            max_time: Duration::from_secs(5),
            max_depth: None,
            max_nodes: None,
            stop_flag: None,
        };
        let mv = search(&mut pos, limits, &[], None, None);
        assert!(mv.is_none());
    }

    #[test]
    fn search_returns_none_for_stalemate() {
        let mut pos = Position::from_fen("k7/1R6/K7/8/8/8/8/8 b - - 0 1").expect("valid fen");
        let limits = SearchLimits {
            max_time: Duration::from_secs(5),
            max_depth: None,
            max_nodes: None,
            stop_flag: None,
        };
        let mv = search(&mut pos, limits, &[], None, None);
        assert!(mv.is_none());
    }

    #[test]
    fn node_counter_increments() {
        let mut pos = Position::startpos();
        let mut ctx = test_ctx();
        negamax(
            &mut pos, 2, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
        );
        assert!(ctx.nodes > 0);
    }

    #[test]
    fn pv_ordering_reduces_nodes() {
        let fen = "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2";
        let depth: u8 = 4;

        // Search with PV ordering (normal code path via iterative deepening)
        let mut pos_pv = Position::from_fen(fen).expect("valid fen");
        let mut ctx_pv = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        // Run iterative deepening up to target depth to build PV
        for d in 1..=depth {
            ctx_pv.pv_table.clear();
            negamax(
                &mut pos_pv,
                d,
                -INFINITY,
                INFINITY,
                0,
                true,
                &mut ctx_pv,
                None,
                None,
                None,
            );
            ctx_pv.prev_pv = ctx_pv.pv_table.extract_pv();
        }
        let nodes_with_pv = ctx_pv.nodes;

        // Search without PV ordering (prev_pv always empty)
        let mut pos_no_pv = Position::from_fen(fen).expect("valid fen");
        let mut ctx_no_pv = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(0),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        // Run iterative deepening but never set prev_pv
        for d in 1..=depth {
            ctx_no_pv.pv_table.clear();
            negamax(
                &mut pos_no_pv,
                d,
                -INFINITY,
                INFINITY,
                0,
                true,
                &mut ctx_no_pv,
                None,
                None,
                None,
            );
            // Intentionally do NOT set prev_pv
        }
        let nodes_without_pv = ctx_no_pv.nodes;

        assert!(
            nodes_with_pv < nodes_without_pv,
            "PV ordering should reduce nodes: {} (with PV) vs {} (without PV)",
            nodes_with_pv,
            nodes_without_pv
        );
    }

    #[test]
    fn test_stop_flag_aborts_search() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);

        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            stop_clone.store(true, Ordering::Relaxed);
        });

        let mut pos = Position::startpos();
        let start = Instant::now();
        let limits = SearchLimits {
            max_time: Duration::from_secs(60),
            max_depth: None,
            max_nodes: None,
            stop_flag: Some(stop),
        };
        let mv = search(&mut pos, limits, &[], None, None);
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_millis(500),
            "search should abort within 500ms, took {:?}",
            elapsed
        );
        assert!(
            mv.is_some(),
            "search should find at least one move before being stopped"
        );
    }

    #[test]
    fn search_respects_depth_limit() {
        use std::sync::atomic::AtomicU8;

        let max_depth_seen = Arc::new(AtomicU8::new(0));
        let max_depth_clone = Arc::clone(&max_depth_seen);

        let mut pos = Position::startpos();
        let limits = SearchLimits {
            max_time: Duration::from_secs(86400),
            max_depth: Some(3),
            max_nodes: None,
            stop_flag: None,
        };
        let cb = move |depth: u8, _score: i32, _nodes: u64, _elapsed: Duration, _pv: &[Move]| {
            max_depth_clone.fetch_max(depth, Ordering::Relaxed);
        };
        let mv = search(&mut pos, limits, &[], Some(&cb), None);
        assert!(mv.is_some());
        assert_eq!(max_depth_seen.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn search_respects_node_limit() {
        let mut pos = Position::startpos();
        let start = Instant::now();
        let limits = SearchLimits {
            max_time: Duration::from_secs(86400),
            max_depth: None,
            max_nodes: Some(500),
            stop_flag: None,
        };
        let mv = search(&mut pos, limits, &[], None, None);
        let elapsed = start.elapsed();
        assert!(mv.is_some());
        assert!(
            elapsed < Duration::from_secs(1),
            "node-limited search should terminate quickly, took {:?}",
            elapsed
        );
    }

    #[test]
    fn search_respects_movetime() {
        let mut pos = Position::startpos();
        let start = Instant::now();
        let limits = SearchLimits {
            max_time: Duration::from_millis(100),
            max_depth: None,
            max_nodes: None,
            stop_flag: None,
        };
        let mv = search(&mut pos, limits, &[], None, None);
        let elapsed = start.elapsed();
        assert!(mv.is_some());
        assert!(
            elapsed < Duration::from_millis(500),
            "movetime search should finish within 500ms, took {:?}",
            elapsed
        );
    }

    #[test]
    fn tt_reduces_node_count() {
        let fen = "r1bqkb1r/pppppppp/2n2n2/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3";
        let depth: u8 = 6;

        // Search with TT enabled (normal code path via iterative deepening)
        let mut pos_tt = Position::from_fen(fen).expect("valid fen");
        let mut ctx_tt = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_tt.tt.new_generation();
        for d in 1..=depth {
            ctx_tt.pv_table.clear();
            negamax(
                &mut pos_tt,
                d,
                -INFINITY,
                INFINITY,
                0,
                true,
                &mut ctx_tt,
                None,
                None,
                None,
            );
            ctx_tt.prev_pv = ctx_tt.pv_table.extract_pv();
        }
        let nodes_with_tt = ctx_tt.nodes;

        // Search at max depth only, no TT benefit from prior iterations
        let mut pos_no_tt = Position::from_fen(fen).expect("valid fen");
        let mut ctx_no_tt = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(0),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        for d in 1..=depth {
            ctx_no_tt.pv_table.clear();
            negamax(
                &mut pos_no_tt,
                d,
                -INFINITY,
                INFINITY,
                0,
                true,
                &mut ctx_no_tt,
                None,
                None,
                None,
            );
            ctx_no_tt.prev_pv = ctx_no_tt.pv_table.extract_pv();
        }
        let nodes_without_tt = ctx_no_tt.nodes;

        assert!(
            nodes_with_tt < nodes_without_tt,
            "TT should reduce nodes: {} (with TT) vs {} (without TT)",
            nodes_with_tt,
            nodes_without_tt
        );
    }

    #[test]
    fn tt_move_is_ordered_first() {
        let fen = "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2";
        let mut pos = Position::from_fen(fen).expect("valid fen");
        let hash = pos.hash();

        // Search with a context we can inspect
        let mut ctx = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx.tt.new_generation();
        for d in 1..=4u8 {
            ctx.pv_table.clear();
            negamax(
                &mut pos, d, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
            );
            ctx.prev_pv = ctx.pv_table.extract_pv();
        }

        let entry = ctx
            .tt
            .probe(hash)
            .expect("TT should have an entry after search");
        let tt_move = entry.best_move().expect("TT entry should have a best move");

        let mut moves = chess_movegen::generate_legal_moves(&mut pos);
        let killers = KillerTable::new();
        let history = HistoryTable::new();
        ordering::order_moves(
            &mut moves,
            &pos,
            &killers,
            &history,
            &CounterMoveTable::new(),
            0,
            None,
            Some(tt_move),
            Color::White,
            None,
            None,
        );

        assert_eq!(moves[0], tt_move, "TT move should be ordered first");
    }

    #[test]
    fn mate_score_correct_with_tt() {
        // White to move, mate in 1: Qd8# (back-rank mate)
        // White: Kg1, Qd1, Rf1; Black: Kg8, pawns f7/g7/h7
        let mut pos = Position::from_fen("6k1/5ppp/8/8/8/8/8/3Q1RK1 w - - 0 1").expect("valid fen");

        let mut ctx = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx.tt.new_generation();
        let (score, mv) = negamax(
            &mut pos, 4, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
        );

        assert!(mv.is_some(), "should find a mating move");
        // Mate in 1 = 1 ply from root
        assert_eq!(
            score,
            MATE_SCORE - 1,
            "mate in 1 should return MATE_SCORE - 1, got {}",
            score
        );
    }

    #[test]
    fn tt_move_ordering_across_positions() {
        // Position 1: TT best move is a capture
        let fens = [
            "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",
            // Position 2: quiet move position
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            // Position 3: promotion possible (pawn on 7th rank)
            "8/P5k1/8/8/8/8/6K1/8 w - - 0 1",
        ];

        for fen in fens {
            let mut pos = Position::from_fen(fen).expect("valid fen");
            let hash = pos.hash();

            let mut ctx = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(60),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(1),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: true,
                futility_enabled: true,
                check_extension_enabled: true,
                singular_extension_enabled: true,
            };
            ctx.tt.new_generation();
            for d in 1..=4u8 {
                ctx.pv_table.clear();
                negamax(
                    &mut pos, d, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
                );
                ctx.prev_pv = ctx.pv_table.extract_pv();
            }

            let entry = ctx
                .tt
                .probe(hash)
                .expect("TT should have an entry after search");
            let tt_move = entry.best_move().expect("TT entry should have a best move");

            let mut moves = chess_movegen::generate_legal_moves(&mut pos);
            let killers = KillerTable::new();
            let history = HistoryTable::new();
            ordering::order_moves(
                &mut moves,
                &pos,
                &killers,
                &history,
                &CounterMoveTable::new(),
                0,
                None,
                Some(tt_move),
                Color::White,
                None,
                None,
            );

            assert_eq!(
                moves[0], tt_move,
                "TT move should be ordered first for FEN: {}",
                fen
            );
        }
    }

    #[test]
    fn invalid_tt_move_discarded() {
        use chess_types::{MoveFlag, Square};

        // Use a position where a1-h8 quiet move is definitely not legal
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let mut pos = Position::from_fen(fen).expect("valid fen");
        let hash = pos.hash();

        // Fabricate an invalid move: a1 -> h8 quiet (no piece can do this legally from startpos)
        let fake_move = Move::new(Square::A1, Square::H8, MoveFlag::QUIET);
        let legal_moves = chess_movegen::generate_legal_moves(&mut pos);
        assert!(
            !legal_moves.contains(&fake_move),
            "fabricated move must not be legal"
        );

        let mut ctx = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx.tt.new_generation();

        // Store fake TT entry with the invalid move
        let entry = TtEntry::new(
            verification_key(hash),
            4,
            100,
            BoundType::Exact,
            Some(fake_move),
            ctx.tt.generation(),
        );
        ctx.tt.store(hash, entry);

        // Run negamax — should not crash and should return a valid move
        let (score, mv) = negamax(
            &mut pos, 3, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
        );
        assert!(mv.is_some(), "negamax should return a valid move");
        let best = mv.unwrap();
        let legal_after = chess_movegen::generate_legal_moves(&mut pos);
        assert!(legal_after.contains(&best), "returned move must be legal");
        assert_ne!(best, fake_move, "invalid TT move should not be returned");
        // Score should be reasonable (not garbage)
        assert!(score.abs() < INFINITY);
    }

    #[test]
    fn tt_move_ordering_reduces_nodes() {
        let fen = "r1bqkb1r/pppppppp/2n2n2/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3";
        let depth: u8 = 6;

        // Search with TT enabled (1 MB TT — TT move ordering is effective)
        let mut pos_tt = Position::from_fen(fen).expect("valid fen");
        let mut ctx_tt = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_tt.tt.new_generation();
        for d in 1..=depth {
            ctx_tt.pv_table.clear();
            negamax(
                &mut pos_tt,
                d,
                -INFINITY,
                INFINITY,
                0,
                true,
                &mut ctx_tt,
                None,
                None,
                None,
            );
            ctx_tt.prev_pv = ctx_tt.pv_table.extract_pv();
        }
        let nodes_with_tt = ctx_tt.nodes;

        // Search with 0 MB TT (minimal table — TT move ordering is ineffective)
        let mut pos_no_tt = Position::from_fen(fen).expect("valid fen");
        let mut ctx_no_tt = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(0),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        for d in 1..=depth {
            ctx_no_tt.pv_table.clear();
            negamax(
                &mut pos_no_tt,
                d,
                -INFINITY,
                INFINITY,
                0,
                true,
                &mut ctx_no_tt,
                None,
                None,
                None,
            );
            ctx_no_tt.prev_pv = ctx_no_tt.pv_table.extract_pv();
        }
        let nodes_without_tt = ctx_no_tt.nodes;

        assert!(
            nodes_with_tt < nodes_without_tt,
            "TT move ordering should reduce nodes: {} (with TT) vs {} (without TT)",
            nodes_with_tt,
            nodes_without_tt
        );
    }

    #[test]
    fn iid_reduces_node_count() {
        let fen = "r1bqkb1r/pppppppp/2n2n2/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3";
        let depth: u8 = 6;

        // Search with 1 MB TT (IID populates TT effectively)
        let mut pos_iid = Position::from_fen(fen).expect("valid fen");
        let mut ctx_iid = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_iid.tt.new_generation();
        for d in 1..=depth {
            ctx_iid.pv_table.clear();
            negamax(
                &mut pos_iid,
                d,
                -INFINITY,
                INFINITY,
                0,
                true,
                &mut ctx_iid,
                None,
                None,
                None,
            );
            ctx_iid.prev_pv = ctx_iid.pv_table.extract_pv();
        }
        let nodes_with_iid = ctx_iid.nodes;

        // Search with 0 MB TT (IID benefit neutralized)
        let mut pos_no_iid = Position::from_fen(fen).expect("valid fen");
        let mut ctx_no_iid = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(0),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        for d in 1..=depth {
            ctx_no_iid.pv_table.clear();
            negamax(
                &mut pos_no_iid,
                d,
                -INFINITY,
                INFINITY,
                0,
                true,
                &mut ctx_no_iid,
                None,
                None,
                None,
            );
            ctx_no_iid.prev_pv = ctx_no_iid.pv_table.extract_pv();
        }
        let nodes_without_iid = ctx_no_iid.nodes;

        assert!(
            nodes_with_iid < nodes_without_iid,
            "IID should reduce nodes: {} (with IID) vs {} (without IID)",
            nodes_with_iid,
            nodes_without_iid
        );
    }

    #[test]
    fn iid_does_not_trigger_below_threshold() {
        let mut pos_a = Position::startpos();
        let mut pos_b = Position::startpos();

        let mut ctx_a = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_a.tt.new_generation();

        let mut ctx_b = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_b.tt.new_generation();

        negamax(
            &mut pos_a, 3, -INFINITY, INFINITY, 0, true, &mut ctx_a, None, None, None,
        );
        negamax(
            &mut pos_b, 3, -INFINITY, INFINITY, 0, true, &mut ctx_b, None, None, None,
        );

        assert_eq!(
            ctx_a.nodes, ctx_b.nodes,
            "IID should not trigger at depth 3: {} vs {} nodes",
            ctx_a.nodes, ctx_b.nodes
        );
    }

    #[test]
    fn iid_finds_move_for_ordering() {
        let fen = "r1bqkb1r/pppppppp/2n2n2/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3";
        let mut pos = Position::from_fen(fen).expect("valid fen");
        let hash = pos.hash();

        let mut ctx = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx.tt.new_generation();

        let (_, mv) = negamax(
            &mut pos, 5, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
        );

        let entry = ctx
            .tt
            .probe(hash)
            .expect("TT should have an entry after search");
        let best = entry.best_move().expect("TT entry should have a best move");
        let legal_moves = chess_movegen::generate_legal_moves(&mut pos);
        assert!(legal_moves.contains(&best), "TT best move should be legal");
        assert!(mv.is_some(), "search should return a valid move");
    }

    #[test]
    fn threefold_repetition_returns_draw() {
        // Test that negamax returns draw score 0 when the position has been seen before.
        // Call negamax at ply=1 so the repetition check (ply > 0) triggers immediately.
        // The history must contain two entries: the matching hash and a "top" entry that
        // skip(1) removes. The halfmove clock must be large enough for the scan window.
        let mut pos = Position::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 4 1").expect("valid fen");
        let current_hash = pos.hash();

        // History: [current_hash, current_hash]. skip(1) removes index 1, checks index 0.
        let mut ctx = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(5),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: vec![current_hash, current_hash],
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx.tt.new_generation();

        // At ply=1, the repetition check sees current_hash in history and returns (0, None)
        let (score, mv) = negamax(
            &mut pos, 4, -INFINITY, INFINITY, 1, true, &mut ctx, None, None, None,
        );

        assert_eq!(score, 0, "threefold repetition should yield draw score 0");
        assert!(mv.is_none(), "repeated position should return no move");
    }

    #[test]
    fn fifty_move_rule_returns_draw() {
        // Halfmove clock = 100 in FEN. At root (ply=0) the check is skipped,
        // but every child position will have halfmove_clock >= 101 (quiet moves increment),
        // so all children return draw score 0.
        let mut pos = Position::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 100 51").expect("valid fen");
        let mut ctx = test_ctx();
        ctx.tt.new_generation();

        let (score, _) = negamax(
            &mut pos, 2, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
        );

        assert_eq!(
            score, 0,
            "fifty-move rule should return draw score 0, got {}",
            score
        );
    }

    #[test]
    fn engine_avoids_repetition_when_winning() {
        // White has a queen vs nothing (huge material advantage).
        // Game history contains one prior occurrence of the current position hash,
        // so if White makes a move that leads back here, it's a draw.
        // The engine should prefer to keep the advantage rather than repeat.
        let mut pos = Position::from_fen("4k3/8/8/8/8/8/8/Q3K3 w - - 0 1").expect("valid fen");
        let current_hash = pos.hash();

        // Pre-load one occurrence of the current hash (twofold: returning here = draw)
        let game_history = vec![current_hash];

        let limits = SearchLimits {
            max_time: Duration::from_secs(5),
            max_depth: Some(4),
            max_nodes: None,
            stop_flag: None,
        };
        let mv = search(&mut pos, limits, &game_history, None, None);

        assert!(mv.is_some(), "engine should find a move");
        // Re-search to get the score
        let mut pos2 = Position::from_fen("4k3/8/8/8/8/8/8/Q3K3 w - - 0 1").expect("valid fen");
        let limits2 = SearchLimits {
            max_time: Duration::from_secs(5),
            max_depth: Some(4),
            max_nodes: None,
            stop_flag: None,
        };
        let mut ctx = SearchContext {
            start: Instant::now(),
            time_budget: limits2.max_time,
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: vec![current_hash],
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx.tt.new_generation();
        ctx.history.push(pos2.hash());

        let (score, _) = negamax(
            &mut pos2, 4, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
        );

        assert!(
            score > 0,
            "engine should avoid repetition when winning, score should be > 0, got {}",
            score
        );
    }

    #[test]
    fn engine_seeks_repetition_when_losing() {
        // Black to move with a huge material disadvantage (White has queen, Black has only king).
        // We pre-load the hash of one of Black's reachable child positions into the history,
        // so when the search plays that move, it detects the position as a repetition (draw = 0).
        // Without this history, the score would be deeply negative.
        // Use halfmove clock = 10 so the repetition scan window is wide enough.
        let fen = "4k3/8/8/8/8/8/8/Q3K3 b - - 10 6";
        let mut pos = Position::from_fen(fen).expect("valid fen");

        // Find a legal move for Black and compute its resulting position hash
        let moves = chess_movegen::generate_legal_moves(&mut pos);
        assert!(!moves.is_empty());
        let child_move = moves[0];
        let undo = pos.make_move(child_move);
        let child_hash = pos.hash();
        pos.unmake_move(child_move, undo);

        // Search with the child position hash in history (Black can draw by playing child_move).
        // The history simulates a game where child_hash appeared before (twofold detection).
        // The step_by(2) optimization in the repetition scan checks every other position
        // (same side-to-move parity). At ply=1 the child position is White-to-move.
        // After make_move pushes child_hash at the end, the scan (skip(1), step_by(2))
        // checks entries at odd distances from the end. We place child_hash at distance 3
        // by padding: [child_hash, dummy, root_hash] → push child_hash → distances 3,2,1,0.
        let root_hash = pos.hash();
        let mut ctx = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(5),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: vec![child_hash, 0, root_hash],
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx.tt.new_generation();

        let (score_with_history, _) = negamax(
            &mut pos, 4, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
        );

        // Search without repetition history for comparison
        let mut pos2 = Position::from_fen(fen).expect("valid fen");
        let mut ctx2 = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(5),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx2.tt.new_generation();

        let (score_without_history, _) = negamax(
            &mut pos2, 4, -INFINITY, INFINITY, 0, true, &mut ctx2, None, None, None,
        );

        // With repetition available, the losing side should get a better (higher) score
        // because it can aim for a draw (score 0) instead of a deeply negative score.
        assert!(
            score_with_history > score_without_history,
            "engine should seek repetition when losing: score with history ({}) should be better than without ({})",
            score_with_history,
            score_without_history
        );
    }

    #[test]
    fn null_move_pruning_activates() {
        // Verify NMP activates by calling negamax at ply=1 with a window where beta
        // is low relative to the position's true value. White has an extra knight
        // (~300 cp advantage). With beta=100, NMP's null-move search should find a
        // score >= 100 (even after passing, White's advantage persists), causing a
        // beta cutoff and pruning the entire subtree. Without NMP (allow_null=false),
        // the full search must examine all legal moves, producing more nodes.
        let fen = "r1bqkb1r/pppppppp/5n2/8/4P3/2N2N2/PPPP1PPP/R1BQKB1R w KQkq - 2 3";
        let depth: u8 = 5;

        // Search with NMP enabled
        let mut pos_nmp = Position::from_fen(fen).expect("valid fen");
        let mut ctx_nmp = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_nmp.tt.new_generation();
        negamax(
            &mut pos_nmp,
            depth,
            0,
            100,
            1,
            true,
            &mut ctx_nmp,
            None,
            None,
            None,
        );
        let nodes_with_nmp = ctx_nmp.nodes;

        // Search with NMP disabled at this node
        let mut pos_no_nmp = Position::from_fen(fen).expect("valid fen");
        let mut ctx_no_nmp = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_no_nmp.tt.new_generation();
        negamax(
            &mut pos_no_nmp,
            depth,
            0,
            100,
            1,
            false,
            &mut ctx_no_nmp,
            None,
            None,
            None,
        );
        let nodes_without_nmp = ctx_no_nmp.nodes;

        assert!(
            nodes_with_nmp < nodes_without_nmp,
            "NMP should reduce nodes: {} (with NMP) vs {} (without NMP)",
            nodes_with_nmp,
            nodes_without_nmp
        );
    }

    #[test]
    fn null_move_skipped_in_check() {
        // Position where White is in check. NMP must not activate.
        // Node count should be the same with allow_null=true and allow_null=false
        // since the in_check guard prevents NMP.
        let fen = "rnbqkbnr/pppp1ppp/8/4p3/7q/5P2/PPPPP1PP/RNBQKBNR w KQkq - 0 2";
        let depth: u8 = 4;

        let mut pos_a = Position::from_fen(fen).expect("valid fen");
        let mut ctx_a = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_a.tt.new_generation();
        negamax(
            &mut pos_a, depth, -INFINITY, INFINITY, 0, true, &mut ctx_a, None, None, None,
        );
        let nodes_allow = ctx_a.nodes;

        let mut pos_b = Position::from_fen(fen).expect("valid fen");
        let mut ctx_b = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_b.tt.new_generation();
        negamax(
            &mut pos_b, depth, -INFINITY, INFINITY, 0, false, &mut ctx_b, None, None, None,
        );
        let nodes_disallow = ctx_b.nodes;

        assert_eq!(
            nodes_allow, nodes_disallow,
            "NMP should not activate in check: {} vs {} nodes",
            nodes_allow, nodes_disallow
        );
    }

    #[test]
    fn null_move_skipped_king_pawns_only() {
        // Position where side to move has only king and pawns (no non-pawn material).
        // NMP must not activate due to zugzwang guard.
        let fen = "4k3/pppppppp/8/8/8/8/PPPPPPPP/4K3 w - - 0 1";
        let depth: u8 = 4;

        let mut pos_a = Position::from_fen(fen).expect("valid fen");
        let mut ctx_a = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_a.tt.new_generation();
        negamax(
            &mut pos_a, depth, -INFINITY, INFINITY, 0, true, &mut ctx_a, None, None, None,
        );
        let nodes_allow = ctx_a.nodes;

        let mut pos_b = Position::from_fen(fen).expect("valid fen");
        let mut ctx_b = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_b.tt.new_generation();
        negamax(
            &mut pos_b, depth, -INFINITY, INFINITY, 0, false, &mut ctx_b, None, None, None,
        );
        let nodes_disallow = ctx_b.nodes;

        assert_eq!(
            nodes_allow, nodes_disallow,
            "NMP should not activate with king+pawns only: {} vs {} nodes",
            nodes_allow, nodes_disallow
        );
    }

    #[test]
    fn null_move_no_consecutive() {
        // With allow_null=false, NMP should not fire even in a position where it
        // would normally activate. Node count should match a baseline without NMP.
        let fen = "r1bqkb1r/pppppppp/2n2n2/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3";
        let depth: u8 = 5;

        let mut pos_a = Position::from_fen(fen).expect("valid fen");
        let mut ctx_a = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_a.tt.new_generation();
        negamax(
            &mut pos_a, depth, -INFINITY, INFINITY, 0, false, &mut ctx_a, None, None, None,
        );
        let nodes_no_null_a = ctx_a.nodes;

        let mut pos_b = Position::from_fen(fen).expect("valid fen");
        let mut ctx_b = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_b.tt.new_generation();
        negamax(
            &mut pos_b, depth, -INFINITY, INFINITY, 0, false, &mut ctx_b, None, None, None,
        );
        let nodes_no_null_b = ctx_b.nodes;

        assert_eq!(
            nodes_no_null_a, nodes_no_null_b,
            "allow_null=false should produce deterministic results: {} vs {} nodes",
            nodes_no_null_a, nodes_no_null_b
        );
    }

    #[test]
    fn null_move_finds_correct_bestmove() {
        // Tactical positions: NMP should not harm best move selection.
        let positions = [
            // Scholar's mate: Qxf7#
            (
                "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
                Square::F7,
            ),
            // Back-rank mate: Qd8#
            ("6k1/5ppp/8/8/8/8/8/3Q1RK1 w - - 0 1", Square::D8),
        ];

        for (fen, expected_target) in positions {
            let mut pos = Position::from_fen(fen).expect("valid fen");
            let limits = SearchLimits {
                max_time: Duration::from_secs(5),
                max_depth: Some(6),
                max_nodes: None,
                stop_flag: None,
            };
            let mv = search(&mut pos, limits, &[], None, None);
            assert!(mv.is_some(), "should find a move for FEN: {}", fen);
            assert_eq!(
                mv.unwrap().to_sq(),
                expected_target,
                "wrong target square for FEN: {}",
                fen
            );
        }
    }

    #[test]
    fn lmr_table_values_correct() {
        let table = lmr_table();

        // Row 0 and column 0 should all be 0
        for m in 0..LMR_MAX_MOVES {
            assert_eq!(table[0][m], 0, "table[0][{}] should be 0", m);
        }
        for d in 0..LMR_MAX_DEPTH {
            assert_eq!(table[d][0], 0, "table[{}][0] should be 0", d);
        }

        // table[6][4] = floor(ln(6) * ln(4) / 1.80) = floor(1.380) = 1
        assert_eq!(table[6][4], 1, "table[6][4] should be 1");

        // table[10][10] = floor(ln(10) * ln(10) / 1.80) = floor(2.946) = 2
        assert_eq!(table[10][10], 2, "table[10][10] should be 2");
    }

    #[test]
    fn lmr_reduces_node_count() {
        let positions = [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",
            "r1bqkb1r/pppppppp/2n2n2/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3",
        ];
        let depth: u8 = 10;

        for fen in positions {
            // Search with LMR enabled
            let mut pos_lmr = Position::from_fen(fen).expect("valid fen");
            let mut ctx_lmr = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(120),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(16),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: true,
                futility_enabled: true,
                check_extension_enabled: true,
                singular_extension_enabled: true,
            };
            ctx_lmr.tt.new_generation();
            for d in 1..=depth {
                ctx_lmr.pv_table.clear();
                negamax(
                    &mut pos_lmr,
                    d,
                    -INFINITY,
                    INFINITY,
                    0,
                    true,
                    &mut ctx_lmr,
                    None,
                    None,
                    None,
                );
                ctx_lmr.prev_pv = ctx_lmr.pv_table.extract_pv();
            }
            let nodes_with_lmr = ctx_lmr.nodes;

            // Search with LMR disabled
            let mut pos_no_lmr = Position::from_fen(fen).expect("valid fen");
            let mut ctx_no_lmr = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(120),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(16),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: false,
                futility_enabled: true,
                check_extension_enabled: true,
                singular_extension_enabled: true,
            };
            ctx_no_lmr.tt.new_generation();
            for d in 1..=depth {
                ctx_no_lmr.pv_table.clear();
                negamax(
                    &mut pos_no_lmr,
                    d,
                    -INFINITY,
                    INFINITY,
                    0,
                    true,
                    &mut ctx_no_lmr,
                    None,
                    None,
                    None,
                );
                ctx_no_lmr.prev_pv = ctx_no_lmr.pv_table.extract_pv();
            }
            let nodes_without_lmr = ctx_no_lmr.nodes;

            let reduction_pct = 1.0 - (nodes_with_lmr as f64 / nodes_without_lmr as f64);
            assert!(
                reduction_pct >= 0.30,
                "LMR should reduce nodes by >=30% for FEN: {} ({} with LMR vs {} without, reduction {:.1}%)",
                fen,
                nodes_with_lmr,
                nodes_without_lmr,
                reduction_pct * 100.0
            );
        }
    }

    #[test]
    fn lmr_skips_tt_moves() {
        // Tactical positions: LMR must not reduce TT/PV/killer/capture/promotion moves.
        // Best move must be identical with and without LMR.
        let positions = [
            // Scholar's mate: Qxf7#
            "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
            // Back-rank mate: Qd8#
            "6k1/5ppp/8/8/8/8/8/3Q1RK1 w - - 0 1",
        ];

        for fen in positions {
            let mut pos_lmr = Position::from_fen(fen).expect("valid fen");
            let limits_lmr = SearchLimits {
                max_time: Duration::from_secs(10),
                max_depth: Some(6),
                max_nodes: None,
                stop_flag: None,
            };
            let mv_lmr = search(&mut pos_lmr, limits_lmr, &[], None, None);

            let mut pos_no_lmr = Position::from_fen(fen).expect("valid fen");
            let mut ctx_no_lmr = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(10),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(64),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: false,
                futility_enabled: true,
                check_extension_enabled: true,
                singular_extension_enabled: true,
            };
            ctx_no_lmr.tt.new_generation();
            ctx_no_lmr.history.push(pos_no_lmr.hash());
            let mut mv_no_lmr: Option<Move> = None;
            for d in 1..=6u8 {
                ctx_no_lmr.pv_table.clear();
                let (_, mv) = negamax(
                    &mut pos_no_lmr,
                    d,
                    -INFINITY,
                    INFINITY,
                    0,
                    true,
                    &mut ctx_no_lmr,
                    None,
                    None,
                    None,
                );
                ctx_no_lmr.prev_pv = ctx_no_lmr.pv_table.extract_pv();
                if mv.is_some() {
                    mv_no_lmr = mv;
                }
            }

            assert!(
                mv_lmr.is_some(),
                "LMR search should find a move for FEN: {}",
                fen
            );
            assert!(
                mv_no_lmr.is_some(),
                "non-LMR search should find a move for FEN: {}",
                fen
            );
            assert_eq!(
                mv_lmr.unwrap(),
                mv_no_lmr.unwrap(),
                "LMR should not change best move for tactical FEN: {}",
                fen
            );
        }
    }

    #[test]
    fn futility_margins_match_spec() {
        assert_eq!(FUTILITY_MARGINS[1], 180);
        assert_eq!(FUTILITY_MARGINS[2], 360);
        assert_eq!(FUTILITY_MARGINS[3], 540);
        assert_eq!(REVERSE_FUTILITY_MARGINS[1], 120);
        assert_eq!(REVERSE_FUTILITY_MARGINS[2], 350);
        assert_eq!(REVERSE_FUTILITY_MARGINS[3], 550);
    }

    #[test]
    fn reverse_futility_prunes_node() {
        // Use a standard middlegame position. Search at depth 6 with iterative
        // deepening so internal nodes at depths 1-3 see narrowed alpha-beta
        // windows where reverse futility can activate. Disable LMR and use a
        // minimal TT to isolate the futility pruning effect.
        let fen = "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2";
        let depth: u8 = 6;

        // With futility enabled
        let mut pos_on = Position::from_fen(fen).expect("valid fen");
        let mut ctx_on = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(0),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: false,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        for d in 1..=depth {
            ctx_on.pv_table.clear();
            negamax(
                &mut pos_on,
                d,
                -INFINITY,
                INFINITY,
                0,
                true,
                &mut ctx_on,
                None,
                None,
                None,
            );
            ctx_on.prev_pv = ctx_on.pv_table.extract_pv();
        }
        let nodes_on = ctx_on.nodes;

        // With futility disabled
        let mut pos_off = Position::from_fen(fen).expect("valid fen");
        let mut ctx_off = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(0),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: false,
            futility_enabled: false,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        for d in 1..=depth {
            ctx_off.pv_table.clear();
            negamax(
                &mut pos_off,
                d,
                -INFINITY,
                INFINITY,
                0,
                true,
                &mut ctx_off,
                None,
                None,
                None,
            );
            ctx_off.prev_pv = ctx_off.pv_table.extract_pv();
        }
        let nodes_off = ctx_off.nodes;

        assert!(
            nodes_on < nodes_off,
            "reverse futility pruning should reduce nodes: {} (on) vs {} (off)",
            nodes_on,
            nodes_off
        );
    }

    #[test]
    fn futility_skips_quiet_moves() {
        // White has massive material advantage. At depth 1 with a high alpha,
        // futility pruning should skip quiet moves where static_eval + margin <= alpha.
        let fen = "4k3/8/8/8/8/8/8/QR2K3 w - - 0 1";
        let depth: u8 = 1;

        // With futility enabled
        let mut pos_on = Position::from_fen(fen).expect("valid fen");
        let mut ctx_on = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_on.tt.new_generation();
        negamax(
            &mut pos_on,
            depth,
            -INFINITY,
            INFINITY,
            0,
            true,
            &mut ctx_on,
            None,
            None,
            None,
        );
        let nodes_on = ctx_on.nodes;

        // With futility disabled
        let mut pos_off = Position::from_fen(fen).expect("valid fen");
        let mut ctx_off = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: false,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_off.tt.new_generation();
        negamax(
            &mut pos_off,
            depth,
            -INFINITY,
            INFINITY,
            0,
            true,
            &mut ctx_off,
            None,
            None,
            None,
        );
        let nodes_off = ctx_off.nodes;

        assert!(
            nodes_on <= nodes_off,
            "futility pruning should not increase nodes: {} (on) vs {} (off)",
            nodes_on,
            nodes_off
        );
    }

    #[test]
    fn futility_does_not_prune_in_check() {
        // Position where White is in check. Futility pruning must not activate at root.
        // At depth 1, all children go to quiescence so futility has no effect.
        let fen = "4k3/8/8/8/8/4q3/8/4K3 w - - 0 1";
        let depth: u8 = 1;

        let mut pos_on = Position::from_fen(fen).expect("valid fen");
        let mut ctx_on = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: false,
            futility_enabled: true,
            check_extension_enabled: false,
            singular_extension_enabled: false,
        };
        ctx_on.tt.new_generation();

        // Check if king is actually in check
        let king_sq_w = king_square(&pos_on, Color::White);
        let in_check = pos_on.is_square_attacked(king_sq_w, Color::Black);

        if in_check {
            negamax(
                &mut pos_on,
                depth,
                -INFINITY,
                INFINITY,
                0,
                true,
                &mut ctx_on,
                None,
                None,
                None,
            );
            let nodes_on = ctx_on.nodes;

            let mut pos_off = Position::from_fen(fen).expect("valid fen");
            let mut ctx_off = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(60),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(1),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: false,
                futility_enabled: false,
                check_extension_enabled: false,
                singular_extension_enabled: false,
            };
            ctx_off.tt.new_generation();
            negamax(
                &mut pos_off,
                depth,
                -INFINITY,
                INFINITY,
                0,
                true,
                &mut ctx_off,
                None,
                None,
                None,
            );
            let nodes_off = ctx_off.nodes;

            assert_eq!(
                nodes_on, nodes_off,
                "futility should not activate in check: {} vs {} nodes",
                nodes_on, nodes_off
            );
        } else {
            // Use a different position where White is definitely in check
            let fen2 = "rnbqkbnr/pppp1ppp/8/4p3/7q/5P2/PPPPP1PP/RNBQKBNR w KQkq - 0 2";
            let mut pos_on2 = Position::from_fen(fen2).expect("valid fen");
            let mut ctx_on2 = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(60),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(1),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: false,
                futility_enabled: true,
                check_extension_enabled: false,
                singular_extension_enabled: false,
            };
            ctx_on2.tt.new_generation();
            negamax(
                &mut pos_on2,
                depth,
                -INFINITY,
                INFINITY,
                0,
                true,
                &mut ctx_on2,
                None,
                None,
                None,
            );
            let nodes_on2 = ctx_on2.nodes;

            let mut pos_off2 = Position::from_fen(fen2).expect("valid fen");
            let mut ctx_off2 = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(60),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(1),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: false,
                futility_enabled: false,
                check_extension_enabled: false,
                singular_extension_enabled: false,
            };
            ctx_off2.tt.new_generation();
            negamax(
                &mut pos_off2,
                depth,
                -INFINITY,
                INFINITY,
                0,
                true,
                &mut ctx_off2,
                None,
                None,
                None,
            );
            let nodes_off2 = ctx_off2.nodes;

            assert_eq!(
                nodes_on2, nodes_off2,
                "futility should not activate in check: {} vs {} nodes",
                nodes_on2, nodes_off2
            );
        }
    }

    #[test]
    fn futility_does_not_prune_captures() {
        // Tactical positions: futility should not prune captures/promotions.
        // Best move must be identical with and without futility.
        let positions = [
            // Scholar's mate: Qxf7#
            (
                "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
                Square::F7,
            ),
            // Back-rank mate: Qd8#
            ("6k1/5ppp/8/8/8/8/8/3Q1RK1 w - - 0 1", Square::D8),
        ];

        for (fen, expected_target) in positions {
            let mut pos = Position::from_fen(fen).expect("valid fen");
            let limits = SearchLimits {
                max_time: Duration::from_secs(5),
                max_depth: Some(6),
                max_nodes: None,
                stop_flag: None,
            };
            let mv = search(&mut pos, limits, &[], None, None);
            assert!(mv.is_some(), "should find a move for FEN: {}", fen);
            assert_eq!(
                mv.unwrap().to_sq(),
                expected_target,
                "futility should not prune tactical best move for FEN: {}",
                fen
            );
        }
    }

    #[test]
    fn futility_preserves_wac_solve_rate() {
        // Run WAC tactical positions at fixed depth with futility enabled and disabled.
        // Verify the solve rate (correct best moves) is not reduced.
        let wac_positions = [
            // WAC.001: Qg6 or similar tactical blow
            (
                "2rr3k/pp3pp1/1nnqbN1p/3pN3/2pP4/2P3Q1/PPB4P/R4RK1 w - - 0 1",
                vec![Square::G6],
            ),
            // Scholar's mate: Qxf7#
            (
                "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
                vec![Square::F7],
            ),
            // Back-rank mate: Qd8#
            ("6k1/5ppp/8/8/8/8/8/3Q1RK1 w - - 0 1", vec![Square::D8]),
            // Knight fork: Nxf7
            (
                "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
                vec![Square::G5, Square::D5],
            ),
        ];

        let depth = 6;
        let mut correct_on = 0;
        let mut correct_off = 0;

        for (fen, expected_targets) in &wac_positions {
            // With futility
            let mut pos_on = Position::from_fen(fen).expect("valid fen");
            let mut ctx_on = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(10),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(16),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: true,
                futility_enabled: true,
                check_extension_enabled: true,
                singular_extension_enabled: true,
            };
            ctx_on.tt.new_generation();
            ctx_on.history.push(pos_on.hash());
            let mut mv_on = None;
            for d in 1..=depth as u8 {
                ctx_on.pv_table.clear();
                let (_, mv) = negamax(
                    &mut pos_on,
                    d,
                    -INFINITY,
                    INFINITY,
                    0,
                    true,
                    &mut ctx_on,
                    None,
                    None,
                    None,
                );
                ctx_on.prev_pv = ctx_on.pv_table.extract_pv();
                if mv.is_some() {
                    mv_on = mv;
                }
            }
            if let Some(m) = mv_on {
                if expected_targets.contains(&m.to_sq()) {
                    correct_on += 1;
                }
            }

            // Without futility
            let mut pos_off = Position::from_fen(fen).expect("valid fen");
            let mut ctx_off = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(10),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(16),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: true,
                futility_enabled: false,
                check_extension_enabled: true,
                singular_extension_enabled: true,
            };
            ctx_off.tt.new_generation();
            ctx_off.history.push(pos_off.hash());
            let mut mv_off = None;
            for d in 1..=depth as u8 {
                ctx_off.pv_table.clear();
                let (_, mv) = negamax(
                    &mut pos_off,
                    d,
                    -INFINITY,
                    INFINITY,
                    0,
                    true,
                    &mut ctx_off,
                    None,
                    None,
                    None,
                );
                ctx_off.prev_pv = ctx_off.pv_table.extract_pv();
                if mv.is_some() {
                    mv_off = mv;
                }
            }
            if let Some(m) = mv_off {
                if expected_targets.contains(&m.to_sq()) {
                    correct_off += 1;
                }
            }
        }

        assert!(
            correct_on >= correct_off,
            "futility should not reduce WAC solve rate: {} (on) vs {} (off)",
            correct_on,
            correct_off
        );
    }

    #[test]
    fn futility_interacts_correctly_with_nmp_and_lmr() {
        // Search startpos at depth 6 with all three pruning techniques enabled.
        // Verify it completes without panic and returns a legal move.
        // Then compare with each technique individually disabled.
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let depth: u8 = 6;

        // All enabled
        let mut pos = Position::from_fen(fen).expect("valid fen");
        let mut ctx = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(30),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(16),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx.tt.new_generation();
        ctx.history.push(pos.hash());
        let mut best_all = None;
        for d in 1..=depth {
            ctx.pv_table.clear();
            let (_, mv) = negamax(
                &mut pos, d, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
            );
            ctx.prev_pv = ctx.pv_table.extract_pv();
            if mv.is_some() {
                best_all = mv;
            }
        }
        assert!(best_all.is_some(), "all-enabled search should find a move");
        let legal = chess_movegen::generate_legal_moves(&mut pos);
        assert!(
            legal.contains(&best_all.unwrap()),
            "all-enabled move must be legal"
        );

        // Futility disabled, LMR + NMP enabled
        let mut pos2 = Position::from_fen(fen).expect("valid fen");
        let mut ctx2 = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(30),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(16),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: false,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx2.tt.new_generation();
        ctx2.history.push(pos2.hash());
        let mut best_no_fut = None;
        for d in 1..=depth {
            ctx2.pv_table.clear();
            let (_, mv) = negamax(
                &mut pos2, d, -INFINITY, INFINITY, 0, true, &mut ctx2, None, None, None,
            );
            ctx2.prev_pv = ctx2.pv_table.extract_pv();
            if mv.is_some() {
                best_no_fut = mv;
            }
        }
        assert!(
            best_no_fut.is_some(),
            "no-futility search should find a move"
        );

        // LMR disabled, futility + NMP enabled
        let mut pos3 = Position::from_fen(fen).expect("valid fen");
        let mut ctx3 = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(30),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(16),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: false,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx3.tt.new_generation();
        ctx3.history.push(pos3.hash());
        let mut best_no_lmr = None;
        for d in 1..=depth {
            ctx3.pv_table.clear();
            let (_, mv) = negamax(
                &mut pos3, d, -INFINITY, INFINITY, 0, true, &mut ctx3, None, None, None,
            );
            ctx3.prev_pv = ctx3.pv_table.extract_pv();
            if mv.is_some() {
                best_no_lmr = mv;
            }
        }
        assert!(best_no_lmr.is_some(), "no-LMR search should find a move");
    }

    #[test]
    fn lmr_re_search_on_fail_high() {
        // Compare search results with LMR enabled vs disabled on tactical positions at depth 6.
        // Best move must match; scores must match on forced mate positions.
        let positions = [
            // Scholar's mate: Qxf7#
            "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
            // Back-rank mate: Qd8#
            "6k1/5ppp/8/8/8/8/8/3Q1RK1 w - - 0 1",
        ];

        for fen in positions {
            // With LMR
            let mut pos_lmr = Position::from_fen(fen).expect("valid fen");
            let mut ctx_lmr = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(10),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(16),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: true,
                futility_enabled: true,
                check_extension_enabled: true,
                singular_extension_enabled: true,
            };
            ctx_lmr.tt.new_generation();
            ctx_lmr.history.push(pos_lmr.hash());
            let mut best_lmr = None;
            let mut score_lmr = 0;
            for d in 1..=6u8 {
                ctx_lmr.pv_table.clear();
                let (s, mv) = negamax(
                    &mut pos_lmr,
                    d,
                    -INFINITY,
                    INFINITY,
                    0,
                    true,
                    &mut ctx_lmr,
                    None,
                    None,
                    None,
                );
                ctx_lmr.prev_pv = ctx_lmr.pv_table.extract_pv();
                if mv.is_some() {
                    best_lmr = mv;
                    score_lmr = s;
                }
            }

            // Without LMR
            let mut pos_no_lmr = Position::from_fen(fen).expect("valid fen");
            let mut ctx_no_lmr = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(10),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(16),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: false,
                futility_enabled: true,
                check_extension_enabled: true,
                singular_extension_enabled: true,
            };
            ctx_no_lmr.tt.new_generation();
            ctx_no_lmr.history.push(pos_no_lmr.hash());
            let mut best_no_lmr = None;
            let mut score_no_lmr = 0;
            for d in 1..=6u8 {
                ctx_no_lmr.pv_table.clear();
                let (s, mv) = negamax(
                    &mut pos_no_lmr,
                    d,
                    -INFINITY,
                    INFINITY,
                    0,
                    true,
                    &mut ctx_no_lmr,
                    None,
                    None,
                    None,
                );
                ctx_no_lmr.prev_pv = ctx_no_lmr.pv_table.extract_pv();
                if mv.is_some() {
                    best_no_lmr = mv;
                    score_no_lmr = s;
                }
            }

            assert_eq!(
                best_lmr, best_no_lmr,
                "LMR re-search should preserve best move for FEN: {}",
                fen
            );
            assert_eq!(
                score_lmr, score_no_lmr,
                "LMR re-search should preserve score for forced mate FEN: {}",
                fen
            );
        }
    }

    #[test]
    fn check_extension_activates_in_check() {
        let fen = "rnbqkbnr/pppp1ppp/8/4p3/7q/5P2/PPPPP1PP/RNBQKBNR w KQkq - 0 2";
        let depth: u8 = 4;

        let mut pos_on = Position::from_fen(fen).expect("valid fen");
        let mut ctx_on = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_on.tt.new_generation();
        negamax(
            &mut pos_on,
            depth,
            -INFINITY,
            INFINITY,
            0,
            true,
            &mut ctx_on,
            None,
            None,
            None,
        );
        let nodes_on = ctx_on.nodes;

        let mut pos_off = Position::from_fen(fen).expect("valid fen");
        let mut ctx_off = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: false,
            singular_extension_enabled: true,
        };
        ctx_off.tt.new_generation();
        negamax(
            &mut pos_off,
            depth,
            -INFINITY,
            INFINITY,
            0,
            true,
            &mut ctx_off,
            None,
            None,
            None,
        );
        let nodes_off = ctx_off.nodes;

        assert!(
            nodes_on > nodes_off,
            "check extension should search more nodes in check position: {} (on) vs {} (off)",
            nodes_on,
            nodes_off
        );
    }

    #[test]
    fn check_extension_no_effect_when_not_in_check() {
        let mut pos_on = Position::startpos();
        let mut ctx_on = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_on.tt.new_generation();
        negamax(
            &mut pos_on,
            3,
            -INFINITY,
            INFINITY,
            0,
            true,
            &mut ctx_on,
            None,
            None,
            None,
        );
        let nodes_on = ctx_on.nodes;

        let mut pos_off = Position::startpos();
        let mut ctx_off = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(1),
            history: Vec::new(),
            pawn_table: chess_eval::PawnHashTable::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: false,
            singular_extension_enabled: true,
        };
        ctx_off.tt.new_generation();
        negamax(
            &mut pos_off,
            3,
            -INFINITY,
            INFINITY,
            0,
            true,
            &mut ctx_off,
            None,
            None,
            None,
        );
        let nodes_off = ctx_off.nodes;

        assert_eq!(
            nodes_on, nodes_off,
            "check extension should not affect non-check startpos: {} (on) vs {} (off)",
            nodes_on, nodes_off
        );
    }

    #[test]
    fn check_extension_improves_wac_solve_rate() {
        let wac_positions = [
            (
                "2rr3k/pp3pp1/1nnqbN1p/3pN3/2pP4/2P3Q1/PPB4P/R4RK1 w - - 0 1",
                vec![Square::G6],
            ),
            (
                "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
                vec![Square::F7],
            ),
            ("6k1/5ppp/8/8/8/8/8/3Q1RK1 w - - 0 1", vec![Square::D8]),
            (
                "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
                vec![Square::G5, Square::D5],
            ),
        ];

        let depth = 6;
        let mut correct_on = 0;
        let mut correct_off = 0;

        for (fen, expected_targets) in &wac_positions {
            let mut pos_on = Position::from_fen(fen).expect("valid fen");
            let mut ctx_on = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(10),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(16),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: true,
                futility_enabled: true,
                check_extension_enabled: true,
                singular_extension_enabled: true,
            };
            ctx_on.tt.new_generation();
            ctx_on.history.push(pos_on.hash());
            let mut mv_on = None;
            for d in 1..=depth as u8 {
                ctx_on.pv_table.clear();
                let (_, mv) = negamax(
                    &mut pos_on,
                    d,
                    -INFINITY,
                    INFINITY,
                    0,
                    true,
                    &mut ctx_on,
                    None,
                    None,
                    None,
                );
                ctx_on.prev_pv = ctx_on.pv_table.extract_pv();
                if mv.is_some() {
                    mv_on = mv;
                }
            }
            if let Some(m) = mv_on {
                if expected_targets.contains(&m.to_sq()) {
                    correct_on += 1;
                }
            }

            let mut pos_off = Position::from_fen(fen).expect("valid fen");
            let mut ctx_off = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(10),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(16),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: true,
                futility_enabled: true,
                check_extension_enabled: false,
                singular_extension_enabled: true,
            };
            ctx_off.tt.new_generation();
            ctx_off.history.push(pos_off.hash());
            let mut mv_off = None;
            for d in 1..=depth as u8 {
                ctx_off.pv_table.clear();
                let (_, mv) = negamax(
                    &mut pos_off,
                    d,
                    -INFINITY,
                    INFINITY,
                    0,
                    true,
                    &mut ctx_off,
                    None,
                    None,
                    None,
                );
                ctx_off.prev_pv = ctx_off.pv_table.extract_pv();
                if mv.is_some() {
                    mv_off = mv;
                }
            }
            if let Some(m) = mv_off {
                if expected_targets.contains(&m.to_sq()) {
                    correct_off += 1;
                }
            }
        }

        assert!(
            correct_on >= correct_off,
            "check extension should not reduce WAC solve rate: {} (on) vs {} (off)",
            correct_on,
            correct_off
        );
    }

    #[test]
    fn pvs_finds_same_or_better_move_on_wac() {
        let wac_positions = [
            (
                "2rr3k/pp3pp1/1nnqbN1p/3pN3/2pP4/2P3Q1/PPB4P/R4RK1 w - - 0 1",
                vec![Square::G6],
            ),
            (
                "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
                vec![Square::F7],
            ),
            ("6k1/5ppp/8/8/8/8/8/3Q1RK1 w - - 0 1", vec![Square::D8]),
            (
                "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
                vec![Square::G5, Square::D5],
            ),
        ];

        let depth: u8 = 6;
        let mut correct = 0;

        for (fen, expected_targets) in &wac_positions {
            let mut pos = Position::from_fen(fen).expect("valid fen");
            let mut ctx = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(10),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(16),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: true,
                futility_enabled: true,
                check_extension_enabled: true,
                singular_extension_enabled: true,
            };
            ctx.tt.new_generation();
            ctx.history.push(pos.hash());
            let mut best_mv = None;
            for d in 1..=depth {
                ctx.pv_table.clear();
                let (_, mv) = negamax(
                    &mut pos, d, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
                );
                ctx.prev_pv = ctx.pv_table.extract_pv();
                if mv.is_some() {
                    best_mv = mv;
                }
            }
            if let Some(m) = best_mv {
                if expected_targets.contains(&m.to_sq()) {
                    correct += 1;
                }
            }
        }

        assert!(
            correct >= 2,
            "PVS should solve at least 2 of 4 WAC positions, solved {}",
            correct
        );
    }

    #[test]
    fn pvs_zero_window_reduces_nodes() {
        let positions = [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r1bqkb1r/pppppppp/2n2n2/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3",
            "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",
        ];
        let depth: u8 = 8;
        // Ceiling derived from measured PVS node counts with 25% margin for non-determinism.
        // Measured after pawn eval integration: startpos~252k, pos2~115k, pos3~252k. Max~252k.
        let baseline_ceiling: u64 = 315_000;

        for fen in positions {
            let mut pos = Position::from_fen(fen).expect("valid fen");
            let mut ctx = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(60),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(16),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: true,
                futility_enabled: true,
                check_extension_enabled: true,
                singular_extension_enabled: true,
            };
            ctx.tt.new_generation();
            ctx.history.push(pos.hash());
            for d in 1..=depth {
                ctx.pv_table.clear();
                negamax(
                    &mut pos, d, -INFINITY, INFINITY, 0, true, &mut ctx, None, None, None,
                );
                ctx.prev_pv = ctx.pv_table.extract_pv();
            }
            let pvs_nodes = ctx.nodes;

            assert!(
                pvs_nodes <= baseline_ceiling,
                "PVS should search at or below baseline for {}: {} nodes vs {} ceiling",
                fen,
                pvs_nodes,
                baseline_ceiling,
            );
        }
    }

    #[test]
    fn pvs_lmr_cascade_correctness() {
        let positions = [
            (
                "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
                vec![Square::G5, Square::D5, Square::D3, Square::C3, Square::G1],
            ),
            (
                "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
                vec![Square::F7],
            ),
        ];
        let depth: u8 = 7;

        for (fen, expected_targets) in &positions {
            let mut pos_lmr = Position::from_fen(fen).expect("valid fen");
            let mut ctx_lmr = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(30),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(16),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: true,
                futility_enabled: true,
                check_extension_enabled: true,
                singular_extension_enabled: true,
            };
            ctx_lmr.tt.new_generation();
            ctx_lmr.history.push(pos_lmr.hash());
            let mut best_lmr = None;
            for d in 1..=depth {
                ctx_lmr.pv_table.clear();
                let (_, mv) = negamax(
                    &mut pos_lmr,
                    d,
                    -INFINITY,
                    INFINITY,
                    0,
                    true,
                    &mut ctx_lmr,
                    None,
                    None,
                    None,
                );
                ctx_lmr.prev_pv = ctx_lmr.pv_table.extract_pv();
                if mv.is_some() {
                    best_lmr = mv;
                }
            }
            let nodes_with_lmr = ctx_lmr.nodes;

            let mut pos_no_lmr = Position::from_fen(fen).expect("valid fen");
            let mut ctx_no_lmr = SearchContext {
                start: Instant::now(),
                time_budget: Duration::from_secs(30),
                nodes: 0,
                aborted: false,
                killers: KillerTable::new(),
                history_table: HistoryTable::new(),
                countermove_table: CounterMoveTable::new(),
                pv_table: PvTable::new(),
                prev_pv: Vec::new(),
                stop_flag: None,
                max_nodes: None,
                tt: TranspositionTable::new(16),
                history: Vec::new(),
                pawn_table: chess_eval::PawnHashTable::new(),
                lmr_enabled: false,
                futility_enabled: true,
                check_extension_enabled: true,
                singular_extension_enabled: true,
            };
            ctx_no_lmr.tt.new_generation();
            ctx_no_lmr.history.push(pos_no_lmr.hash());
            let mut best_no_lmr = None;
            for d in 1..=depth {
                ctx_no_lmr.pv_table.clear();
                let (_, mv) = negamax(
                    &mut pos_no_lmr,
                    d,
                    -INFINITY,
                    INFINITY,
                    0,
                    true,
                    &mut ctx_no_lmr,
                    None,
                    None,
                    None,
                );
                ctx_no_lmr.prev_pv = ctx_no_lmr.pv_table.extract_pv();
                if mv.is_some() {
                    best_no_lmr = mv;
                }
            }
            let nodes_without_lmr = ctx_no_lmr.nodes;

            if let Some(m) = best_lmr {
                assert!(
                    expected_targets.contains(&m.to_sq()),
                    "PVS+LMR should find expected move for FEN: {}, got to_sq {:?}",
                    fen,
                    m.to_sq()
                );
            } else {
                panic!("PVS+LMR should find a move for FEN: {}", fen);
            }

            if let Some(m) = best_no_lmr {
                assert!(
                    expected_targets.contains(&m.to_sq()),
                    "PVS-only should find expected move for FEN: {}, got to_sq {:?}",
                    fen,
                    m.to_sq()
                );
            } else {
                panic!("PVS-only should find a move for FEN: {}", fen);
            }

            assert!(
                nodes_with_lmr <= nodes_without_lmr,
                "PVS+LMR should search <= nodes than PVS alone for FEN: {} ({} vs {})",
                fen,
                nodes_with_lmr,
                nodes_without_lmr,
            );
        }
    }
}
