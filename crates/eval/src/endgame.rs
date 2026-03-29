use chess_board::Position;
use chess_types::{Color, Piece, PieceKind, Square};

const KNOWN_WIN: i32 = 10000;
const CORNER_WEIGHT: i32 = 20;
const KING_PROXIMITY_WEIGHT: i32 = 10;

fn chebyshev_distance(sq1: Square, sq2: Square) -> i32 {
    let file_diff = (sq1.file() as i32 - sq2.file() as i32).abs();
    let rank_diff = (sq1.rank() as i32 - sq2.rank() as i32).abs();
    file_diff.max(rank_diff)
}

fn edge_distance(sq: Square) -> i32 {
    let file = sq.file() as i32;
    let rank = sq.rank() as i32;
    file.min(7 - file).min(rank).min(7 - rank)
}

fn king_square(pos: &Position, color: Color) -> Square {
    let bb = pos.piece_bitboard(Piece::new(color, PieceKind::King));
    let idx = bb.lsb().expect("king must exist");
    Square::new(idx as u8).expect("valid square")
}

fn eval_kr_vs_k(pos: &Position, strong_side: Color) -> i32 {
    let weak_side = strong_side.opposite();
    let losing_king = king_square(pos, weak_side);
    let winning_king = king_square(pos, strong_side);

    let edge_bonus = (3 - edge_distance(losing_king)) * 20;
    let proximity_bonus = (7 - chebyshev_distance(winning_king, losing_king)) * 10;

    let score = KNOWN_WIN + edge_bonus + proximity_bonus;
    if strong_side == Color::White {
        score
    } else {
        -score
    }
}

fn eval_kq_vs_k(pos: &Position, strong_side: Color) -> i32 {
    let weak_side = strong_side.opposite();
    let losing_king = king_square(pos, weak_side);
    let winning_king = king_square(pos, strong_side);

    let edge_bonus = (3 - edge_distance(losing_king)) * 20;
    let proximity_bonus = (7 - chebyshev_distance(winning_king, losing_king)) * 10;

    let score = KNOWN_WIN + edge_bonus + proximity_bonus;
    if strong_side == Color::White {
        score
    } else {
        -score
    }
}

fn eval_kbn_vs_k(pos: &Position, strong_side: Color) -> i32 {
    let weak_side = strong_side.opposite();
    let losing_king = king_square(pos, weak_side);
    let winning_king = king_square(pos, strong_side);

    let bishop_bb = pos.piece_bitboard(Piece::new(strong_side, PieceKind::Bishop));
    let bishop_idx = bishop_bb.lsb().expect("bishop must exist");
    let bishop_sq = Square::new(bishop_idx as u8).expect("valid square");

    let bishop_color = (bishop_sq.file() as u8 + bishop_sq.rank() as u8) % 2;

    let (corner_a, corner_b) = if bishop_color == 0 {
        // Dark-square bishop: correct corners are a1 (0) and h8 (63)
        (
            Square::new(0).unwrap(),  // a1
            Square::new(63).unwrap(), // h8
        )
    } else {
        // Light-square bishop: correct corners are a8 (56) and h1 (7)
        (
            Square::new(56).unwrap(), // a8
            Square::new(7).unwrap(),  // h1
        )
    };

    let dist_a = chebyshev_distance(losing_king, corner_a);
    let dist_b = chebyshev_distance(losing_king, corner_b);
    let min_corner_dist = dist_a.min(dist_b);

    let corner_bonus = (7 - min_corner_dist) * CORNER_WEIGHT;
    let proximity_bonus =
        (7 - chebyshev_distance(winning_king, losing_king)) * KING_PROXIMITY_WEIGHT;

    let score = KNOWN_WIN + corner_bonus + proximity_bonus;
    if strong_side == Color::White {
        score
    } else {
        -score
    }
}

struct PieceCounts {
    pawns: u32,
    knights: u32,
    bishops: u32,
    rooks: u32,
    queens: u32,
}

impl PieceCounts {
    fn total(&self) -> u32 {
        self.pawns + self.knights + self.bishops + self.rooks + self.queens
    }
}

fn count_pieces(pos: &Position, color: Color) -> PieceCounts {
    PieceCounts {
        pawns: pos
            .piece_bitboard(Piece::new(color, PieceKind::Pawn))
            .pop_count(),
        knights: pos
            .piece_bitboard(Piece::new(color, PieceKind::Knight))
            .pop_count(),
        bishops: pos
            .piece_bitboard(Piece::new(color, PieceKind::Bishop))
            .pop_count(),
        rooks: pos
            .piece_bitboard(Piece::new(color, PieceKind::Rook))
            .pop_count(),
        queens: pos
            .piece_bitboard(Piece::new(color, PieceKind::Queen))
            .pop_count(),
    }
}

pub fn probe_endgame(pos: &Position) -> Option<i32> {
    let white = count_pieces(pos, Color::White);
    let black = count_pieces(pos, Color::Black);

    let w_total = white.total();
    let b_total = black.total();

    // K vs K
    if w_total == 0 && b_total == 0 {
        return Some(0);
    }

    // KB vs K
    if w_total == 1 && white.bishops == 1 && b_total == 0 {
        return Some(0);
    }
    if b_total == 1 && black.bishops == 1 && w_total == 0 {
        return Some(0);
    }

    // KN vs K
    if w_total == 1 && white.knights == 1 && b_total == 0 {
        return Some(0);
    }
    if b_total == 1 && black.knights == 1 && w_total == 0 {
        return Some(0);
    }

    // KNN vs K
    if w_total == 2 && white.knights == 2 && b_total == 0 {
        return Some(0);
    }
    if b_total == 2 && black.knights == 2 && w_total == 0 {
        return Some(0);
    }

    // KQ vs K
    if w_total == 1 && white.queens == 1 && b_total == 0 {
        return Some(eval_kq_vs_k(pos, Color::White));
    }
    if b_total == 1 && black.queens == 1 && w_total == 0 {
        return Some(eval_kq_vs_k(pos, Color::Black));
    }

    // KR vs K
    if w_total == 1 && white.rooks == 1 && b_total == 0 {
        return Some(eval_kr_vs_k(pos, Color::White));
    }
    if b_total == 1 && black.rooks == 1 && w_total == 0 {
        return Some(eval_kr_vs_k(pos, Color::Black));
    }

    // KBN vs K
    if w_total == 2 && white.bishops == 1 && white.knights == 1 && b_total == 0 {
        return Some(eval_kbn_vs_k(pos, Color::White));
    }
    if b_total == 2 && black.bishops == 1 && black.knights == 1 && w_total == 0 {
        return Some(eval_kbn_vs_k(pos, Color::Black));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_board::Position;

    #[test]
    fn kbn_vs_k_white_wins_dark_bishop() {
        // White: Ke1, Bc1 (dark square), Nb1; Black: Ke8
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/1NB1K3 w - - 0 1").unwrap();
        let score = probe_endgame(&pos);
        assert!(score.is_some());
        assert!(score.unwrap() > 0);
    }

    #[test]
    fn kbn_vs_k_black_wins_light_bishop() {
        // Black: Ke8, Bf8 (light square), Ng8; White: Ke1
        let pos = Position::from_fen("4kbn1/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let score = probe_endgame(&pos);
        assert!(score.is_some());
        assert!(score.unwrap() < 0);
    }

    #[test]
    fn kbn_vs_k_corner_drives_correct() {
        // Dark-square bishop: correct corners are a1 and h8
        // White: Ke2, Bc1 (dark), Nb1; Black king near correct corner a1 vs center
        let near_corner = Position::from_fen("8/8/8/8/8/8/4K3/kNB5 w - - 0 1").unwrap();
        let in_center = Position::from_fen("8/8/8/3k4/8/8/4K3/1NB5 w - - 0 1").unwrap();
        let score_corner = probe_endgame(&near_corner).unwrap();
        let score_center = probe_endgame(&in_center).unwrap();
        assert!(
            score_corner > score_center,
            "corner score {score_corner} should exceed center score {score_center}"
        );
    }

    #[test]
    fn kr_vs_k_white_wins() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/4K2R w - - 0 1").unwrap();
        let score = probe_endgame(&pos);
        assert!(score.is_some());
        assert!(score.unwrap() > 0);
    }

    #[test]
    fn kr_vs_k_edge_better() {
        // Losing king on edge vs center
        let on_edge = Position::from_fen("k7/8/8/8/8/8/8/4K2R w - - 0 1").unwrap();
        let in_center = Position::from_fen("8/8/8/3k4/8/8/8/4K2R w - - 0 1").unwrap();
        let score_edge = probe_endgame(&on_edge).unwrap();
        let score_center = probe_endgame(&in_center).unwrap();
        assert!(
            score_edge > score_center,
            "edge score {score_edge} should exceed center score {score_center}"
        );
    }

    #[test]
    fn kq_vs_k_white_wins() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/3QK3 w - - 0 1").unwrap();
        let score = probe_endgame(&pos);
        assert!(score.is_some());
        assert!(score.unwrap() > 0);
    }

    #[test]
    fn k_vs_k_draw() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert_eq!(probe_endgame(&pos), Some(0));
    }

    #[test]
    fn kb_vs_k_draw() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/4KB2 w - - 0 1").unwrap();
        assert_eq!(probe_endgame(&pos), Some(0));
    }

    #[test]
    fn kn_vs_k_draw() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/4KN2 w - - 0 1").unwrap();
        assert_eq!(probe_endgame(&pos), Some(0));
    }

    #[test]
    fn knn_vs_k_draw() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/3NKN2 w - - 0 1").unwrap();
        assert_eq!(probe_endgame(&pos), Some(0));
    }

    #[test]
    fn non_endgame_returns_none() {
        let pos =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(probe_endgame(&pos), None);
    }
}
