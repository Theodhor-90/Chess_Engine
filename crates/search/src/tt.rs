use chess_types::Move;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BoundType {
    Exact = 0,
    LowerBound = 1,
    UpperBound = 2,
}

impl TryFrom<u8> for BoundType {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(BoundType::Exact),
            1 => Ok(BoundType::LowerBound),
            2 => Ok(BoundType::UpperBound),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TtEntry {
    key: u16,
    depth: u8,
    score: i32,
    bound: BoundType,
    best_move: Option<Move>,
    age: u8,
}

impl TtEntry {
    pub fn new(
        key: u16,
        depth: u8,
        score: i32,
        bound: BoundType,
        best_move: Option<Move>,
        age: u8,
    ) -> TtEntry {
        TtEntry {
            key,
            depth,
            score,
            bound,
            best_move,
            age,
        }
    }

    pub fn key(&self) -> u16 {
        self.key
    }

    pub fn depth(&self) -> u8 {
        self.depth
    }

    pub fn score(&self) -> i32 {
        self.score
    }

    pub fn bound(&self) -> BoundType {
        self.bound
    }

    pub fn best_move(&self) -> Option<Move> {
        self.best_move
    }

    pub fn age(&self) -> u8 {
        self.age
    }
}

impl Default for TtEntry {
    fn default() -> Self {
        TtEntry {
            key: 0,
            depth: 0,
            score: 0,
            bound: BoundType::Exact,
            best_move: None,
            age: 0,
        }
    }
}

pub fn verification_key(hash: u64) -> u16 {
    (hash >> 48) as u16
}

const MAX_PLY: i32 = 128;

pub fn score_to_tt(score: i32, ply: u8) -> i32 {
    let ply = ply as i32;
    if score > crate::MATE_SCORE - MAX_PLY {
        score + ply
    } else if score < -(crate::MATE_SCORE - MAX_PLY) {
        score - ply
    } else {
        score
    }
}

pub fn score_from_tt(score: i32, ply: u8) -> i32 {
    let ply = ply as i32;
    if score > crate::MATE_SCORE - MAX_PLY {
        score - ply
    } else if score < -(crate::MATE_SCORE - MAX_PLY) {
        score + ply
    } else {
        score
    }
}

pub struct TranspositionTable {
    entries: Vec<TtEntry>,
    mask: usize,
    generation: u8,
}

impl TranspositionTable {
    pub fn new(mb: usize) -> Self {
        let bytes = mb * 1024 * 1024;
        let entry_size = std::mem::size_of::<TtEntry>();
        let max_entries = bytes / entry_size;
        let num_entries = if max_entries == 0 {
            1
        } else {
            1 << (usize::BITS - 1 - max_entries.leading_zeros())
        };
        TranspositionTable {
            entries: vec![TtEntry::default(); num_entries],
            mask: num_entries - 1,
            generation: 0,
        }
    }

    pub fn probe(&self, hash: u64) -> Option<TtEntry> {
        let index = (hash as usize) & self.mask;
        let entry = self.entries[index];
        if entry.key() == verification_key(hash) {
            Some(entry)
        } else {
            None
        }
    }

    pub fn store(&mut self, hash: u64, entry: TtEntry) {
        let index = (hash as usize) & self.mask;
        let existing = self.entries[index];
        if existing.age() != self.generation || entry.depth() >= existing.depth() {
            self.entries[index] = entry;
        }
    }

    pub fn clear(&mut self) {
        self.entries.fill(TtEntry::default());
        self.generation = 0;
    }

    pub fn resize(&mut self, mb: usize) {
        let bytes = mb * 1024 * 1024;
        let entry_size = std::mem::size_of::<TtEntry>();
        let max_entries = bytes / entry_size;
        let num_entries = if max_entries == 0 {
            1
        } else {
            1 << (usize::BITS - 1 - max_entries.leading_zeros())
        };
        self.entries = vec![TtEntry::default(); num_entries];
        self.mask = num_entries - 1;
        self.generation = 0;
    }

    pub fn new_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn generation(&self) -> u8 {
        self.generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_types::{MoveFlag, Square};

    fn quiet_move(from: u8, to: u8) -> Move {
        Move::new(
            Square::new(from).unwrap(),
            Square::new(to).unwrap(),
            MoveFlag::QUIET,
        )
    }

    #[test]
    fn round_trip_all_fields() {
        let mv = quiet_move(12, 28);
        let entry = TtEntry::new(0xABCD, 12, 150, BoundType::Exact, Some(mv), 3);
        assert_eq!(entry.key(), 0xABCD);
        assert_eq!(entry.depth(), 12);
        assert_eq!(entry.score(), 150);
        assert_eq!(entry.bound(), BoundType::Exact);
        assert_eq!(entry.best_move(), Some(mv));
        assert_eq!(entry.age(), 3);
    }

    #[test]
    fn round_trip_no_move() {
        let entry = TtEntry::new(0x1234, 5, -100, BoundType::LowerBound, None, 1);
        assert_eq!(entry.best_move(), None);
    }

    #[test]
    fn bound_type_exact() {
        let entry = TtEntry::new(0, 0, 0, BoundType::Exact, None, 0);
        assert_eq!(entry.bound(), BoundType::Exact);
    }

    #[test]
    fn bound_type_lower_bound() {
        let entry = TtEntry::new(0, 0, 0, BoundType::LowerBound, None, 0);
        assert_eq!(entry.bound(), BoundType::LowerBound);
    }

    #[test]
    fn bound_type_upper_bound() {
        let entry = TtEntry::new(0, 0, 0, BoundType::UpperBound, None, 0);
        assert_eq!(entry.bound(), BoundType::UpperBound);
    }

    #[test]
    fn bound_type_try_from_valid() {
        assert_eq!(BoundType::try_from(0u8), Ok(BoundType::Exact));
        assert_eq!(BoundType::try_from(1u8), Ok(BoundType::LowerBound));
        assert_eq!(BoundType::try_from(2u8), Ok(BoundType::UpperBound));
    }

    #[test]
    fn bound_type_try_from_invalid() {
        assert_eq!(BoundType::try_from(3u8), Err(()));
        assert_eq!(BoundType::try_from(255u8), Err(()));
    }

    #[test]
    fn default_entry() {
        let entry = TtEntry::default();
        assert_eq!(entry.key(), 0);
        assert_eq!(entry.depth(), 0);
        assert_eq!(entry.score(), 0);
        assert_eq!(entry.bound(), BoundType::Exact);
        assert_eq!(entry.best_move(), None);
        assert_eq!(entry.age(), 0);
    }

    #[test]
    fn verification_key_extracts_upper_bits() {
        assert_eq!(verification_key(0xABCD_0000_0000_0000), 0xABCD);
        assert_eq!(verification_key(0xFFFF_1234_5678_9ABC), 0xFFFF);
        assert_eq!(verification_key(0x0000_FFFF_FFFF_FFFF), 0x0000);
        assert_eq!(verification_key(0x1234_5678_9ABC_DEF0), 0x1234);
    }

    #[test]
    fn store_and_probe_round_trip() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xABCD_0000_0000_0001;
        let mv = quiet_move(12, 28);
        let entry = TtEntry::new(
            verification_key(hash),
            10,
            250,
            BoundType::Exact,
            Some(mv),
            tt.generation(),
        );
        tt.store(hash, entry);
        let probed = tt.probe(hash);
        assert_eq!(probed, Some(entry));
    }

    #[test]
    fn probe_returns_none_for_missing() {
        let mut tt = TranspositionTable::new(1);
        let hash_a: u64 = 0xABCD_0000_0000_0001;
        let entry = TtEntry::new(
            verification_key(hash_a),
            5,
            100,
            BoundType::LowerBound,
            None,
            tt.generation(),
        );
        tt.store(hash_a, entry);
        let hash_b: u64 = 0x1234_0000_0000_0002;
        assert_eq!(tt.probe(hash_b), None);
    }

    #[test]
    fn shallower_does_not_replace_deeper_same_age() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xABCD_0000_0000_0001;
        let entry_a = TtEntry::new(
            verification_key(hash),
            10,
            200,
            BoundType::Exact,
            None,
            tt.generation(),
        );
        tt.store(hash, entry_a);
        let entry_b = TtEntry::new(
            verification_key(hash),
            5,
            100,
            BoundType::Exact,
            None,
            tt.generation(),
        );
        tt.store(hash, entry_b);
        let probed = tt.probe(hash).unwrap();
        assert_eq!(probed.depth(), 10);
        assert_eq!(probed.score(), 200);
    }

    #[test]
    fn replacement_stale_entry_replaced_by_shallower() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xABCD_0000_0000_0001;
        let entry_a = TtEntry::new(
            verification_key(hash),
            10,
            200,
            BoundType::Exact,
            None,
            tt.generation(),
        );
        tt.store(hash, entry_a);
        tt.new_generation();
        let entry_b = TtEntry::new(
            verification_key(hash),
            3,
            50,
            BoundType::LowerBound,
            None,
            tt.generation(),
        );
        tt.store(hash, entry_b);
        let probed = tt.probe(hash).unwrap();
        assert_eq!(probed.depth(), 3);
        assert_eq!(probed.score(), 50);
    }

    #[test]
    fn replacement_equal_depth_same_age_replaces() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xABCD_0000_0000_0001;
        let mv_a = quiet_move(0, 8);
        let entry_a = TtEntry::new(
            verification_key(hash),
            5,
            100,
            BoundType::Exact,
            Some(mv_a),
            tt.generation(),
        );
        tt.store(hash, entry_a);
        let mv_b = quiet_move(1, 9);
        let entry_b = TtEntry::new(
            verification_key(hash),
            5,
            -50,
            BoundType::UpperBound,
            Some(mv_b),
            tt.generation(),
        );
        tt.store(hash, entry_b);
        let probed = tt.probe(hash).unwrap();
        assert_eq!(probed.score(), -50);
        assert_eq!(probed.bound(), BoundType::UpperBound);
        assert_eq!(probed.best_move(), Some(mv_b));
    }

    #[test]
    fn power_of_two_sizing() {
        let tt = TranspositionTable::new(16);
        let count = tt.entries.len();
        assert!(count > 0);
        assert_eq!(count & (count - 1), 0);
        let total_bytes = count * std::mem::size_of::<TtEntry>();
        assert!(total_bytes <= 16 * 1024 * 1024);
    }

    #[test]
    fn clear_makes_all_probes_none() {
        let mut tt = TranspositionTable::new(1);
        let hashes: [u64; 3] = [
            0xABCD_0000_0000_0001,
            0x1234_0000_0000_0002,
            0x5678_0000_0000_0003,
        ];
        for &h in &hashes {
            let entry = TtEntry::new(
                verification_key(h),
                5,
                100,
                BoundType::Exact,
                None,
                tt.generation(),
            );
            tt.store(h, entry);
        }
        tt.clear();
        for &h in &hashes {
            assert_eq!(tt.probe(h), None);
        }
    }

    #[test]
    fn new_generation_increments_age() {
        let mut tt = TranspositionTable::new(1);
        assert_eq!(tt.generation(), 0);
        tt.new_generation();
        assert_eq!(tt.generation(), 1);
        let hash: u64 = 0xABCD_0000_0000_0001;
        let entry = TtEntry::new(
            verification_key(hash),
            5,
            100,
            BoundType::Exact,
            None,
            tt.generation(),
        );
        tt.store(hash, entry);
        let probed = tt.probe(hash).unwrap();
        assert_eq!(probed.age(), 1);
    }

    #[test]
    fn new_with_zero_mb_creates_minimum_table() {
        let mut tt = TranspositionTable::new(0);
        assert_eq!(tt.entries.len(), 1);
        let hash: u64 = 0xABCD_0000_0000_0000;
        let entry = TtEntry::new(
            verification_key(hash),
            3,
            42,
            BoundType::LowerBound,
            None,
            tt.generation(),
        );
        tt.store(hash, entry);
        let probed = tt.probe(hash);
        assert_eq!(probed, Some(entry));
    }

    #[test]
    fn score_to_tt_positive_mate() {
        assert_eq!(score_to_tt(crate::MATE_SCORE - 5, 3), crate::MATE_SCORE - 2);
    }

    #[test]
    fn score_to_tt_negative_mate() {
        assert_eq!(
            score_to_tt(-(crate::MATE_SCORE - 5), 3),
            -(crate::MATE_SCORE - 2)
        );
    }

    #[test]
    fn score_to_tt_non_mate_unchanged() {
        assert_eq!(score_to_tt(150, 3), 150);
    }

    #[test]
    fn score_from_tt_positive_mate() {
        assert_eq!(
            score_from_tt(crate::MATE_SCORE - 2, 3),
            crate::MATE_SCORE - 5
        );
    }

    #[test]
    fn score_from_tt_negative_mate() {
        assert_eq!(
            score_from_tt(-(crate::MATE_SCORE - 2), 3),
            -(crate::MATE_SCORE - 5)
        );
    }

    #[test]
    fn score_from_tt_non_mate_unchanged() {
        assert_eq!(score_from_tt(150, 3), 150);
    }

    #[test]
    fn score_round_trip() {
        let cases = [
            (crate::MATE_SCORE - 5, 3u8),
            (-(crate::MATE_SCORE - 5), 3),
            (crate::MATE_SCORE - 1, 10),
            (-(crate::MATE_SCORE - 1), 10),
            (150, 5),
            (-200, 0),
            (0, 7),
        ];
        for (score, ply) in cases {
            assert_eq!(
                score_from_tt(score_to_tt(score, ply), ply),
                score,
                "round trip failed for score={score}, ply={ply}"
            );
        }
    }
}
