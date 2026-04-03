use chess_types::{Bitboard, Square};
use std::sync::OnceLock;

// --- Blocker mask generation ---

const fn bishop_blocker_mask(sq: usize) -> Bitboard {
    let file = (sq % 8) as i8;
    let rank = (sq / 8) as i8;
    let mut bits: u64 = 0;

    // Four diagonal directions
    let dirs: [(i8, i8); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
    let mut d = 0;
    while d < 4 {
        let (df, dr) = dirs[d];
        let mut f = file + df;
        let mut r = rank + dr;
        while f > 0 && f < 7 && r > 0 && r < 7 {
            bits |= 1u64 << (r as usize * 8 + f as usize);
            f += df;
            r += dr;
        }
        d += 1;
    }
    Bitboard::new(bits)
}

const fn rook_blocker_mask(sq: usize) -> Bitboard {
    let file = (sq % 8) as i8;
    let rank = (sq / 8) as i8;
    let mut bits: u64 = 0;

    // File ray up (exclude rank 7 edge)
    let mut r = rank + 1;
    while r < 7 {
        bits |= 1u64 << (r as usize * 8 + file as usize);
        r += 1;
    }
    // File ray down (exclude rank 0 edge)
    r = rank - 1;
    while r > 0 {
        bits |= 1u64 << (r as usize * 8 + file as usize);
        r -= 1;
    }
    // Rank ray right (exclude file 7 edge)
    let mut f = file + 1;
    while f < 7 {
        bits |= 1u64 << (rank as usize * 8 + f as usize);
        f += 1;
    }
    // Rank ray left (exclude file 0 edge)
    f = file - 1;
    while f > 0 {
        bits |= 1u64 << (rank as usize * 8 + f as usize);
        f -= 1;
    }
    Bitboard::new(bits)
}

// --- Precomputed mask arrays ---

const fn init_bishop_masks() -> [Bitboard; 64] {
    let mut masks = [Bitboard::new(0); 64];
    let mut sq = 0;
    while sq < 64 {
        masks[sq] = bishop_blocker_mask(sq);
        sq += 1;
    }
    masks
}

const fn init_rook_masks() -> [Bitboard; 64] {
    let mut masks = [Bitboard::new(0); 64];
    let mut sq = 0;
    while sq < 64 {
        masks[sq] = rook_blocker_mask(sq);
        sq += 1;
    }
    masks
}

const BISHOP_MASKS: [Bitboard; 64] = init_bishop_masks();
const ROOK_MASKS: [Bitboard; 64] = init_rook_masks();

// --- Slow reference attack functions ---

const fn bishop_attacks_slow(sq: usize, occupied: u64) -> Bitboard {
    let file = (sq % 8) as i8;
    let rank = (sq / 8) as i8;
    let mut bits: u64 = 0;

    let dirs: [(i8, i8); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
    let mut d = 0;
    while d < 4 {
        let (df, dr) = dirs[d];
        let mut f = file + df;
        let mut r = rank + dr;
        while f >= 0 && f <= 7 && r >= 0 && r <= 7 {
            let bit = 1u64 << (r as usize * 8 + f as usize);
            bits |= bit;
            if occupied & bit != 0 {
                break;
            }
            f += df;
            r += dr;
        }
        d += 1;
    }
    Bitboard::new(bits)
}

const fn rook_attacks_slow(sq: usize, occupied: u64) -> Bitboard {
    let file = (sq % 8) as i8;
    let rank = (sq / 8) as i8;
    let mut bits: u64 = 0;

    let dirs: [(i8, i8); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    let mut d = 0;
    while d < 4 {
        let (df, dr) = dirs[d];
        let mut f = file + df;
        let mut r = rank + dr;
        while f >= 0 && f <= 7 && r >= 0 && r <= 7 {
            let bit = 1u64 << (r as usize * 8 + f as usize);
            bits |= bit;
            if occupied & bit != 0 {
                break;
            }
            f += df;
            r += dr;
        }
        d += 1;
    }
    Bitboard::new(bits)
}

// --- Pre-computed magic numbers (from BBC chess engine by Maksim Korzh) ---

#[rustfmt::skip]
const BISHOP_MAGICS: [u64; 64] = [
    0x0040040844404084, 0x002004208a004208, 0x0010190041080202, 0x0108060845042010,
    0x0581104180800210, 0x2112080446200010, 0x1080820820060210, 0x03c0808410220200,
    0x0004050404440404, 0x0000021001420088, 0x24d0080801082102, 0x0001020a0a020400,
    0x0000040308200402, 0x0004011002100800, 0x0401484104104005, 0x0801010402020200,
    0x00400210c3880100, 0x0404022024108200, 0x0810018200204102, 0x0004002801a02003,
    0x0085040820080400, 0x810102c808880400, 0x000e900410884800, 0x8002020480840102,
    0x0220200865090201, 0x2010100a02021202, 0x0152048408022401, 0x0020080002081110,
    0x4001001021004000, 0x800040400a011002, 0x00e4004081011002, 0x001c004001012080,
    0x8004200962a00220, 0x8422100208500202, 0x2000402200300c08, 0x8646020080080080,
    0x80020a0200100808, 0x2010004880111000, 0x623000a080011400, 0x42008c0340209202,
    0x0209188240001000, 0x400408a884001800, 0x00110400a6080400, 0x1840060a44020800,
    0x0090080104000041, 0x0201011000808101, 0x1a2208080504f080, 0x8012020600211212,
    0x0500861011240000, 0x0180806108200800, 0x4000020e01040044, 0x300000261044000a,
    0x0802241102020002, 0x0020906061210001, 0x5a84841004010310, 0x0004010801011c04,
    0x000a010109502200, 0x0000004a02012000, 0x500201010098b028, 0x8040002811040900,
    0x0028000010020204, 0x06000020202d0240, 0x8918844842082200, 0x4010011029020020,
];

#[rustfmt::skip]
const ROOK_MAGICS: [u64; 64] = [
    0x8a80104000800020, 0x0140002000100040, 0x02801880a0017001, 0x0100081001000420,
    0x0200020010080420, 0x03001c0002010008, 0x8480008002000100, 0x2080088004402900,
    0x0000800098204000, 0x2024401000200040, 0x0100802000801000, 0x0120800800801000,
    0x0208808088000400, 0x0002802200800400, 0x2200800100020080, 0x0801000060821100,
    0x0080044006422000, 0x0100808020004000, 0x12108a0010204200, 0x0140848010000802,
    0x0481828014002800, 0x8094004002004100, 0x4010040010010802, 0x0000020008806104,
    0x0100400080208000, 0x2040002120081000, 0x0021200680100081, 0x0020100080080080,
    0x0002000a00200410, 0x0000020080800400, 0x0080088400100102, 0x0080004600042881,
    0x4040008040800020, 0x0440003000200801, 0x0004200011004500, 0x0188020010100100,
    0x0014800401802800, 0x2080040080800200, 0x0124080204001001, 0x0200046502000484,
    0x0480400080088020, 0x1000422010034000, 0x0030200100110040, 0x0000100021010009,
    0x2002080100110004, 0x0202008004008002, 0x0020020004010100, 0x2048440040820001,
    0x0101002200408200, 0x0040802000401080, 0x4008142004410100, 0x02060820c0120200,
    0x0001001004080100, 0x020c020080040080, 0x2935610830022400, 0x0044440041009200,
    0x0280001040802101, 0x2100190040002085, 0x80c0084100102001, 0x4024081001000421,
    0x0020030a0244872, 0x0012001008414402, 0x02006104900a0804, 0x0001004081002402,
];

const fn bishop_mask_bits(sq: usize) -> u8 {
    let file = (sq % 8) as i8;
    let rank = (sq / 8) as i8;
    let mut bits: u64 = 0;
    let dirs: [(i8, i8); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
    let mut d = 0;
    while d < 4 {
        let (df, dr) = dirs[d];
        let mut f = file + df;
        let mut r = rank + dr;
        while f > 0 && f < 7 && r > 0 && r < 7 {
            bits |= 1u64 << (r as usize * 8 + f as usize);
            f += df;
            r += dr;
        }
        d += 1;
    }
    bits.count_ones() as u8
}

const fn rook_mask_bits(sq: usize) -> u8 {
    let file = (sq % 8) as i8;
    let rank = (sq / 8) as i8;
    let mut bits: u64 = 0;
    let mut r = rank + 1;
    while r < 7 {
        bits |= 1u64 << (r as usize * 8 + file as usize);
        r += 1;
    }
    r = rank - 1;
    while r > 0 {
        bits |= 1u64 << (r as usize * 8 + file as usize);
        r -= 1;
    }
    let mut f = file + 1;
    while f < 7 {
        bits |= 1u64 << (rank as usize * 8 + f as usize);
        f += 1;
    }
    f = file - 1;
    while f > 0 {
        bits |= 1u64 << (rank as usize * 8 + f as usize);
        f -= 1;
    }
    bits.count_ones() as u8
}

const fn init_bishop_shifts() -> [u8; 64] {
    let mut shifts = [0u8; 64];
    let mut sq = 0;
    while sq < 64 {
        shifts[sq] = 64 - bishop_mask_bits(sq);
        sq += 1;
    }
    shifts
}

const fn init_rook_shifts() -> [u8; 64] {
    let mut shifts = [0u8; 64];
    let mut sq = 0;
    while sq < 64 {
        shifts[sq] = 64 - rook_mask_bits(sq);
        sq += 1;
    }
    shifts
}

const BISHOP_SHIFTS: [u8; 64] = init_bishop_shifts();
const ROOK_SHIFTS: [u8; 64] = init_rook_shifts();

// --- Attack table storage ---

struct MagicTable {
    bishop_table: Vec<Bitboard>,
    bishop_offsets: [usize; 64],
    rook_table: Vec<Bitboard>,
    rook_offsets: [usize; 64],
}

static MAGIC_TABLE: OnceLock<MagicTable> = OnceLock::new();

fn init_magic_table() -> MagicTable {
    let mut bishop_offsets = [0usize; 64];
    let mut rook_offsets = [0usize; 64];

    let mut bishop_total = 0usize;
    let mut rook_total = 0usize;
    for sq in 0..64 {
        bishop_offsets[sq] = bishop_total;
        bishop_total += 1 << (64 - BISHOP_SHIFTS[sq]);
        rook_offsets[sq] = rook_total;
        rook_total += 1 << (64 - ROOK_SHIFTS[sq]);
    }

    let mut bishop_table = vec![Bitboard::new(0); bishop_total];
    let mut rook_table = vec![Bitboard::new(0); rook_total];

    for sq in 0..64 {
        let mask = BISHOP_MASKS[sq].inner();
        let magic = BISHOP_MAGICS[sq];
        let shift = BISHOP_SHIFTS[sq];

        // Enumerate all subsets of the mask (Carry-Rippler)
        let mut subset: u64 = 0;
        loop {
            let index = (subset.wrapping_mul(magic) >> shift) as usize;
            bishop_table[bishop_offsets[sq] + index] = bishop_attacks_slow(sq, subset);
            subset = subset.wrapping_sub(mask) & mask;
            if subset == 0 {
                break;
            }
        }
    }

    for sq in 0..64 {
        let mask = ROOK_MASKS[sq].inner();
        let magic = ROOK_MAGICS[sq];
        let shift = ROOK_SHIFTS[sq];

        let mut subset: u64 = 0;
        loop {
            let index = (subset.wrapping_mul(magic) >> shift) as usize;
            rook_table[rook_offsets[sq] + index] = rook_attacks_slow(sq, subset);
            subset = subset.wrapping_sub(mask) & mask;
            if subset == 0 {
                break;
            }
        }
    }

    MagicTable {
        bishop_table,
        bishop_offsets,
        rook_table,
        rook_offsets,
    }
}

// --- Public lookup functions ---

pub fn bishop_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    let table = MAGIC_TABLE.get_or_init(init_magic_table);
    let sq = square.index() as usize;
    let masked = occupancy.inner() & BISHOP_MASKS[sq].inner();
    let index = (masked.wrapping_mul(BISHOP_MAGICS[sq]) >> BISHOP_SHIFTS[sq]) as usize;
    table.bishop_table[table.bishop_offsets[sq] + index]
}

pub fn rook_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    let table = MAGIC_TABLE.get_or_init(init_magic_table);
    let sq = square.index() as usize;
    let masked = occupancy.inner() & ROOK_MASKS[sq].inner();
    let index = (masked.wrapping_mul(ROOK_MAGICS[sq]) >> ROOK_SHIFTS[sq]) as usize;
    table.rook_table[table.rook_offsets[sq] + index]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bishop_blocker_mask_center() {
        let mask = bishop_blocker_mask(Square::D4.index() as usize);
        assert_eq!(mask.pop_count(), 9);
        // Should exclude edge squares
        assert!((mask & Bitboard::new(1u64 << Square::A1.index())).is_empty());
        assert!((mask & Bitboard::new(1u64 << Square::H8.index())).is_empty());
        assert!((mask & Bitboard::new(1u64 << Square::A7.index())).is_empty());
        assert!((mask & Bitboard::new(1u64 << Square::G1.index())).is_empty());
    }

    #[test]
    fn test_bishop_blocker_mask_corner() {
        let mask = bishop_blocker_mask(Square::A1.index() as usize);
        assert_eq!(mask.pop_count(), 6);
        // Diagonal a1-h8, excluding endpoints
        assert!(!(mask & Bitboard::new(1u64 << Square::B2.index())).is_empty());
        assert!(!(mask & Bitboard::new(1u64 << Square::G7.index())).is_empty());
        // h8 should be excluded (edge)
        assert!((mask & Bitboard::new(1u64 << Square::H8.index())).is_empty());
    }

    #[test]
    fn test_rook_blocker_mask_center() {
        let mask = rook_blocker_mask(Square::D4.index() as usize);
        assert_eq!(mask.pop_count(), 10);
        // Edges should be excluded
        assert!((mask & Bitboard::new(1u64 << Square::D1.index())).is_empty());
        assert!((mask & Bitboard::new(1u64 << Square::D8.index())).is_empty());
        assert!((mask & Bitboard::new(1u64 << Square::A4.index())).is_empty());
        assert!((mask & Bitboard::new(1u64 << Square::H4.index())).is_empty());
    }

    #[test]
    fn test_rook_blocker_mask_corner() {
        let mask = rook_blocker_mask(Square::A1.index() as usize);
        assert_eq!(mask.pop_count(), 12);
    }

    #[test]
    fn test_bishop_attacks_empty_board() {
        let attacks = bishop_attacks(Square::D4, Bitboard::EMPTY);
        assert_eq!(attacks.pop_count(), 13);
        // Check some expected squares on diagonals
        assert!(!(attacks & Bitboard::new(1u64 << Square::A1.index())).is_empty());
        assert!(!(attacks & Bitboard::new(1u64 << Square::G7.index())).is_empty());
        assert!(!(attacks & Bitboard::new(1u64 << Square::H8.index())).is_empty());
        assert!(!(attacks & Bitboard::new(1u64 << Square::A7.index())).is_empty());
        assert!(!(attacks & Bitboard::new(1u64 << Square::G1.index())).is_empty());
    }

    #[test]
    fn test_bishop_attacks_with_blockers() {
        let blockers = Bitboard::new((1u64 << Square::B2.index()) | (1u64 << Square::F6.index()));
        let attacks = bishop_attacks(Square::D4, blockers);
        // Should include blockers
        assert!(!(attacks & Bitboard::new(1u64 << Square::B2.index())).is_empty());
        assert!(!(attacks & Bitboard::new(1u64 << Square::F6.index())).is_empty());
        // Should NOT include squares beyond blockers
        assert!((attacks & Bitboard::new(1u64 << Square::A1.index())).is_empty());
        assert!((attacks & Bitboard::new(1u64 << Square::G7.index())).is_empty());
    }

    #[test]
    fn test_rook_attacks_empty_board() {
        let attacks = rook_attacks(Square::D4, Bitboard::EMPTY);
        assert_eq!(attacks.pop_count(), 14);
        assert!(!(attacks & Bitboard::new(1u64 << Square::D1.index())).is_empty());
        assert!(!(attacks & Bitboard::new(1u64 << Square::D8.index())).is_empty());
        assert!(!(attacks & Bitboard::new(1u64 << Square::A4.index())).is_empty());
        assert!(!(attacks & Bitboard::new(1u64 << Square::H4.index())).is_empty());
    }

    #[test]
    fn test_rook_attacks_with_blockers() {
        let blockers = Bitboard::new((1u64 << Square::D2.index()) | (1u64 << Square::F4.index()));
        let attacks = rook_attacks(Square::D4, blockers);
        // Should include blockers
        assert!(!(attacks & Bitboard::new(1u64 << Square::D2.index())).is_empty());
        assert!(!(attacks & Bitboard::new(1u64 << Square::F4.index())).is_empty());
        // Should NOT include squares beyond blockers
        assert!((attacks & Bitboard::new(1u64 << Square::D1.index())).is_empty());
        assert!((attacks & Bitboard::new(1u64 << Square::G4.index())).is_empty());
    }

    #[test]
    fn test_bishop_attacks_all_squares() {
        for sq_idx in 0..64 {
            let square = Square::new(sq_idx as u8).expect("valid square");
            let fast = bishop_attacks(square, Bitboard::EMPTY);
            let slow = bishop_attacks_slow(sq_idx, 0);
            assert_eq!(fast, slow, "bishop mismatch at square index {sq_idx}");
        }
    }

    #[test]
    fn test_rook_attacks_all_squares() {
        for sq_idx in 0..64 {
            let square = Square::new(sq_idx as u8).expect("valid square");
            let fast = rook_attacks(square, Bitboard::EMPTY);
            let slow = rook_attacks_slow(sq_idx, 0);
            assert_eq!(fast, slow, "rook mismatch at square index {sq_idx}");
        }
    }
}
