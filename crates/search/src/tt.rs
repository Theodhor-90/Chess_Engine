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
}
