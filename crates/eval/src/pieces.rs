use chess_board::Position;
use chess_types::{Bitboard, Color, Piece, PieceKind};

const BISHOP_PAIR_BONUS_MG: i32 = 30;
const BISHOP_PAIR_BONUS_EG: i32 = 50;

const ROOK_OPEN_FILE_BONUS_MG: i32 = 20;
const ROOK_OPEN_FILE_BONUS_EG: i32 = 10;

const ROOK_SEMI_OPEN_FILE_BONUS_MG: i32 = 10;
const ROOK_SEMI_OPEN_FILE_BONUS_EG: i32 = 5;

const ROOK_SEVENTH_RANK_BONUS_MG: i32 = 20;
const ROOK_SEVENTH_RANK_BONUS_EG: i32 = 30;

const KNIGHT_OUTPOST_BONUS_MG: i32 = 20;
const KNIGHT_OUTPOST_BONUS_EG: i32 = 15;

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

pub fn evaluate_piece_bonuses(pos: &Position, color: Color) -> (i32, i32) {
    let mut mg = 0;
    let mut eg = 0;

    let friendly_pawns = pos.piece_bitboard(Piece::new(color, PieceKind::Pawn));
    let enemy_pawns = pos.piece_bitboard(Piece::new(color.opposite(), PieceKind::Pawn));

    let bishops = pos.piece_bitboard(Piece::new(color, PieceKind::Bishop));
    if bishops.pop_count() >= 2 {
        mg += BISHOP_PAIR_BONUS_MG;
        eg += BISHOP_PAIR_BONUS_EG;
    }

    let rooks = pos.piece_bitboard(Piece::new(color, PieceKind::Rook));
    for sq_idx in rooks {
        let file = (sq_idx % 8) as usize;
        let rank = sq_idx / 8;
        let file_mask = FILE_MASKS[file];

        if (friendly_pawns & file_mask).is_empty() && (enemy_pawns & file_mask).is_empty() {
            mg += ROOK_OPEN_FILE_BONUS_MG;
            eg += ROOK_OPEN_FILE_BONUS_EG;
        } else if (friendly_pawns & file_mask).is_empty() {
            mg += ROOK_SEMI_OPEN_FILE_BONUS_MG;
            eg += ROOK_SEMI_OPEN_FILE_BONUS_EG;
        }

        let seventh = match color {
            Color::White => 6,
            Color::Black => 1,
        };
        if rank == seventh {
            mg += ROOK_SEVENTH_RANK_BONUS_MG;
            eg += ROOK_SEVENTH_RANK_BONUS_EG;
        }
    }

    let knights = pos.piece_bitboard(Piece::new(color, PieceKind::Knight));
    for sq_idx in knights {
        let file = sq_idx % 8;
        let rank = sq_idx / 8;

        let on_outpost_rank = match color {
            Color::White => rank == 4 || rank == 5,
            Color::Black => rank == 2 || rank == 3,
        };

        if !on_outpost_rank {
            continue;
        }

        let pawn_support = match color {
            Color::White => {
                let left = if file > 0 {
                    !(Bitboard::new(1u64 << (sq_idx - 9)) & friendly_pawns).is_empty()
                } else {
                    false
                };
                let right = if file < 7 {
                    !(Bitboard::new(1u64 << (sq_idx - 7)) & friendly_pawns).is_empty()
                } else {
                    false
                };
                left || right
            }
            Color::Black => {
                let left = if file > 0 {
                    !(Bitboard::new(1u64 << (sq_idx + 7)) & friendly_pawns).is_empty()
                } else {
                    false
                };
                let right = if file < 7 {
                    !(Bitboard::new(1u64 << (sq_idx + 9)) & friendly_pawns).is_empty()
                } else {
                    false
                };
                left || right
            }
        };

        if !pawn_support {
            continue;
        }

        let no_enemy_pawn_threat = match color {
            Color::White => {
                let mut safe = true;
                let adj_files = adj_file_mask(file);
                for r in (rank + 1)..8 {
                    let rank_mask = Bitboard::new(0xFFu64 << (r * 8));
                    if !(enemy_pawns & adj_files & rank_mask).is_empty() {
                        safe = false;
                        break;
                    }
                }
                safe
            }
            Color::Black => {
                let mut safe = true;
                let adj_files = adj_file_mask(file);
                for r in 0..rank {
                    let rank_mask = Bitboard::new(0xFFu64 << (r * 8));
                    if !(enemy_pawns & adj_files & rank_mask).is_empty() {
                        safe = false;
                        break;
                    }
                }
                safe
            }
        };

        if no_enemy_pawn_threat {
            mg += KNIGHT_OUTPOST_BONUS_MG;
            eg += KNIGHT_OUTPOST_BONUS_EG;
        }
    }

    (mg, eg)
}

fn adj_file_mask(file: u32) -> Bitboard {
    let mut mask = Bitboard::new(0);
    if file > 0 {
        mask |= FILE_MASKS[(file - 1) as usize];
    }
    if file < 7 {
        mask |= FILE_MASKS[(file + 1) as usize];
    }
    mask
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_board::Position;

    #[test]
    fn bishop_pair_bonus_awarded() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/2B1KB2 w - - 0 1").unwrap();
        let (mg, eg) = evaluate_piece_bonuses(&pos, Color::White);
        assert!(
            mg >= BISHOP_PAIR_BONUS_MG,
            "mg {mg} should be >= {BISHOP_PAIR_BONUS_MG}"
        );
        assert!(
            eg >= BISHOP_PAIR_BONUS_EG,
            "eg {eg} should be >= {BISHOP_PAIR_BONUS_EG}"
        );
    }

    #[test]
    fn single_bishop_no_pair_bonus() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/2B1K3 w - - 0 1").unwrap();
        let (mg, eg) = evaluate_piece_bonuses(&pos, Color::White);
        assert_eq!(mg, 0);
        assert_eq!(eg, 0);
    }

    #[test]
    fn rook_on_open_file() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/PP3PP1/4K2R w - - 0 1").unwrap();
        let (mg, _eg) = evaluate_piece_bonuses(&pos, Color::White);
        assert!(
            mg >= ROOK_OPEN_FILE_BONUS_MG,
            "mg {mg} should be >= {ROOK_OPEN_FILE_BONUS_MG}"
        );
    }

    #[test]
    fn rook_on_semi_open_file() {
        let pos = Position::from_fen("4k3/7p/8/8/8/8/PP3PP1/4K2R w - - 0 1").unwrap();
        let (mg, _eg) = evaluate_piece_bonuses(&pos, Color::White);
        assert!(
            mg >= ROOK_SEMI_OPEN_FILE_BONUS_MG,
            "mg {mg} should be >= {ROOK_SEMI_OPEN_FILE_BONUS_MG}"
        );
        assert!(
            mg < ROOK_OPEN_FILE_BONUS_MG,
            "semi-open mg {mg} should be < open file bonus {ROOK_OPEN_FILE_BONUS_MG}"
        );
    }

    #[test]
    fn rook_on_seventh_rank() {
        let pos = Position::from_fen("4k3/R7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let (mg, eg) = evaluate_piece_bonuses(&pos, Color::White);
        assert!(
            mg >= ROOK_SEVENTH_RANK_BONUS_MG,
            "mg {mg} should be >= {ROOK_SEVENTH_RANK_BONUS_MG}"
        );
        assert!(
            eg >= ROOK_SEVENTH_RANK_BONUS_EG,
            "eg {eg} should be >= {ROOK_SEVENTH_RANK_BONUS_EG}"
        );
    }

    #[test]
    fn knight_outpost_awarded() {
        let pos = Position::from_fen("4k3/8/8/4N3/3P4/8/8/4K3 w - - 0 1").unwrap();
        let (mg, eg) = evaluate_piece_bonuses(&pos, Color::White);
        assert!(
            mg >= KNIGHT_OUTPOST_BONUS_MG,
            "mg {mg} should be >= {KNIGHT_OUTPOST_BONUS_MG}"
        );
        assert!(
            eg >= KNIGHT_OUTPOST_BONUS_EG,
            "eg {eg} should be >= {KNIGHT_OUTPOST_BONUS_EG}"
        );
    }

    #[test]
    fn knight_without_pawn_support_no_outpost() {
        let pos = Position::from_fen("4k3/8/8/4N3/8/8/8/4K3 w - - 0 1").unwrap();
        let (mg, eg) = evaluate_piece_bonuses(&pos, Color::White);
        assert_eq!(mg, 0);
        assert_eq!(eg, 0);
    }

    #[test]
    fn all_bonuses_produce_midgame_and_endgame() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/R1B1KB2 w - - 0 1").unwrap();
        let (mg, eg) = evaluate_piece_bonuses(&pos, Color::White);
        assert!(mg > BISHOP_PAIR_BONUS_MG, "mg {mg} should combine bonuses");
        assert!(eg > BISHOP_PAIR_BONUS_EG, "eg {eg} should combine bonuses");
    }
}
