use chess_board::Position;
use chess_movegen::king::KING_ATTACKS;
use chess_movegen::knights::KNIGHT_ATTACKS;
use chess_movegen::magic::{bishop_attacks, rook_attacks};
use chess_types::{Bitboard, Color, Piece, PieceKind, Square};

const PAWN_SHIELD_BONUS_MG: [i32; 4] = [18, 10, 4, 0];
const PAWN_SHIELD_BONUS_EG: [i32; 4] = [12, 6, 2, 0];

const OPEN_FILE_NEAR_KING_PENALTY_MG: i32 = -18;
const OPEN_FILE_NEAR_KING_PENALTY_EG: i32 = -12;

const KNIGHT_ATTACK_WEIGHT: i32 = 2;
const BISHOP_ATTACK_WEIGHT: i32 = 2;
const ROOK_ATTACK_WEIGHT: i32 = 3;
const QUEEN_ATTACK_WEIGHT: i32 = 5;

const ATTACK_SCORE_SCALE_MG: i32 = 1;
const ATTACK_SCORE_SCALE_EG: i32 = 1;

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

const NOT_A_FILE: Bitboard = Bitboard::new(0xFEFE_FEFE_FEFE_FEFE);
const NOT_H_FILE: Bitboard = Bitboard::new(0x7F7F_7F7F_7F7F_7F7F);

fn king_zone(king_sq: u32, color: Color) -> Bitboard {
    let surrounding = KING_ATTACKS[king_sq as usize];
    let king_bb = Bitboard::new(1u64 << king_sq);
    let extended = match color {
        Color::White => {
            let front2 = king_bb << 16;
            front2 | ((front2 & NOT_A_FILE) >> 1) | ((front2 & NOT_H_FILE) << 1)
        }
        Color::Black => {
            let front2 = king_bb >> 16;
            front2 | ((front2 & NOT_A_FILE) >> 1) | ((front2 & NOT_H_FILE) << 1)
        }
    };
    surrounding | extended
}

pub fn evaluate_king_safety(pos: &Position, color: Color) -> (i32, i32) {
    let king_bb = pos.piece_bitboard(Piece::new(color, PieceKind::King));
    let king_sq = match king_bb.into_iter().next() {
        Some(sq) => sq,
        None => return (0, 0),
    };
    let king_file = king_sq % 8;
    let king_rank = king_sq / 8;

    let file_min = if king_file > 0 { king_file - 1 } else { 0 };
    let file_max = if king_file < 7 { king_file + 1 } else { 7 };

    let friendly_pawns = pos.piece_bitboard(Piece::new(color, PieceKind::Pawn));

    let mut mg = 0i32;
    let mut eg = 0i32;

    for f in file_min..=file_max {
        let file_pawns = friendly_pawns & FILE_MASKS[f as usize];
        if file_pawns.is_empty() {
            mg += OPEN_FILE_NEAR_KING_PENALTY_MG;
            eg += OPEN_FILE_NEAR_KING_PENALTY_EG;
        } else {
            let closest_rank = match color {
                Color::White => {
                    let mut best = 0u32;
                    for sq_idx in file_pawns {
                        let r = sq_idx / 8;
                        if r > king_rank && (best == 0 || r < best) {
                            best = r;
                        }
                    }
                    best
                }
                Color::Black => {
                    let mut best = 7u32;
                    for sq_idx in file_pawns {
                        let r = sq_idx / 8;
                        if r < king_rank && (best == 7 || r > best) {
                            best = r;
                        }
                    }
                    best
                }
            };
            let dist = closest_rank.abs_diff(king_rank);
            let idx = if dist >= 4 { 3 } else { dist as usize };
            mg += PAWN_SHIELD_BONUS_MG[idx];
            eg += PAWN_SHIELD_BONUS_EG[idx];
        }
    }

    let zone = king_zone(king_sq, color);
    let opp = color.opposite();
    let occupied = pos.occupied();
    let mut total_attack = 0i32;

    let opp_knights = pos.piece_bitboard(Piece::new(opp, PieceKind::Knight));
    for sq_idx in opp_knights {
        let attacks = KNIGHT_ATTACKS[sq_idx as usize];
        total_attack += (attacks & zone).pop_count() as i32 * KNIGHT_ATTACK_WEIGHT;
    }

    let opp_bishops = pos.piece_bitboard(Piece::new(opp, PieceKind::Bishop));
    for sq_idx in opp_bishops {
        let sq = Square::new(sq_idx as u8).unwrap();
        let attacks = bishop_attacks(sq, occupied);
        total_attack += (attacks & zone).pop_count() as i32 * BISHOP_ATTACK_WEIGHT;
    }

    let opp_rooks = pos.piece_bitboard(Piece::new(opp, PieceKind::Rook));
    for sq_idx in opp_rooks {
        let sq = Square::new(sq_idx as u8).unwrap();
        let attacks = rook_attacks(sq, occupied);
        total_attack += (attacks & zone).pop_count() as i32 * ROOK_ATTACK_WEIGHT;
    }

    let opp_queens = pos.piece_bitboard(Piece::new(opp, PieceKind::Queen));
    for sq_idx in opp_queens {
        let sq = Square::new(sq_idx as u8).unwrap();
        let b_attacks = bishop_attacks(sq, occupied);
        let r_attacks = rook_attacks(sq, occupied);
        total_attack += ((b_attacks | r_attacks) & zone).pop_count() as i32 * QUEEN_ATTACK_WEIGHT;
    }

    mg -= total_attack * ATTACK_SCORE_SCALE_MG;
    eg -= total_attack * ATTACK_SCORE_SCALE_EG;

    (mg, eg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strong_shield_scores_higher_than_weak() {
        let strong = Position::from_fen("4k3/8/8/8/8/8/5PPP/6K1 w - - 0 1").unwrap();
        let weak = Position::from_fen("4k3/8/8/8/8/5P2/7P/6K1 w - - 0 1").unwrap();
        let (strong_mg, _) = evaluate_king_safety(&strong, Color::White);
        let (weak_mg, _) = evaluate_king_safety(&weak, Color::White);
        assert!(
            strong_mg > weak_mg,
            "strong shield mg {strong_mg} should exceed weak shield mg {weak_mg}"
        );
    }

    #[test]
    fn open_file_near_king_penalized() {
        let open = Position::from_fen("4k3/8/8/8/8/8/P4P2/4K3 w - - 0 1").unwrap();
        let closed = Position::from_fen("4k3/8/8/8/8/8/3PPP2/4K3 w - - 0 1").unwrap();
        let (open_mg, _) = evaluate_king_safety(&open, Color::White);
        let (closed_mg, _) = evaluate_king_safety(&closed, Color::White);
        assert!(
            open_mg < closed_mg,
            "open file mg {open_mg} should be less than closed file mg {closed_mg}"
        );
    }

    #[test]
    fn attacking_pieces_toward_king_zone() {
        let pressure = Position::from_fen("4k3/8/8/8/8/8/5PPP/4r1K1 w - - 0 1").unwrap();
        let no_pressure = Position::from_fen("4k3/r7/8/8/8/8/5PPP/6K1 w - - 0 1").unwrap();
        let (pressure_mg, _) = evaluate_king_safety(&pressure, Color::White);
        let (no_pressure_mg, _) = evaluate_king_safety(&no_pressure, Color::White);
        assert!(
            pressure_mg < no_pressure_mg,
            "pressure mg {pressure_mg} should be less than no_pressure mg {no_pressure_mg}"
        );
    }

    #[test]
    fn midgame_shield_bonus_exceeds_endgame() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/5PPP/6K1 w - - 0 1").unwrap();
        let (mg, eg) = evaluate_king_safety(&pos, Color::White);
        assert!(
            mg > eg,
            "midgame shield bonus {mg} should exceed endgame shield bonus {eg}"
        );
    }
}
