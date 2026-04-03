use chess_board::zobrist::piece_square_key;
use chess_board::Position;
use chess_types::{Bitboard, Color, Piece, PieceKind, Square};

const DOUBLED_PAWN_PENALTY_MG: i32 = -8;
const DOUBLED_PAWN_PENALTY_EG: i32 = -22;
const ISOLATED_PAWN_PENALTY_MG: i32 = -12;
const ISOLATED_PAWN_PENALTY_EG: i32 = -18;
const BACKWARD_PAWN_PENALTY_MG: i32 = -8;
const BACKWARD_PAWN_PENALTY_EG: i32 = -13;
const PASSED_PAWN_BONUS_MG: [i32; 8] = [0, 0, 7, 12, 23, 38, 65, 0];
const PASSED_PAWN_BONUS_EG: [i32; 8] = [0, 0, 13, 25, 47, 78, 131, 0];
const CONNECTED_PAWN_BONUS_MG: i32 = 7;
const CONNECTED_PAWN_BONUS_EG: i32 = 9;

const KING_PROXIMITY_OWN_BONUS: i32 = 6;
const KING_PROXIMITY_OPP_BONUS: i32 = 4;
const BLOCKED_PASSED_PAWN_PENALTY_MG: i32 = -7;
const BLOCKED_PASSED_PAWN_PENALTY_EG: i32 = -12;

const NOT_A_FILE: Bitboard = Bitboard::new(0xFEFE_FEFE_FEFE_FEFE);
const NOT_H_FILE: Bitboard = Bitboard::new(0x7F7F_7F7F_7F7F_7F7F);

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
    Bitboard::new(0x0101_0101_0101_0101 << 1), // A -> B
    Bitboard::new(0x0101_0101_0101_0101 | (0x0101_0101_0101_0101 << 2)), // B -> A|C
    Bitboard::new((0x0101_0101_0101_0101 << 1) | (0x0101_0101_0101_0101 << 3)), // C -> B|D
    Bitboard::new((0x0101_0101_0101_0101 << 2) | (0x0101_0101_0101_0101 << 4)), // D -> C|E
    Bitboard::new((0x0101_0101_0101_0101 << 3) | (0x0101_0101_0101_0101 << 5)), // E -> D|F
    Bitboard::new((0x0101_0101_0101_0101 << 4) | (0x0101_0101_0101_0101 << 6)), // F -> E|G
    Bitboard::new((0x0101_0101_0101_0101 << 5) | (0x0101_0101_0101_0101 << 7)), // G -> F|H
    Bitboard::new(0x0101_0101_0101_0101 << 6), // H -> G
];

const fn rank_bb(rank: u8) -> Bitboard {
    Bitboard::new(0xFFu64 << (rank * 8))
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

fn ranks_from(rank: u8) -> Bitboard {
    let mut mask = 0u64;
    let mut r = rank;
    while r <= 7 {
        mask |= 0xFFu64 << (r * 8);
        r += 1;
    }
    Bitboard::new(mask)
}

fn forward_mask(file: u8, rank: u8, color: Color) -> Bitboard {
    let file_and_adj = FILE_MASKS[file as usize] | ADJACENT_FILE_MASKS[file as usize];
    match color {
        Color::White => file_and_adj & ranks_from(rank + 1),
        Color::Black => {
            if rank == 0 {
                return Bitboard::EMPTY;
            }
            file_and_adj & ranks_up_to(rank - 1)
        }
    }
}

fn evaluate_color(friendly_pawns: Bitboard, enemy_pawns: Bitboard, color: Color) -> (i32, i32) {
    let mut mg = 0i32;
    let mut eg = 0i32;

    let mut doubled_counted = [false; 8];

    let pawn_attacks = match color {
        Color::White => ((friendly_pawns & NOT_A_FILE) << 7) | ((friendly_pawns & NOT_H_FILE) << 9),
        Color::Black => ((friendly_pawns & NOT_H_FILE) >> 7) | ((friendly_pawns & NOT_A_FILE) >> 9),
    };

    for sq_idx in friendly_pawns {
        let file = sq_idx % 8;
        let rank = sq_idx / 8;

        if !doubled_counted[file as usize] {
            let count = (friendly_pawns & FILE_MASKS[file as usize]).pop_count();
            if count > 1 {
                mg += (count as i32 - 1) * DOUBLED_PAWN_PENALTY_MG;
                eg += (count as i32 - 1) * DOUBLED_PAWN_PENALTY_EG;
            }
            doubled_counted[file as usize] = true;
        }

        let isolated = (friendly_pawns & ADJACENT_FILE_MASKS[file as usize]).is_empty();

        if isolated {
            mg += ISOLATED_PAWN_PENALTY_MG;
            eg += ISOLATED_PAWN_PENALTY_EG;
        }

        let passed_mask = forward_mask(file as u8, rank as u8, color);
        let is_passed = (enemy_pawns & passed_mask).is_empty();

        if is_passed {
            let bonus_rank = match color {
                Color::White => rank,
                Color::Black => 7 - rank,
            };
            mg += PASSED_PAWN_BONUS_MG[bonus_rank as usize];
            eg += PASSED_PAWN_BONUS_EG[bonus_rank as usize];
        }

        if !isolated && !is_passed {
            let support_mask = ADJACENT_FILE_MASKS[file as usize] & ranks_up_to(rank as u8);
            let has_support = !(friendly_pawns & support_mask).is_empty();

            if !has_support {
                let advance_attacked = match color {
                    Color::White => {
                        if rank < 7 {
                            let advance_rank = rank + 1;
                            let enemy_attacker_mask = ADJACENT_FILE_MASKS[file as usize]
                                & rank_bb(advance_rank as u8 + 1);
                            if advance_rank < 7 {
                                !(enemy_pawns & enemy_attacker_mask).is_empty()
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                    Color::Black => {
                        if rank > 0 {
                            let advance_rank = rank - 1;
                            if advance_rank > 0 {
                                let enemy_attacker_mask = ADJACENT_FILE_MASKS[file as usize]
                                    & rank_bb(advance_rank as u8 - 1);
                                !(enemy_pawns & enemy_attacker_mask).is_empty()
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                };

                if advance_attacked {
                    mg += BACKWARD_PAWN_PENALTY_MG;
                    eg += BACKWARD_PAWN_PENALTY_EG;
                }
            }
        }

        let sq_bb = Bitboard::new(1u64 << sq_idx);
        if !(sq_bb & pawn_attacks).is_empty() {
            mg += CONNECTED_PAWN_BONUS_MG;
            eg += CONNECTED_PAWN_BONUS_EG;
        }
    }

    (mg, eg)
}

pub fn evaluate_pawns(pos: &Position) -> (i32, i32) {
    let white_pawns = pos.piece_bitboard(Piece::new(Color::White, PieceKind::Pawn));
    let black_pawns = pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Pawn));

    let (white_mg, white_eg) = evaluate_color(white_pawns, black_pawns, Color::White);
    let (black_mg, black_eg) = evaluate_color(black_pawns, white_pawns, Color::Black);

    (white_mg - black_mg, white_eg - black_eg)
}

fn chebyshev_distance(sq1: u32, sq2: u32) -> i32 {
    let file1 = (sq1 % 8) as i32;
    let rank1 = (sq1 / 8) as i32;
    let file2 = (sq2 % 8) as i32;
    let rank2 = (sq2 / 8) as i32;
    (file1 - file2).abs().max((rank1 - rank2).abs())
}

pub fn evaluate_passed_pawn_extras(pos: &Position) -> (i32, i32) {
    let white_pawns = pos.piece_bitboard(Piece::new(Color::White, PieceKind::Pawn));
    let black_pawns = pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Pawn));
    let occupied = pos.occupied();

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

    let mut mg = 0i32;
    let mut eg = 0i32;

    for sq_idx in white_pawns {
        let file = sq_idx % 8;
        let rank = sq_idx / 8;
        let passed_mask = forward_mask(file as u8, rank as u8, Color::White);
        if !(black_pawns & passed_mask).is_empty() {
            continue;
        }
        let own_dist = chebyshev_distance(white_king_sq, sq_idx);
        let opp_dist = chebyshev_distance(black_king_sq, sq_idx);
        eg += KING_PROXIMITY_OWN_BONUS * (7 - own_dist);
        eg += KING_PROXIMITY_OPP_BONUS * opp_dist;
        if rank < 7 {
            let advance_sq = sq_idx + 8;
            if !(occupied & Bitboard::new(1u64 << advance_sq)).is_empty() {
                mg += BLOCKED_PASSED_PAWN_PENALTY_MG;
                eg += BLOCKED_PASSED_PAWN_PENALTY_EG;
            }
        }
    }

    for sq_idx in black_pawns {
        let file = sq_idx % 8;
        let rank = sq_idx / 8;
        let passed_mask = forward_mask(file as u8, rank as u8, Color::Black);
        if !(white_pawns & passed_mask).is_empty() {
            continue;
        }
        let own_dist = chebyshev_distance(black_king_sq, sq_idx);
        let opp_dist = chebyshev_distance(white_king_sq, sq_idx);
        eg -= KING_PROXIMITY_OWN_BONUS * (7 - own_dist);
        eg -= KING_PROXIMITY_OPP_BONUS * opp_dist;
        if rank > 0 {
            let advance_sq = sq_idx - 8;
            if !(occupied & Bitboard::new(1u64 << advance_sq)).is_empty() {
                mg -= BLOCKED_PASSED_PAWN_PENALTY_MG;
                eg -= BLOCKED_PASSED_PAWN_PENALTY_EG;
            }
        }
    }

    (mg, eg)
}

const DEFAULT_SIZE: usize = 16_384;

#[derive(Clone, Copy, Default)]
pub struct PawnHashEntry {
    key: u64,
    mg: i32,
    eg: i32,
}

pub struct PawnHashTable {
    entries: Vec<PawnHashEntry>,
    mask: usize,
}

impl Default for PawnHashTable {
    fn default() -> Self {
        Self::new()
    }
}

impl PawnHashTable {
    pub fn new() -> Self {
        Self {
            entries: vec![PawnHashEntry::default(); DEFAULT_SIZE],
            mask: DEFAULT_SIZE - 1,
        }
    }

    pub fn probe(&self, key: u64) -> Option<(i32, i32)> {
        let index = (key as usize) & self.mask;
        let entry = self.entries[index];
        if entry.key == key {
            Some((entry.mg, entry.eg))
        } else {
            None
        }
    }

    pub fn store(&mut self, key: u64, mg: i32, eg: i32) {
        let index = (key as usize) & self.mask;
        self.entries[index] = PawnHashEntry { key, mg, eg };
    }
}

pub fn pawn_zobrist_hash(pos: &Position) -> u64 {
    let mut hash: u64 = 0;
    for color in [Color::White, Color::Black] {
        let bb = pos.piece_bitboard(Piece::new(color, PieceKind::Pawn));
        for sq_idx in bb {
            let sq = Square::new(sq_idx as u8).unwrap();
            hash ^= piece_square_key(Piece::new(color, PieceKind::Pawn), sq);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_miss_returns_none() {
        let table = PawnHashTable::new();
        assert_eq!(table.probe(0xDEAD_BEEF), None);
    }

    #[test]
    fn store_then_probe_returns_correct_scores() {
        let mut table = PawnHashTable::new();
        table.store(0x1234, 50, 30);
        assert_eq!(table.probe(0x1234), Some((50, 30)));
    }

    #[test]
    fn different_pawn_configs_produce_different_keys() {
        let startpos =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let moved =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1")
                .unwrap();
        let key1 = pawn_zobrist_hash(&startpos);
        let key2 = pawn_zobrist_hash(&moved);
        assert_ne!(key1, key2);
    }

    #[test]
    fn doubled_pawns_penalized() {
        let pos = Position::from_fen("4k3/8/8/8/4P3/8/4P3/4K3 w - - 0 1").unwrap();
        let (mg, eg) = evaluate_pawns(&pos);
        assert!(
            mg < 0,
            "mg should be negative due to doubled+isolated: {mg}"
        );
        assert!(
            eg < 0,
            "eg should be negative due to doubled+isolated: {eg}"
        );
    }

    #[test]
    fn isolated_pawn_penalized() {
        let isolated = Position::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let supported = Position::from_fen("4k3/8/8/8/8/8/3PP3/4K3 w - - 0 1").unwrap();
        let (iso_mg, iso_eg) = evaluate_pawns(&isolated);
        let (sup_mg, sup_eg) = evaluate_pawns(&supported);
        assert!(
            iso_mg < sup_mg,
            "isolated mg {iso_mg} should be less than supported mg {sup_mg}"
        );
        assert!(
            iso_eg < sup_eg,
            "isolated eg {iso_eg} should be less than supported eg {sup_eg}"
        );
    }

    #[test]
    fn backward_pawn_penalized() {
        let backward = Position::from_fen("4k3/8/8/8/4p3/2P5/3P4/4K3 w - - 0 1").unwrap();
        let not_backward = Position::from_fen("4k3/8/8/8/4p3/2PP4/8/4K3 w - - 0 1").unwrap();
        let (bw_mg, bw_eg) = evaluate_pawns(&backward);
        let (nb_mg, nb_eg) = evaluate_pawns(&not_backward);
        assert!(
            bw_mg < nb_mg,
            "backward mg {bw_mg} should be less than non-backward mg {nb_mg}"
        );
        assert!(
            bw_eg < nb_eg,
            "backward eg {bw_eg} should be less than non-backward eg {nb_eg}"
        );
    }

    #[test]
    fn passed_pawn_bonus_scales_by_rank() {
        let rank4 = Position::from_fen("4k3/8/8/8/3P4/8/8/4K3 w - - 0 1").unwrap();
        let rank6 = Position::from_fen("4k3/8/3P4/8/8/8/8/4K3 w - - 0 1").unwrap();
        let (r4_mg, r4_eg) = evaluate_pawns(&rank4);
        let (r6_mg, r6_eg) = evaluate_pawns(&rank6);
        assert!(
            r6_mg > r4_mg,
            "rank6 mg {r6_mg} should exceed rank4 mg {r4_mg}"
        );
        assert!(
            r6_eg > r4_eg,
            "rank6 eg {r6_eg} should exceed rank4 eg {r4_eg}"
        );
    }

    #[test]
    fn connected_pawns_bonus() {
        let connected = Position::from_fen("4k3/8/8/8/3PP3/8/8/4K3 w - - 0 1").unwrap();
        let separated = Position::from_fen("4k3/8/8/8/3P1P2/8/8/4K3 w - - 0 1").unwrap();
        let (con_mg, con_eg) = evaluate_pawns(&connected);
        let (sep_mg, sep_eg) = evaluate_pawns(&separated);
        assert!(
            con_mg > sep_mg,
            "connected mg {con_mg} should exceed separated mg {sep_mg}"
        );
        assert!(
            con_eg > sep_eg,
            "connected eg {con_eg} should exceed separated eg {sep_eg}"
        );
    }

    #[test]
    fn both_colors_evaluated() {
        let pos = Position::from_fen("4k3/3p4/8/8/8/8/PPP5/4K3 w - - 0 1").unwrap();
        let (mg, eg) = evaluate_pawns(&pos);
        assert!(mg > 0, "mg should be positive (White advantage): {mg}");
        assert!(eg > 0, "eg should be positive (White advantage): {eg}");
    }

    #[test]
    fn blocked_passed_pawn_lower_than_unblocked() {
        let blocked = Position::from_fen("4k3/8/3n4/3P4/8/8/8/4K3 w - - 0 1").unwrap();
        let unblocked = Position::from_fen("4k3/8/8/3P4/8/8/8/4K3 w - - 0 1").unwrap();
        let (blk_mg, blk_eg) = evaluate_passed_pawn_extras(&blocked);
        let (unblk_mg, unblk_eg) = evaluate_passed_pawn_extras(&unblocked);
        assert!(
            blk_mg < unblk_mg,
            "blocked mg {blk_mg} should be less than unblocked mg {unblk_mg}"
        );
        assert!(
            blk_eg < unblk_eg,
            "blocked eg {blk_eg} should be less than unblocked eg {unblk_eg}"
        );
    }

    #[test]
    fn king_proximity_own_king_close_scores_higher() {
        let close = Position::from_fen("7k/8/8/3P4/4K3/8/8/8 w - - 0 1").unwrap();
        let far = Position::from_fen("7k/8/8/3P4/8/8/8/K7 w - - 0 1").unwrap();
        let (_, close_eg) = evaluate_passed_pawn_extras(&close);
        let (_, far_eg) = evaluate_passed_pawn_extras(&far);
        assert!(
            close_eg > far_eg,
            "close king eg {close_eg} should exceed far king eg {far_eg}"
        );
    }

    #[test]
    fn king_proximity_opp_king_far_scores_higher() {
        let far_opp = Position::from_fen("7k/8/8/3P4/8/8/8/4K3 w - - 0 1").unwrap();
        let close_opp = Position::from_fen("4k3/8/8/3P4/8/8/8/4K3 w - - 0 1").unwrap();
        let (_, far_eg) = evaluate_passed_pawn_extras(&far_opp);
        let (_, close_eg) = evaluate_passed_pawn_extras(&close_opp);
        assert!(
            far_eg > close_eg,
            "far opponent eg {far_eg} should exceed close opponent eg {close_eg}"
        );
    }

    #[test]
    fn startpos_pawns_symmetric() {
        let pos =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let (mg, eg) = evaluate_pawns(&pos);
        assert_eq!(mg, 0, "symmetric startpos mg should be 0");
        assert_eq!(eg, 0, "symmetric startpos eg should be 0");
    }
}
