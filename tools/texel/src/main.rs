use chess_board::Position;
use chess_movegen::king::KING_ATTACKS;
use chess_movegen::knights::KNIGHT_ATTACKS;
use chess_movegen::magic::{bishop_attacks, rook_attacks};
use chess_types::{Bitboard, Color, Piece, PieceKind, Square};
use std::env;
use std::fs;
use std::io::{self, BufRead};

// --- Non-tunable constants (file masks, adjacent file masks, bitboard helpers) ---

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
    Bitboard::new(0x0101_0101_0101_0101 << 1),
    Bitboard::new(0x0101_0101_0101_0101 | (0x0101_0101_0101_0101 << 2)),
    Bitboard::new((0x0101_0101_0101_0101 << 1) | (0x0101_0101_0101_0101 << 3)),
    Bitboard::new((0x0101_0101_0101_0101 << 2) | (0x0101_0101_0101_0101 << 4)),
    Bitboard::new((0x0101_0101_0101_0101 << 3) | (0x0101_0101_0101_0101 << 5)),
    Bitboard::new((0x0101_0101_0101_0101 << 4) | (0x0101_0101_0101_0101 << 6)),
    Bitboard::new((0x0101_0101_0101_0101 << 5) | (0x0101_0101_0101_0101 << 7)),
    Bitboard::new(0x0101_0101_0101_0101 << 6),
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

fn chebyshev_distance(sq1: u32, sq2: u32) -> i32 {
    let file1 = (sq1 % 8) as i32;
    let rank1 = (sq1 / 8) as i32;
    let file2 = (sq2 % 8) as i32;
    let rank2 = (sq2 / 8) as i32;
    (file1 - file2).abs().max((rank1 - rank2).abs())
}

fn chebyshev_distance_to_center(sq_idx: u32) -> i32 {
    let file = (sq_idx % 8) as i32;
    let rank = (sq_idx / 8) as i32;
    let dist_d4 = (file - 3).abs().max((rank - 3).abs());
    let dist_d5 = (file - 3).abs().max((rank - 4).abs());
    let dist_e4 = (file - 4).abs().max((rank - 3).abs());
    let dist_e5 = (file - 4).abs().max((rank - 4).abs());
    dist_d4.min(dist_d5).min(dist_e4).min(dist_e5)
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

fn pawn_attacks_bb(pawns: Bitboard, color: Color) -> Bitboard {
    match color {
        Color::White => ((pawns & NOT_A_FILE) << 7) | ((pawns & NOT_H_FILE) << 9),
        Color::Black => ((pawns & NOT_H_FILE) >> 7) | ((pawns & NOT_A_FILE) >> 9),
    }
}

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

fn is_passed_pawn(sq_idx: u32, color: Color, enemy_pawns: Bitboard) -> bool {
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

// --- Data structures ---

struct TuningEntry {
    pos: Position,
    result: f64,
}

#[allow(clippy::struct_field_names)]
struct EvalParams {
    material: [i32; 5],
    mg_pst: [[i32; 64]; 6],
    eg_pst: [[i32; 64]; 6],
    doubled_pawn_mg: i32,
    doubled_pawn_eg: i32,
    isolated_pawn_mg: i32,
    isolated_pawn_eg: i32,
    backward_pawn_mg: i32,
    backward_pawn_eg: i32,
    connected_pawn_mg: i32,
    connected_pawn_eg: i32,
    blocked_passed_pawn_mg: i32,
    blocked_passed_pawn_eg: i32,
    passed_pawn_bonus_mg: [i32; 8],
    passed_pawn_bonus_eg: [i32; 8],
    king_proximity_own: i32,
    king_proximity_opp: i32,
    pawn_shield_mg: [i32; 4],
    pawn_shield_eg: [i32; 4],
    open_file_king_mg: i32,
    open_file_king_eg: i32,
    attack_weights: [i32; 4],
    attack_scale_mg: i32,
    attack_scale_eg: i32,
    mobility_mg: [i32; 4],
    mobility_eg: [i32; 4],
    bishop_pair_mg: i32,
    bishop_pair_eg: i32,
    rook_open_file_mg: i32,
    rook_open_file_eg: i32,
    rook_semi_open_file_mg: i32,
    rook_semi_open_file_eg: i32,
    rook_seventh_mg: i32,
    rook_seventh_eg: i32,
    knight_outpost_mg: i32,
    knight_outpost_eg: i32,
    king_center_weight: i32,
    king_passed_pawn_proximity_weight: i32,
}

// --- EvalParams methods ---

impl EvalParams {
    // NOTE: keep in sync with eval crate constants
    fn from_current() -> Self {
        let kinds = [
            PieceKind::Pawn,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Rook,
            PieceKind::Queen,
            PieceKind::King,
        ];

        let mut mg_pst = [[0i32; 64]; 6];
        let mut eg_pst = [[0i32; 64]; 6];
        for (i, &kind) in kinds.iter().enumerate() {
            mg_pst[i] = *chess_eval::pst::mg_table(kind);
            eg_pst[i] = *chess_eval::pst::eg_table(kind);
        }

        EvalParams {
            material: [100, 320, 330, 500, 900],
            mg_pst,
            eg_pst,
            doubled_pawn_mg: -10,
            doubled_pawn_eg: -20,
            isolated_pawn_mg: -15,
            isolated_pawn_eg: -20,
            backward_pawn_mg: -10,
            backward_pawn_eg: -15,
            connected_pawn_mg: 5,
            connected_pawn_eg: 7,
            blocked_passed_pawn_mg: -5,
            blocked_passed_pawn_eg: -10,
            passed_pawn_bonus_mg: [0, 0, 5, 10, 20, 35, 60, 0],
            passed_pawn_bonus_eg: [0, 0, 10, 20, 40, 70, 120, 0],
            king_proximity_own: 5,
            king_proximity_opp: 3,
            pawn_shield_mg: [15, 8, 3, 0],
            pawn_shield_eg: [10, 5, 2, 0],
            open_file_king_mg: -15,
            open_file_king_eg: -10,
            attack_weights: [2, 2, 3, 5],
            attack_scale_mg: 1,
            attack_scale_eg: 1,
            mobility_mg: [4, 5, 2, 1],
            mobility_eg: [4, 5, 3, 2],
            bishop_pair_mg: 30,
            bishop_pair_eg: 50,
            rook_open_file_mg: 20,
            rook_open_file_eg: 10,
            rook_semi_open_file_mg: 10,
            rook_semi_open_file_eg: 5,
            rook_seventh_mg: 20,
            rook_seventh_eg: 30,
            knight_outpost_mg: 20,
            knight_outpost_eg: 15,
            king_center_weight: 10,
            king_passed_pawn_proximity_weight: 5,
        }
    }

    fn param_count() -> usize {
        841
    }

    fn as_param_vec(&self) -> Vec<i32> {
        let mut v = Vec::with_capacity(Self::param_count());
        // material: 5
        v.extend_from_slice(&self.material);
        // mg_pst: 6*64 = 384
        for table in &self.mg_pst {
            v.extend_from_slice(table);
        }
        // eg_pst: 6*64 = 384
        for table in &self.eg_pst {
            v.extend_from_slice(table);
        }
        // pawn scalars: 14
        v.push(self.doubled_pawn_mg);
        v.push(self.doubled_pawn_eg);
        v.push(self.isolated_pawn_mg);
        v.push(self.isolated_pawn_eg);
        v.push(self.backward_pawn_mg);
        v.push(self.backward_pawn_eg);
        v.push(self.connected_pawn_mg);
        v.push(self.connected_pawn_eg);
        v.push(self.blocked_passed_pawn_mg);
        v.push(self.blocked_passed_pawn_eg);
        // passed pawn arrays: 16
        v.extend_from_slice(&self.passed_pawn_bonus_mg);
        v.extend_from_slice(&self.passed_pawn_bonus_eg);
        // king proximity: 2
        v.push(self.king_proximity_own);
        v.push(self.king_proximity_opp);
        // pawn shield: 8
        v.extend_from_slice(&self.pawn_shield_mg);
        v.extend_from_slice(&self.pawn_shield_eg);
        // open file king: 2
        v.push(self.open_file_king_mg);
        v.push(self.open_file_king_eg);
        // attack weights: 4
        v.extend_from_slice(&self.attack_weights);
        // attack scale: 2
        v.push(self.attack_scale_mg);
        v.push(self.attack_scale_eg);
        // mobility: 8
        v.extend_from_slice(&self.mobility_mg);
        v.extend_from_slice(&self.mobility_eg);
        // piece bonuses: 10
        v.push(self.bishop_pair_mg);
        v.push(self.bishop_pair_eg);
        v.push(self.rook_open_file_mg);
        v.push(self.rook_open_file_eg);
        v.push(self.rook_semi_open_file_mg);
        v.push(self.rook_semi_open_file_eg);
        v.push(self.rook_seventh_mg);
        v.push(self.rook_seventh_eg);
        v.push(self.knight_outpost_mg);
        v.push(self.knight_outpost_eg);
        // king centralization: 2
        v.push(self.king_center_weight);
        v.push(self.king_passed_pawn_proximity_weight);
        debug_assert_eq!(v.len(), Self::param_count());
        v
    }

    fn from_param_vec(vec: &[i32]) -> Self {
        assert_eq!(vec.len(), Self::param_count());
        let mut idx = 0;

        let material: [i32; 5] = vec[idx..idx + 5].try_into().unwrap();
        idx += 5;

        let mut mg_pst = [[0i32; 64]; 6];
        for table in &mut mg_pst {
            *table = vec[idx..idx + 64].try_into().unwrap();
            idx += 64;
        }

        let mut eg_pst = [[0i32; 64]; 6];
        for table in &mut eg_pst {
            *table = vec[idx..idx + 64].try_into().unwrap();
            idx += 64;
        }

        let doubled_pawn_mg = vec[idx];
        let doubled_pawn_eg = vec[idx + 1];
        let isolated_pawn_mg = vec[idx + 2];
        let isolated_pawn_eg = vec[idx + 3];
        let backward_pawn_mg = vec[idx + 4];
        let backward_pawn_eg = vec[idx + 5];
        let connected_pawn_mg = vec[idx + 6];
        let connected_pawn_eg = vec[idx + 7];
        let blocked_passed_pawn_mg = vec[idx + 8];
        let blocked_passed_pawn_eg = vec[idx + 9];
        idx += 10;

        let passed_pawn_bonus_mg: [i32; 8] = vec[idx..idx + 8].try_into().unwrap();
        idx += 8;
        let passed_pawn_bonus_eg: [i32; 8] = vec[idx..idx + 8].try_into().unwrap();
        idx += 8;

        let king_proximity_own = vec[idx];
        let king_proximity_opp = vec[idx + 1];
        idx += 2;

        let pawn_shield_mg: [i32; 4] = vec[idx..idx + 4].try_into().unwrap();
        idx += 4;
        let pawn_shield_eg: [i32; 4] = vec[idx..idx + 4].try_into().unwrap();
        idx += 4;

        let open_file_king_mg = vec[idx];
        let open_file_king_eg = vec[idx + 1];
        idx += 2;

        let attack_weights: [i32; 4] = vec[idx..idx + 4].try_into().unwrap();
        idx += 4;

        let attack_scale_mg = vec[idx];
        let attack_scale_eg = vec[idx + 1];
        idx += 2;

        let mobility_mg: [i32; 4] = vec[idx..idx + 4].try_into().unwrap();
        idx += 4;
        let mobility_eg: [i32; 4] = vec[idx..idx + 4].try_into().unwrap();
        idx += 4;

        let bishop_pair_mg = vec[idx];
        let bishop_pair_eg = vec[idx + 1];
        let rook_open_file_mg = vec[idx + 2];
        let rook_open_file_eg = vec[idx + 3];
        let rook_semi_open_file_mg = vec[idx + 4];
        let rook_semi_open_file_eg = vec[idx + 5];
        let rook_seventh_mg = vec[idx + 6];
        let rook_seventh_eg = vec[idx + 7];
        let knight_outpost_mg = vec[idx + 8];
        let knight_outpost_eg = vec[idx + 9];
        idx += 10;

        let king_center_weight = vec[idx];
        let king_passed_pawn_proximity_weight = vec[idx + 1];

        EvalParams {
            material,
            mg_pst,
            eg_pst,
            doubled_pawn_mg,
            doubled_pawn_eg,
            isolated_pawn_mg,
            isolated_pawn_eg,
            backward_pawn_mg,
            backward_pawn_eg,
            connected_pawn_mg,
            connected_pawn_eg,
            blocked_passed_pawn_mg,
            blocked_passed_pawn_eg,
            passed_pawn_bonus_mg,
            passed_pawn_bonus_eg,
            king_proximity_own,
            king_proximity_opp,
            pawn_shield_mg,
            pawn_shield_eg,
            open_file_king_mg,
            open_file_king_eg,
            attack_weights,
            attack_scale_mg,
            attack_scale_eg,
            mobility_mg,
            mobility_eg,
            bishop_pair_mg,
            bishop_pair_eg,
            rook_open_file_mg,
            rook_open_file_eg,
            rook_semi_open_file_mg,
            rook_semi_open_file_eg,
            rook_seventh_mg,
            rook_seventh_eg,
            knight_outpost_mg,
            knight_outpost_eg,
            king_center_weight,
            king_passed_pawn_proximity_weight,
        }
    }
}

// --- Dataset loading ---

fn load_dataset(path: &str) -> Vec<TuningEntry> {
    let file = fs::File::open(path).expect("failed to open dataset file");
    let reader = io::BufReader::new(file);
    let mut entries = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(2, '|').collect();
        if parts.len() != 2 {
            continue;
        }
        let fen = parts[0].trim();
        let result_str = parts[1].trim();

        let result: f64 = match result_str.parse() {
            Ok(r) => r,
            Err(_) => continue,
        };
        if !(0.0..=1.0).contains(&result) {
            continue;
        }

        let pos = match Position::from_fen(fen) {
            Ok(p) => p,
            Err(_) => continue,
        };

        entries.push(TuningEntry { pos, result });
    }

    entries
}

// --- Sigmoid and MSE ---

fn sigmoid(eval: f64, k: f64) -> f64 {
    1.0 / (1.0 + 10.0_f64.powf(-k * eval / 400.0))
}

fn evaluate_position(pos: &Position) -> i32 {
    let mut pawn_table = chess_eval::PawnHashTable::new();
    let score = chess_eval::evaluate(pos, &mut pawn_table);
    if pos.side_to_move() == Color::White {
        score
    } else {
        -score
    }
}

fn compute_mse(data: &[TuningEntry], k: f64) -> f64 {
    let mut total = 0.0;
    for entry in data {
        let eval = evaluate_position(&entry.pos) as f64;
        let predicted = sigmoid(eval, k);
        let diff = predicted - entry.result;
        total += diff * diff;
    }
    total / data.len() as f64
}

// --- Custom evaluation using EvalParams ---

fn eval_with_params(pos: &Position, params: &EvalParams) -> i32 {
    // 0. Endgame probe
    if let Some(eg_score) = chess_eval::endgame::probe_endgame(pos) {
        return eg_score;
    }

    let mut mg_score: i32 = 0;
    let mut eg_score: i32 = 0;

    // 1. Material + PST
    let kinds = [
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ];

    for (kind_idx, &kind) in kinds.iter().enumerate() {
        let mat_value = if kind_idx < 5 {
            params.material[kind_idx]
        } else {
            0
        };

        let white_bb = pos.piece_bitboard(Piece::new(Color::White, kind));
        for sq_idx in white_bb {
            let idx = sq_idx as usize;
            mg_score += mat_value + params.mg_pst[kind_idx][idx];
            eg_score += mat_value + params.eg_pst[kind_idx][idx];
        }

        let black_bb = pos.piece_bitboard(Piece::new(Color::Black, kind));
        for sq_idx in black_bb {
            let idx = chess_eval::pst::mirror_square(sq_idx as u8);
            mg_score -= mat_value + params.mg_pst[kind_idx][idx];
            eg_score -= mat_value + params.eg_pst[kind_idx][idx];
        }
    }

    // 2. Pawn structure
    let (pawn_mg, pawn_eg) = eval_pawns_with_params(pos, params);
    let (pp_extra_mg, pp_extra_eg) = eval_passed_pawn_extras_with_params(pos, params);
    mg_score += pawn_mg + pp_extra_mg;
    eg_score += pawn_eg + pp_extra_eg;

    // 3. King safety
    let (w_ks_mg, w_ks_eg) = eval_king_safety_with_params(pos, Color::White, params);
    let (b_ks_mg, b_ks_eg) = eval_king_safety_with_params(pos, Color::Black, params);
    mg_score += w_ks_mg - b_ks_mg;
    eg_score += w_ks_eg - b_ks_eg;

    // 4. Mobility
    let (w_mob_mg, w_mob_eg) = eval_mobility_with_params(pos, Color::White, params);
    let (b_mob_mg, b_mob_eg) = eval_mobility_with_params(pos, Color::Black, params);
    mg_score += w_mob_mg - b_mob_mg;
    eg_score += w_mob_eg - b_mob_eg;

    // 5. Piece bonuses
    let (w_pc_mg, w_pc_eg) = eval_piece_bonuses_with_params(pos, Color::White, params);
    let (b_pc_mg, b_pc_eg) = eval_piece_bonuses_with_params(pos, Color::Black, params);
    mg_score += w_pc_mg - b_pc_mg;
    eg_score += w_pc_eg - b_pc_eg;

    // 6. King centralization
    let (kc_mg, kc_eg) = eval_king_centralization_with_params(pos, params);
    mg_score += kc_mg;
    eg_score += kc_eg;

    // 7. Tapered interpolation
    let phase = chess_eval::phase::compute_phase(pos);
    let max_phase = chess_eval::phase::MAX_PHASE;
    ((mg_score * phase) + (eg_score * (max_phase - phase))) / max_phase
}

fn eval_pawn_color_with_params(
    friendly_pawns: Bitboard,
    enemy_pawns: Bitboard,
    color: Color,
    params: &EvalParams,
) -> (i32, i32) {
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
                mg += (count as i32 - 1) * params.doubled_pawn_mg;
                eg += (count as i32 - 1) * params.doubled_pawn_eg;
            }
            doubled_counted[file as usize] = true;
        }

        let isolated = (friendly_pawns & ADJACENT_FILE_MASKS[file as usize]).is_empty();

        if isolated {
            mg += params.isolated_pawn_mg;
            eg += params.isolated_pawn_eg;
        }

        let passed_mask = forward_mask(file as u8, rank as u8, color);
        let is_passed = (enemy_pawns & passed_mask).is_empty();

        if is_passed {
            let bonus_rank = match color {
                Color::White => rank,
                Color::Black => 7 - rank,
            };
            mg += params.passed_pawn_bonus_mg[bonus_rank as usize];
            eg += params.passed_pawn_bonus_eg[bonus_rank as usize];
        }

        if !isolated && !is_passed {
            let support_mask = ADJACENT_FILE_MASKS[file as usize] & ranks_up_to(rank as u8);
            let has_support = !(friendly_pawns & support_mask).is_empty();

            if !has_support {
                let advance_attacked = match color {
                    Color::White => {
                        if rank < 7 {
                            let advance_rank = rank + 1;
                            if advance_rank < 7 {
                                let enemy_attacker_mask = ADJACENT_FILE_MASKS[file as usize]
                                    & rank_bb(advance_rank as u8 + 1);
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
                    mg += params.backward_pawn_mg;
                    eg += params.backward_pawn_eg;
                }
            }
        }

        let sq_bb = Bitboard::new(1u64 << sq_idx);
        if !(sq_bb & pawn_attacks).is_empty() {
            mg += params.connected_pawn_mg;
            eg += params.connected_pawn_eg;
        }
    }

    (mg, eg)
}

fn eval_pawns_with_params(pos: &Position, params: &EvalParams) -> (i32, i32) {
    let white_pawns = pos.piece_bitboard(Piece::new(Color::White, PieceKind::Pawn));
    let black_pawns = pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Pawn));

    let (white_mg, white_eg) =
        eval_pawn_color_with_params(white_pawns, black_pawns, Color::White, params);
    let (black_mg, black_eg) =
        eval_pawn_color_with_params(black_pawns, white_pawns, Color::Black, params);

    (white_mg - black_mg, white_eg - black_eg)
}

fn eval_passed_pawn_extras_with_params(pos: &Position, params: &EvalParams) -> (i32, i32) {
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
        eg += params.king_proximity_own * (7 - own_dist);
        eg += params.king_proximity_opp * opp_dist;
        if rank < 7 {
            let advance_sq = sq_idx + 8;
            if !(occupied & Bitboard::new(1u64 << advance_sq)).is_empty() {
                mg += params.blocked_passed_pawn_mg;
                eg += params.blocked_passed_pawn_eg;
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
        eg -= params.king_proximity_own * (7 - own_dist);
        eg -= params.king_proximity_opp * opp_dist;
        if rank > 0 {
            let advance_sq = sq_idx - 8;
            if !(occupied & Bitboard::new(1u64 << advance_sq)).is_empty() {
                mg -= params.blocked_passed_pawn_mg;
                eg -= params.blocked_passed_pawn_eg;
            }
        }
    }

    (mg, eg)
}

fn eval_king_safety_with_params(pos: &Position, color: Color, params: &EvalParams) -> (i32, i32) {
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
            mg += params.open_file_king_mg;
            eg += params.open_file_king_eg;
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
            mg += params.pawn_shield_mg[idx];
            eg += params.pawn_shield_eg[idx];
        }
    }

    let zone = king_zone(king_sq, color);
    let opp = color.opposite();
    let occupied = pos.occupied();
    let mut total_attack = 0i32;

    let opp_knights = pos.piece_bitboard(Piece::new(opp, PieceKind::Knight));
    for sq_idx in opp_knights {
        let attacks = KNIGHT_ATTACKS[sq_idx as usize];
        total_attack += (attacks & zone).pop_count() as i32 * params.attack_weights[0];
    }

    let opp_bishops = pos.piece_bitboard(Piece::new(opp, PieceKind::Bishop));
    for sq_idx in opp_bishops {
        let sq = Square::new(sq_idx as u8).unwrap();
        let attacks = bishop_attacks(sq, occupied);
        total_attack += (attacks & zone).pop_count() as i32 * params.attack_weights[1];
    }

    let opp_rooks = pos.piece_bitboard(Piece::new(opp, PieceKind::Rook));
    for sq_idx in opp_rooks {
        let sq = Square::new(sq_idx as u8).unwrap();
        let attacks = rook_attacks(sq, occupied);
        total_attack += (attacks & zone).pop_count() as i32 * params.attack_weights[2];
    }

    let opp_queens = pos.piece_bitboard(Piece::new(opp, PieceKind::Queen));
    for sq_idx in opp_queens {
        let sq = Square::new(sq_idx as u8).unwrap();
        let b_attacks = bishop_attacks(sq, occupied);
        let r_attacks = rook_attacks(sq, occupied);
        total_attack +=
            ((b_attacks | r_attacks) & zone).pop_count() as i32 * params.attack_weights[3];
    }

    mg -= total_attack * params.attack_scale_mg;
    eg -= total_attack * params.attack_scale_eg;

    (mg, eg)
}

fn eval_mobility_with_params(pos: &Position, color: Color, params: &EvalParams) -> (i32, i32) {
    let opponent = color.opposite();
    let opponent_pawns = pos.piece_bitboard(Piece::new(opponent, PieceKind::Pawn));
    let opp_pawn_attacks = pawn_attacks_bb(opponent_pawns, opponent);
    let safe_squares = !opp_pawn_attacks;
    let occupied = pos.occupied();

    let mut mg_score: i32 = 0;
    let mut eg_score: i32 = 0;

    let knights = pos.piece_bitboard(Piece::new(color, PieceKind::Knight));
    for sq_idx in knights {
        let mobility = (KNIGHT_ATTACKS[sq_idx as usize] & safe_squares).pop_count() as i32;
        mg_score += mobility * params.mobility_mg[0];
        eg_score += mobility * params.mobility_eg[0];
    }

    let bishops = pos.piece_bitboard(Piece::new(color, PieceKind::Bishop));
    for sq_idx in bishops {
        let sq = Square::new(sq_idx as u8).unwrap();
        let attacks = bishop_attacks(sq, occupied);
        let mobility = (attacks & safe_squares).pop_count() as i32;
        mg_score += mobility * params.mobility_mg[1];
        eg_score += mobility * params.mobility_eg[1];
    }

    let rook_bb = pos.piece_bitboard(Piece::new(color, PieceKind::Rook));
    for sq_idx in rook_bb {
        let sq = Square::new(sq_idx as u8).unwrap();
        let attacks = rook_attacks(sq, occupied);
        let mobility = (attacks & safe_squares).pop_count() as i32;
        mg_score += mobility * params.mobility_mg[2];
        eg_score += mobility * params.mobility_eg[2];
    }

    let queens = pos.piece_bitboard(Piece::new(color, PieceKind::Queen));
    for sq_idx in queens {
        let sq = Square::new(sq_idx as u8).unwrap();
        let b_attacks = bishop_attacks(sq, occupied);
        let r_attacks = rook_attacks(sq, occupied);
        let mobility = ((b_attacks | r_attacks) & safe_squares).pop_count() as i32;
        mg_score += mobility * params.mobility_mg[3];
        eg_score += mobility * params.mobility_eg[3];
    }

    (mg_score, eg_score)
}

fn eval_piece_bonuses_with_params(pos: &Position, color: Color, params: &EvalParams) -> (i32, i32) {
    let mut mg = 0;
    let mut eg = 0;

    let friendly_pawns = pos.piece_bitboard(Piece::new(color, PieceKind::Pawn));
    let enemy_pawns = pos.piece_bitboard(Piece::new(color.opposite(), PieceKind::Pawn));

    let bishops = pos.piece_bitboard(Piece::new(color, PieceKind::Bishop));
    if bishops.pop_count() >= 2 {
        mg += params.bishop_pair_mg;
        eg += params.bishop_pair_eg;
    }

    let rooks = pos.piece_bitboard(Piece::new(color, PieceKind::Rook));
    for sq_idx in rooks {
        let file = (sq_idx % 8) as usize;
        let rank = sq_idx / 8;
        let file_mask = FILE_MASKS[file];

        if (friendly_pawns & file_mask).is_empty() && (enemy_pawns & file_mask).is_empty() {
            mg += params.rook_open_file_mg;
            eg += params.rook_open_file_eg;
        } else if (friendly_pawns & file_mask).is_empty() {
            mg += params.rook_semi_open_file_mg;
            eg += params.rook_semi_open_file_eg;
        }

        let seventh = match color {
            Color::White => 6,
            Color::Black => 1,
        };
        if rank == seventh {
            mg += params.rook_seventh_mg;
            eg += params.rook_seventh_eg;
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
            mg += params.knight_outpost_mg;
            eg += params.knight_outpost_eg;
        }
    }

    (mg, eg)
}

fn eval_king_centralization_with_params(pos: &Position, params: &EvalParams) -> (i32, i32) {
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
        (3 - chebyshev_distance_to_center(white_king_sq)).max(0) * params.king_center_weight;
    let black_center_bonus =
        (3 - chebyshev_distance_to_center(black_king_sq)).max(0) * params.king_center_weight;

    let mut eg = white_center_bonus - black_center_bonus;

    if is_pure_pawn_endgame(pos) {
        let white_pawns = pos.piece_bitboard(Piece::new(Color::White, PieceKind::Pawn));
        let black_pawns = pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Pawn));

        for sq_idx in white_pawns {
            if is_passed_pawn(sq_idx, Color::White, black_pawns) {
                let dist = chebyshev_distance(white_king_sq, sq_idx);
                eg += (7 - dist) * params.king_passed_pawn_proximity_weight;
            }
        }

        for sq_idx in black_pawns {
            if is_passed_pawn(sq_idx, Color::Black, white_pawns) {
                let dist = chebyshev_distance(black_king_sq, sq_idx);
                eg -= (7 - dist) * params.king_passed_pawn_proximity_weight;
            }
        }
    }

    (0, eg)
}

// --- K-constant optimization ---

fn optimize_k(data: &[TuningEntry]) -> f64 {
    let mut best_k = 0.0;
    let mut best_mse = f64::MAX;

    let mut step = 1.0;
    let mut lo = 0.0;
    let mut hi = 10.0;

    for _ in 0..4 {
        let mut k = lo;
        while k <= hi {
            let mse = compute_mse(data, k);
            if mse < best_mse {
                best_mse = mse;
                best_k = k;
            }
            k += step;
        }
        lo = best_k - step;
        hi = best_k + step;
        step /= 10.0;
    }

    eprintln!("Optimal K = {best_k:.4} (MSE = {best_mse:.8})");
    best_k
}

// --- Iterative weight optimization ---

fn compute_mse_with_params(data: &[TuningEntry], params: &EvalParams, k: f64) -> f64 {
    let mut total = 0.0;
    for entry in data {
        let eval = eval_with_params(&entry.pos, params) as f64;
        let predicted = sigmoid(eval, k);
        let diff = predicted - entry.result;
        total += diff * diff;
    }
    total / data.len() as f64
}

fn optimize_weights(data: &[TuningEntry], k: f64, iterations: usize) -> EvalParams {
    let mut params = EvalParams::from_current();
    let mut best_mse = compute_mse_with_params(data, &params, k);
    eprintln!("Initial MSE: {best_mse:.8}");

    for iter in 0..iterations {
        let mut vec = params.as_param_vec();
        let delta = if iter < iterations / 2 { 10 } else { 1 };
        let mut improved = false;

        for i in 0..EvalParams::param_count() {
            let original = vec[i];

            // Try +delta
            vec[i] = original + delta;
            let candidate = EvalParams::from_param_vec(&vec);
            let mse_plus = compute_mse_with_params(data, &candidate, k);

            if mse_plus < best_mse {
                best_mse = mse_plus;
                improved = true;
                continue;
            }

            // Try -delta
            vec[i] = original - delta;
            let candidate = EvalParams::from_param_vec(&vec);
            let mse_minus = compute_mse_with_params(data, &candidate, k);

            if mse_minus < best_mse {
                best_mse = mse_minus;
                improved = true;
                continue;
            }

            // Revert
            vec[i] = original;
        }

        params = EvalParams::from_param_vec(&vec);
        eprintln!("Iteration {}: MSE = {best_mse:.8}", iter + 1);

        if !improved {
            eprintln!("No improvement, stopping early.");
            break;
        }
    }

    params
}

// --- Output formatting ---

fn print_weights(params: &EvalParams) {
    // material.rs
    println!("// material.rs");
    let mat_names = [
        "PAWN_VALUE",
        "KNIGHT_VALUE",
        "BISHOP_VALUE",
        "ROOK_VALUE",
        "QUEEN_VALUE",
    ];
    for (i, name) in mat_names.iter().enumerate() {
        println!("pub const {}: i32 = {};", name, params.material[i]);
    }
    println!();

    // pst.rs
    println!("// pst.rs");
    let pst_names_mg = [
        "MG_PAWN_TABLE",
        "MG_KNIGHT_TABLE",
        "MG_BISHOP_TABLE",
        "MG_ROOK_TABLE",
        "MG_QUEEN_TABLE",
        "MG_KING_TABLE",
    ];
    let pst_names_eg = [
        "EG_PAWN_TABLE",
        "EG_KNIGHT_TABLE",
        "EG_BISHOP_TABLE",
        "EG_ROOK_TABLE",
        "EG_QUEEN_TABLE",
        "EG_KING_TABLE",
    ];
    for (i, name) in pst_names_mg.iter().enumerate() {
        print_pst_table(name, &params.mg_pst[i]);
    }
    for (i, name) in pst_names_eg.iter().enumerate() {
        print_pst_table(name, &params.eg_pst[i]);
    }
    println!();

    // pawn.rs
    println!("// pawn.rs");
    println!(
        "const DOUBLED_PAWN_PENALTY_MG: i32 = {};",
        params.doubled_pawn_mg
    );
    println!(
        "const DOUBLED_PAWN_PENALTY_EG: i32 = {};",
        params.doubled_pawn_eg
    );
    println!(
        "const ISOLATED_PAWN_PENALTY_MG: i32 = {};",
        params.isolated_pawn_mg
    );
    println!(
        "const ISOLATED_PAWN_PENALTY_EG: i32 = {};",
        params.isolated_pawn_eg
    );
    println!(
        "const BACKWARD_PAWN_PENALTY_MG: i32 = {};",
        params.backward_pawn_mg
    );
    println!(
        "const BACKWARD_PAWN_PENALTY_EG: i32 = {};",
        params.backward_pawn_eg
    );
    println!(
        "const CONNECTED_PAWN_BONUS_MG: i32 = {};",
        params.connected_pawn_mg
    );
    println!(
        "const CONNECTED_PAWN_BONUS_EG: i32 = {};",
        params.connected_pawn_eg
    );
    println!(
        "const BLOCKED_PASSED_PAWN_PENALTY_MG: i32 = {};",
        params.blocked_passed_pawn_mg
    );
    println!(
        "const BLOCKED_PASSED_PAWN_PENALTY_EG: i32 = {};",
        params.blocked_passed_pawn_eg
    );
    print!(
        "const PASSED_PAWN_BONUS_MG: [i32; 8] = {:?};",
        params.passed_pawn_bonus_mg
    );
    println!();
    print!(
        "const PASSED_PAWN_BONUS_EG: [i32; 8] = {:?};",
        params.passed_pawn_bonus_eg
    );
    println!();
    println!(
        "const KING_PROXIMITY_OWN_BONUS: i32 = {};",
        params.king_proximity_own
    );
    println!(
        "const KING_PROXIMITY_OPP_BONUS: i32 = {};",
        params.king_proximity_opp
    );
    println!();

    // king_safety.rs
    println!("// king_safety.rs");
    println!(
        "const PAWN_SHIELD_BONUS_MG: [i32; 4] = {:?};",
        params.pawn_shield_mg
    );
    println!(
        "const PAWN_SHIELD_BONUS_EG: [i32; 4] = {:?};",
        params.pawn_shield_eg
    );
    println!(
        "const OPEN_FILE_NEAR_KING_PENALTY_MG: i32 = {};",
        params.open_file_king_mg
    );
    println!(
        "const OPEN_FILE_NEAR_KING_PENALTY_EG: i32 = {};",
        params.open_file_king_eg
    );
    println!(
        "const KNIGHT_ATTACK_WEIGHT: i32 = {};",
        params.attack_weights[0]
    );
    println!(
        "const BISHOP_ATTACK_WEIGHT: i32 = {};",
        params.attack_weights[1]
    );
    println!(
        "const ROOK_ATTACK_WEIGHT: i32 = {};",
        params.attack_weights[2]
    );
    println!(
        "const QUEEN_ATTACK_WEIGHT: i32 = {};",
        params.attack_weights[3]
    );
    println!(
        "const ATTACK_SCORE_SCALE_MG: i32 = {};",
        params.attack_scale_mg
    );
    println!(
        "const ATTACK_SCORE_SCALE_EG: i32 = {};",
        params.attack_scale_eg
    );
    println!();

    // mobility.rs
    println!("// mobility.rs");
    println!("const KNIGHT_MOBILITY_MG: i32 = {};", params.mobility_mg[0]);
    println!("const KNIGHT_MOBILITY_EG: i32 = {};", params.mobility_eg[0]);
    println!("const BISHOP_MOBILITY_MG: i32 = {};", params.mobility_mg[1]);
    println!("const BISHOP_MOBILITY_EG: i32 = {};", params.mobility_eg[1]);
    println!("const ROOK_MOBILITY_MG: i32 = {};", params.mobility_mg[2]);
    println!("const ROOK_MOBILITY_EG: i32 = {};", params.mobility_eg[2]);
    println!("const QUEEN_MOBILITY_MG: i32 = {};", params.mobility_mg[3]);
    println!("const QUEEN_MOBILITY_EG: i32 = {};", params.mobility_eg[3]);
    println!();

    // pieces.rs
    println!("// pieces.rs");
    println!(
        "const BISHOP_PAIR_BONUS_MG: i32 = {};",
        params.bishop_pair_mg
    );
    println!(
        "const BISHOP_PAIR_BONUS_EG: i32 = {};",
        params.bishop_pair_eg
    );
    println!(
        "const ROOK_OPEN_FILE_BONUS_MG: i32 = {};",
        params.rook_open_file_mg
    );
    println!(
        "const ROOK_OPEN_FILE_BONUS_EG: i32 = {};",
        params.rook_open_file_eg
    );
    println!(
        "const ROOK_SEMI_OPEN_FILE_BONUS_MG: i32 = {};",
        params.rook_semi_open_file_mg
    );
    println!(
        "const ROOK_SEMI_OPEN_FILE_BONUS_EG: i32 = {};",
        params.rook_semi_open_file_eg
    );
    println!(
        "const ROOK_SEVENTH_RANK_BONUS_MG: i32 = {};",
        params.rook_seventh_mg
    );
    println!(
        "const ROOK_SEVENTH_RANK_BONUS_EG: i32 = {};",
        params.rook_seventh_eg
    );
    println!(
        "const KNIGHT_OUTPOST_BONUS_MG: i32 = {};",
        params.knight_outpost_mg
    );
    println!(
        "const KNIGHT_OUTPOST_BONUS_EG: i32 = {};",
        params.knight_outpost_eg
    );
    println!();

    // king_centralization.rs
    println!("// king_centralization.rs");
    println!(
        "const KING_CENTER_BONUS_WEIGHT: i32 = {};",
        params.king_center_weight
    );
    println!(
        "const KING_PASSED_PAWN_PROXIMITY_WEIGHT: i32 = {};",
        params.king_passed_pawn_proximity_weight
    );
}

fn print_pst_table(name: &str, table: &[i32; 64]) {
    println!("#[rustfmt::skip]");
    println!("pub const {}: [i32; 64] = [", name);
    for rank in 0..8 {
        let start = rank * 8;
        print!("   ");
        for file in 0..8 {
            let val = table[start + file];
            print!("{val:4}");
            if file < 7 || rank < 7 {
                print!(",");
            }
        }
        println!("  // rank {}", rank + 1);
    }
    println!("];");
}

// --- CLI and main ---

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut data_path: Option<String> = None;
    let mut optimize_k_only = false;
    let mut iterations: usize = 100;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--data" => {
                i += 1;
                if i < args.len() {
                    data_path = Some(args[i].clone());
                }
            }
            "--optimize-k" => {
                optimize_k_only = true;
            }
            "--iterations" => {
                i += 1;
                if i < args.len() {
                    iterations = args[i].parse().expect("invalid iterations value");
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let data_path = data_path.expect("--data <path> is required");

    let data = load_dataset(&data_path);
    eprintln!("Loaded {} positions", data.len());

    let mut wins = 0;
    let mut draws = 0;
    let mut losses = 0;
    for entry in &data {
        if entry.result == 1.0 {
            wins += 1;
        } else if entry.result == 0.5 {
            draws += 1;
        } else {
            losses += 1;
        }
    }
    eprintln!("W/D/L: {wins}/{draws}/{losses}");

    let k = optimize_k(&data);

    if optimize_k_only {
        return;
    }

    let optimized = optimize_weights(&data, k, iterations);
    print_weights(&optimized);
}
