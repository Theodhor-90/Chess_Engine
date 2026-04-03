use chess_board::Position;
use chess_types::{Bitboard, Color, Piece, PieceKind};

const KING_CENTER_BONUS_WEIGHT: i32 = 10;
const KING_PASSED_PAWN_PROXIMITY_WEIGHT: i32 = 5;

const FILE_MASKS: [Bitboard; 8] = [
    Bitboard::new(0x0101_0101_0101_0101),
    Bitboard::new(0x0101_0101_0101_0101 << 1),
    Bitboard::new(0x0101_0101_0101_0101 << 2),
    Bitboard::new(0x0101_0101_0101_0101 << 3),
    Bitboard::new(0x0101_0101_0101_0101 << 4),
    Bitboard::new(0x0101_0101_0101_0101 << 5),
    Bitboard::new(0x0101_0101_0101_0101 << 6),
    Bitboard::new(0x0101_0101_0101_0101 << 7),
];

const ADJACENT_FILE_MASKS: [Bitboard; 8] = [
    Bitboard::new(0x0101_0101_0101_0101 << 1),
    Bitboard::new(0x0101_0101_0101_0101 | (0x0101_0101_0101_0101 << 2)),
    Bitboard::new((0x0101_0101_0101_0101 << 1) | (0x0101_0101_0101_0101 << 3)),
    Bitboard::new((0x0101_0101_0101_0101 << 2) | (0x0101_0101_0101_0101 << 4)),
    Bitboard::new((0x0101_0101_0101_0101 << 3) | (0x0101_0101_0101_0101 << 5)),
    Bitboard::new((0x0101_0101_0101_0101 << 4) | (0x0101_0101_0101_0101 << 6)),
    Bitboard::new((0x0101_0101_0101_0101 << 5) | (0x0101_0101_0101_0101 << 7)),
    Bitboard::new(0x0101_0101_0101_0101 << 6),
];

fn chebyshev_distance_to_center(sq_idx: u32) -> i32 {
    let file = (sq_idx % 8) as i32;
    let rank = (sq_idx / 8) as i32;
    let dist_d4 = (file - 3).abs().max((rank - 3).abs());
    let dist_d5 = (file - 3).abs().max((rank - 4).abs());
    let dist_e4 = (file - 4).abs().max((rank - 3).abs());
    let dist_e5 = (file - 4).abs().max((rank - 4).abs());
    dist_d4.min(dist_d5).min(dist_e4).min(dist_e5)
}

fn chebyshev_distance(sq1: u32, sq2: u32) -> i32 {
    let file1 = (sq1 % 8) as i32;
    let rank1 = (sq1 / 8) as i32;
    let file2 = (sq2 % 8) as i32;
    let rank2 = (sq2 / 8) as i32;
    (file1 - file2).abs().max((rank1 - rank2).abs())
}

fn is_pure_pawn_endgame(pos: &Position) -> bool {
    for kind in [
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
    ] {
        if !pos
            .piece_bitboard(Piece::new(Color::White, kind))
            .is_empty()
        {
            return false;
        }
        if !pos
            .piece_bitboard(Piece::new(Color::Black, kind))
            .is_empty()
        {
            return false;
        }
    }
    true
}

fn ranks_from(rank: u8) -> Bitboard {
    let mut mask = 0u64;
    let mut r = rank;
    while r <= 7 {
        mask |= 0xFFu64 << (r * 8);
        r += 1;
    }
    Bitboard::new(mask)
}

fn ranks_up_to(rank: u8) -> Bitboard {
    let mut mask = 0u64;
    let mut r = 0u8;
    while r <= rank {
        mask |= 0xFFu64 << (r * 8);
        r += 1;
    }
    Bitboard::new(mask)
}

fn is_passed_pawn(
    sq_idx: u32,
    color: Color,
    _friendly_pawns: Bitboard,
    enemy_pawns: Bitboard,
) -> bool {
    let file = sq_idx % 8;
    let rank = sq_idx / 8;
    let file_and_adj = FILE_MASKS[file as usize] | ADJACENT_FILE_MASKS[file as usize];
    let forward = match color {
        Color::White => file_and_adj & ranks_from(rank as u8 + 1),
        Color::Black => {
            if rank == 0 {
                return true;
            }
            file_and_adj & ranks_up_to(rank as u8 - 1)
        }
    };
    (enemy_pawns & forward).is_empty()
}

pub fn evaluate_king_centralization(pos: &Position) -> (i32, i32) {
    let white_king_sq = pos
        .piece_bitboard(Piece::new(Color::White, PieceKind::King))
        .into_iter()
        .next()
        .unwrap();
    let black_king_sq = pos
        .piece_bitboard(Piece::new(Color::Black, PieceKind::King))
        .into_iter()
        .next()
        .unwrap();

    let white_center_bonus =
        (3 - chebyshev_distance_to_center(white_king_sq)).max(0) * KING_CENTER_BONUS_WEIGHT;
    let black_center_bonus =
        (3 - chebyshev_distance_to_center(black_king_sq)).max(0) * KING_CENTER_BONUS_WEIGHT;

    let mut eg = white_center_bonus - black_center_bonus;

    if is_pure_pawn_endgame(pos) {
        let white_pawns = pos.piece_bitboard(Piece::new(Color::White, PieceKind::Pawn));
        let black_pawns = pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Pawn));

        for sq_idx in white_pawns {
            if is_passed_pawn(sq_idx, Color::White, white_pawns, black_pawns) {
                let dist = chebyshev_distance(white_king_sq, sq_idx);
                eg += (7 - dist) * KING_PASSED_PAWN_PROXIMITY_WEIGHT;
            }
        }

        for sq_idx in black_pawns {
            if is_passed_pawn(sq_idx, Color::Black, black_pawns, white_pawns) {
                let dist = chebyshev_distance(black_king_sq, sq_idx);
                eg -= (7 - dist) * KING_PASSED_PAWN_PROXIMITY_WEIGHT;
            }
        }
    }

    (0, eg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centralized_king_scores_higher_than_corner_king() {
        let centralized = Position::from_fen("4k3/8/8/8/4K3/8/8/8 w - - 0 1").unwrap();
        let corner = Position::from_fen("4k3/8/8/8/8/8/8/K7 w - - 0 1").unwrap();
        let (cen_mg, cen_eg) = evaluate_king_centralization(&centralized);
        let (cor_mg, cor_eg) = evaluate_king_centralization(&corner);
        assert_eq!(cen_mg, 0);
        assert_eq!(cor_mg, 0);
        assert!(
            cen_eg > cor_eg,
            "centralized eg {cen_eg} should exceed corner eg {cor_eg}"
        );
    }

    #[test]
    fn king_proximity_to_passed_pawn_rewarded() {
        let close = Position::from_fen("8/8/8/3PK3/8/8/8/7k w - - 0 1").unwrap();
        let far = Position::from_fen("8/8/8/3P4/8/8/8/K6k w - - 0 1").unwrap();
        let (_, close_eg) = evaluate_king_centralization(&close);
        let (_, far_eg) = evaluate_king_centralization(&far);
        assert!(
            close_eg > far_eg,
            "close king eg {close_eg} should exceed far king eg {far_eg}"
        );
    }
}
